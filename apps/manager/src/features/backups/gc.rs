//! Daily mark-and-sweep GC per backup_target.

use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::{BackupTargetRepository, BackupTargetRow};
use anyhow::{Context, Result};
use aws_sdk_s3::{
    config::{Builder as S3ConfBuilder, Region},
    types::{Delete, ObjectIdentifier},
    Client,
};
use chrono::{Timelike, Utc};
use nexus_backup::{decrypt_manifest, ChunkKey, Manifest};
use sqlx::PgPool;
use std::collections::HashSet;

pub async fn gc_loop(pool: PgPool) {
    loop {
        let now = Utc::now();
        let next_check = (60 - now.minute()) as u64 * 60 - now.second() as u64;
        tokio::time::sleep(std::time::Duration::from_secs(next_check.max(60))).await;

        let now = Utc::now();
        let repo = BackupTargetRepository::new(pool.clone());
        match repo.list_active().await {
            Ok(targets) => {
                for t in targets {
                    if (t.gc_hour as u32) == now.hour() {
                        if let Err(e) = run_gc(&pool, &t).await {
                            tracing::error!(target=%t.name, "GC run failed: {e:#}");
                        }
                    }
                }
            }
            Err(e) => tracing::error!("gc_loop list_active: {e}"),
        }
    }
}

pub async fn run_gc(pool: &PgPool, target: &BackupTargetRow) -> Result<()> {
    let run_id: uuid::Uuid = sqlx::query_scalar(
        r#"INSERT INTO backup_gc_run (target_id, status) VALUES ($1, 'running') RETURNING id"#,
    )
    .bind(target.id)
    .fetch_one(pool)
    .await?;

    let result = sweep(target).await;
    match result {
        Ok((bytes, chunks)) => {
            sqlx::query(
                r#"UPDATE backup_gc_run SET status='completed', completed_at=now(),
                   bytes_freed=$1, chunks_deleted=$2 WHERE id = $3"#,
            )
            .bind(bytes as i64)
            .bind(chunks as i64)
            .bind(run_id)
            .execute(pool)
            .await?;
            tracing::info!(target=%target.name, bytes_freed=bytes, chunks_deleted=chunks, "GC complete");
            Ok(())
        }
        Err(e) => {
            sqlx::query(
                r#"UPDATE backup_gc_run SET status='failed', completed_at=now(), error_message=$1 WHERE id=$2"#,
            )
            .bind(e.to_string())
            .bind(run_id)
            .execute(pool)
            .await
            .ok();
            Err(e)
        }
    }
}

async fn sweep(target: &BackupTargetRow) -> Result<(u64, u64)> {
    let secret = envelope::unwrap_to_string(&target.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target.encrypted_target_key)?;

    let creds = aws_credential_types::Credentials::new(
        &target.access_key_id,
        &secret,
        None,
        None,
        "nqrust-gc",
    );
    let region = Region::new(target.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let s3_cfg = S3ConfBuilder::new()
        .behavior_version_latest()
        .endpoint_url(&target.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true)
        .build();
    let client = Client::from_conf(s3_cfg);

    // 1. Mark: walk manifests, collect referenced chunk ids.
    let prefix_manifests = if target.prefix.is_empty() {
        "manifests/".to_string()
    } else {
        format!("{}/manifests/", target.prefix.trim_end_matches('/'))
    };
    let mut referenced: HashSet<[u8; 32]> = HashSet::new();
    let mut continuation: Option<String> = None;
    loop {
        let mut req = client
            .list_objects_v2()
            .bucket(&target.bucket)
            .prefix(&prefix_manifests);
        if let Some(c) = continuation.as_deref() {
            req = req.continuation_token(c);
        }
        let resp = req.send().await.context("LIST manifests")?;
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            let blob = client
                .get_object()
                .bucket(&target.bucket)
                .key(k)
                .send()
                .await
                .with_context(|| format!("GET {k}"))?
                .body
                .collect()
                .await
                .context("body collect")?
                .into_bytes()
                .to_vec();
            let chunk_key = ChunkKey::from_bytes(target_key);
            let compressed = decrypt_manifest(&chunk_key, &blob).context("decrypt manifest")?;
            let m: Manifest =
                Manifest::deserialize_compressed(&compressed).context("deserialize manifest")?;
            for c in m.chunks {
                referenced.insert(c.chunk_id);
            }
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else {
            break;
        }
    }

    // 2. Sweep: walk chunks, delete unreferenced ones older than 24h.
    let prefix_chunks = if target.prefix.is_empty() {
        "chunks/".to_string()
    } else {
        format!("{}/chunks/", target.prefix.trim_end_matches('/'))
    };
    let cutoff = aws_smithy_types::DateTime::from_secs(
        (Utc::now() - chrono::Duration::hours(24)).timestamp(),
    );
    let mut bytes_freed: u64 = 0;
    let mut chunks_deleted: u64 = 0;

    let mut continuation: Option<String> = None;
    loop {
        let mut req = client
            .list_objects_v2()
            .bucket(&target.bucket)
            .prefix(&prefix_chunks);
        if let Some(c) = continuation.as_deref() {
            req = req.continuation_token(c);
        }
        let resp = req.send().await.context("LIST chunks")?;
        let mut to_delete: Vec<ObjectIdentifier> = Vec::new();
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            let too_recent = obj.last_modified().map(|lm| lm > &cutoff).unwrap_or(true);
            if too_recent {
                continue;
            }
            let parts: Vec<&str> = k.rsplit('/').collect();
            if parts.is_empty() {
                continue;
            }
            let id_hex = parts[0];
            if id_hex.len() != 64 {
                continue;
            }
            let mut chunk_id = [0u8; 32];
            if hex::decode_to_slice(id_hex, &mut chunk_id).is_err() {
                continue;
            }
            if referenced.contains(&chunk_id) {
                continue;
            }
            bytes_freed += obj.size().unwrap_or(0) as u64;
            chunks_deleted += 1;
            if let Ok(oid) = ObjectIdentifier::builder().key(k).build() {
                to_delete.push(oid);
            }
            if to_delete.len() == 1000 {
                if let Ok(del) = Delete::builder()
                    .set_objects(Some(std::mem::take(&mut to_delete)))
                    .build()
                {
                    client
                        .delete_objects()
                        .bucket(&target.bucket)
                        .delete(del)
                        .send()
                        .await
                        .context("DELETE chunks batch")?;
                }
            }
        }
        if !to_delete.is_empty() {
            if let Ok(del) = Delete::builder().set_objects(Some(to_delete)).build() {
                client
                    .delete_objects()
                    .bucket(&target.bucket)
                    .delete(del)
                    .send()
                    .await
                    .context("DELETE chunks final")?;
            }
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else {
            break;
        }
    }

    Ok((bytes_freed, chunks_deleted))
}
