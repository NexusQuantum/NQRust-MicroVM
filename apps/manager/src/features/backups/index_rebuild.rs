//! Reconstruct the `backup` table from S3 manifests.
//! Run via: `manager backup index-rebuild --target <id>`.

use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::BackupTargetRepository;
use anyhow::{Context, Result};
use aws_sdk_s3::{config::{Builder, Region}, Client};
use chrono::TimeZone;
use nexus_backup::{decrypt_manifest, ChunkKey, Manifest};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn run(pool: PgPool, target_id: Uuid) -> Result<()> {
    let repo = BackupTargetRepository::new(pool.clone());
    let target = repo.get(target_id).await?
        .ok_or_else(|| anyhow::anyhow!("target {target_id} not found"))?;

    let secret = envelope::unwrap_to_string(&target.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target.encrypted_target_key)?;

    let creds = aws_credential_types::Credentials::new(
        &target.access_key_id, &secret, None, None, "nqrust-rebuild");
    let region = Region::new(target.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let s3_cfg = Builder::new()
        .behavior_version_latest()
        .endpoint_url(&target.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true)
        .build();
    let client = Client::from_conf(s3_cfg);

    let prefix = if target.prefix.is_empty() {
        "manifests/".to_string()
    } else {
        format!("{}/manifests/", target.prefix.trim_end_matches('/'))
    };

    let mut reconstructed = 0usize;
    let mut skipped = 0usize;
    let mut continuation: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(&target.bucket).prefix(&prefix);
        if let Some(c) = continuation.as_deref() { req = req.continuation_token(c); }
        let resp = req.send().await.context("LIST manifests")?;
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            let blob = client.get_object().bucket(&target.bucket).key(k).send().await
                .with_context(|| format!("GET {k}"))?
                .body.collect().await?
                .into_bytes().to_vec();
            let key = ChunkKey::from_bytes(target_key);
            let compressed = decrypt_manifest(&key, &blob)?;
            let m = Manifest::deserialize_compressed(&compressed)?;
            let existed: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT id FROM backup WHERE id = $1"#,
            )
            .bind(m.backup_id)
            .fetch_optional(&pool)
            .await?;
            if existed.is_some() { skipped += 1; continue; }
            let total_size: i64 = m.chunks.iter().map(|c| c.ciphertext_length as i64).sum();
            let chunk_count = m.chunks.len() as i64;
            let created_at = chrono::Utc.timestamp_opt(m.created_at_unix_seconds, 0).single()
                .unwrap_or_else(chrono::Utc::now);
            sqlx::query(
                r#"INSERT INTO backup
                   (id, source_volume_id, source_snapshot_id, target_id,
                    manifest_object_key, size_bytes, unique_bytes, chunk_count,
                    status, created_at, completed_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'completed', $9, $9, now())"#,
            )
            .bind(m.backup_id)
            .bind(m.source_volume_id)
            .bind(m.source_snapshot_id)
            .bind(target_id)
            .bind(k)
            .bind(total_size)
            .bind(0i64)
            .bind(chunk_count)
            .bind(created_at)
            .execute(&pool)
            .await
            .ok();
            reconstructed += 1;
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else { break; }
    }
    println!("index-rebuild: reconstructed {reconstructed}, skipped (already in DB) {skipped}");
    Ok(())
}
