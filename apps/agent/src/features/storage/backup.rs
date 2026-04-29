//! The chunker pipeline: read snapshot bytes → FastCDC → encrypt → HEAD-or-PUT.

use crate::features::storage::registry::HostBackendRegistry;
use crate::features::storage::s3::{self, BackupTargetConfig};
use anyhow::{Context, Result};
use chrono::Utc;
use nexus_backup::{
    chunk_object_key, decrypt_chunk, decrypt_manifest, encrypt_chunk, encrypt_manifest,
    manifest_object_key, ChunkKey, ChunkRef, Chunker, ChunkerParams, Manifest, MANIFEST_VERSION,
};
use nexus_storage::{AttachedPath, VolumeHandle, VolumeSnapshotHandle};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use uuid::Uuid;

pub struct BackupParams {
    pub backup_id: Uuid,
    pub snapshot: VolumeSnapshotHandle,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
    pub chunker_params: ChunkerParams,
}

pub struct BackupOutcome {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

pub async fn run_backup(
    registry: Arc<HostBackendRegistry>,
    params: BackupParams,
) -> Result<BackupOutcome> {
    let start = Instant::now();
    let backend = registry
        .get(params.snapshot.backend_kind)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no host backend for kind {:?}",
                params.snapshot.backend_kind
            )
        })?
        .clone();

    let mut reader = backend
        .read_snapshot(&params.snapshot)
        .await
        .context("read_snapshot")?;
    let mut chunker = Chunker::new(&mut reader, params.chunker_params);
    let s3 = s3::make_client(&params.target);
    let key = ChunkKey::from_bytes(params.encryption_key);

    let mut chunks: Vec<ChunkRef> = Vec::new();
    let mut bytes_written: u64 = 0;
    let mut bytes_unique: u64 = 0;
    let mut total_plaintext: u64 = 0;

    while let Some(chunk) = chunker.next_chunk().await? {
        let plaintext_hash: [u8; 32] = *blake3::hash(&chunk.plaintext_bytes).as_bytes();
        let ciphertext = encrypt_chunk(&key, &chunk.plaintext_bytes).context("encrypt_chunk")?;
        let chunk_id: [u8; 32] = *blake3::hash(&ciphertext).as_bytes();
        let object_key = chunk_object_key(&params.target.prefix, &chunk_id);

        let exists = {
            let mut attempt = 0u32;
            loop {
                match s3::head_object(&s3, &params.target.bucket, &object_key).await {
                    Ok(v) => break v,
                    Err(e) if attempt < 4 => {
                        let backoff_ms = 200u64 * (1u64 << attempt);
                        tracing::warn!(
                            "HEAD chunk attempt {} failed: {e}; retrying in {backoff_ms}ms",
                            attempt + 1
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("HEAD chunk failed after 5 attempts: {e}"))
                    }
                }
            }
        };
        bytes_written += ciphertext.len() as u64;
        if !exists {
            let cipher_len = ciphertext.len() as u64;
            let mut attempt = 0u32;
            loop {
                match s3::put_object(&s3, &params.target.bucket, &object_key, ciphertext.clone())
                    .await
                {
                    Ok(()) => break,
                    Err(e) if attempt < 4 => {
                        let backoff_ms = 200u64 * (1u64 << attempt);
                        tracing::warn!(
                            "PUT chunk attempt {} failed: {e}; retrying in {backoff_ms}ms",
                            attempt + 1
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("PUT chunk failed after 5 attempts: {e}"))
                    }
                }
            }
            bytes_unique += cipher_len;
        }

        chunks.push(ChunkRef {
            plaintext_offset: chunk.plaintext_offset,
            plaintext_length: chunk.plaintext_length,
            plaintext_hash,
            chunk_id,
            ciphertext_length: ciphertext.len() as u32,
        });
        total_plaintext += chunk.plaintext_length as u64;
    }

    let manifest = Manifest {
        version: MANIFEST_VERSION,
        backup_id: params.backup_id,
        source_volume_id: params.snapshot.source_volume_id,
        source_snapshot_id: Some(params.snapshot.snapshot_id),
        total_plaintext_size: total_plaintext,
        created_at_unix_seconds: Utc::now().timestamp(),
        chunks: chunks.clone(),
    };
    let manifest_compressed = manifest
        .serialize_compressed()
        .context("manifest serialize")?;
    let manifest_blob = encrypt_manifest(&key, &manifest_compressed).context("encrypt manifest")?;
    let mkey = manifest_object_key(&params.target.prefix, &params.backup_id);
    {
        let mut attempt = 0u32;
        loop {
            match s3::put_object(&s3, &params.target.bucket, &mkey, manifest_blob.clone()).await {
                Ok(()) => break,
                Err(e) if attempt < 4 => {
                    let backoff_ms = 200u64 * (1u64 << attempt);
                    tracing::warn!(
                        "PUT manifest attempt {} failed: {e}; retrying in {backoff_ms}ms",
                        attempt + 1
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    attempt += 1;
                }
                Err(e) => return Err(anyhow::anyhow!("PUT manifest failed after 5 attempts: {e}")),
            }
        }
    }

    Ok(BackupOutcome {
        manifest_object_key: mkey,
        chunk_count: chunks.len() as u64,
        bytes_written,
        bytes_unique,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

pub struct RestoreParams {
    pub target_volume: VolumeHandle,
    pub target_attached: AttachedPath,
    pub manifest_object_key: String,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
}

pub struct RestoreOutcome {
    pub bytes_written: u64,
    pub duration_ms: u64,
}

pub async fn run_restore(params: RestoreParams) -> Result<RestoreOutcome> {
    let start = Instant::now();
    let s3 = s3::make_client(&params.target);
    let key = ChunkKey::from_bytes(params.encryption_key);

    let blob = {
        let mut attempt = 0u32;
        loop {
            match s3::get_object(&s3, &params.target.bucket, &params.manifest_object_key).await {
                Ok(v) => break v,
                Err(e) if attempt < 4 => {
                    let backoff_ms = 200u64 * (1u64 << attempt);
                    tracing::warn!(
                        "GET manifest attempt {} failed: {e}; retrying in {backoff_ms}ms",
                        attempt + 1
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    attempt += 1;
                }
                Err(e) => return Err(anyhow::anyhow!("GET manifest failed after 5 attempts: {e}")),
            }
        }
    };
    let compressed = decrypt_manifest(&key, &blob).context("decrypt manifest")?;
    let manifest = Manifest::deserialize_compressed(&compressed).context("deserialize manifest")?;

    let mut dst = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(params.target_attached.path())
        .await?;

    let mut bytes_written: u64 = 0;
    for chunk_ref in &manifest.chunks {
        let object_key = chunk_object_key(&params.target.prefix, &chunk_ref.chunk_id);
        let ciphertext = {
            let mut attempt = 0u32;
            loop {
                match s3::get_object(&s3, &params.target.bucket, &object_key).await {
                    Ok(v) => break v,
                    Err(e) if attempt < 4 => {
                        let backoff_ms = 200u64 * (1u64 << attempt);
                        tracing::warn!(
                            "GET chunk {} attempt {} failed: {e}; retrying in {backoff_ms}ms",
                            hex::encode(chunk_ref.chunk_id),
                            attempt + 1
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "GET chunk {} failed after 5 attempts: {e}",
                            hex::encode(chunk_ref.chunk_id)
                        ))
                    }
                }
            }
        };
        let plaintext =
            decrypt_chunk(&key, &ciphertext, &chunk_ref.plaintext_hash).context("decrypt chunk")?;
        dst.seek(std::io::SeekFrom::Start(chunk_ref.plaintext_offset))
            .await?;
        dst.write_all(&plaintext).await?;
        bytes_written += plaintext.len() as u64;
    }
    dst.flush().await?;

    let _ = params.target_volume; // suppress unused warning; kept for future logging

    Ok(RestoreOutcome {
        bytes_written,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
