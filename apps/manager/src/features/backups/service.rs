use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::BackupTargetRepository;
use crate::features::backups::repo::BackupRepository;
use crate::features::backups::types::{BackupReq, BackupTargetConfig, ChunkerParams, RestoreReq};
use crate::features::storage::agent_rpc;
use crate::features::volumes::repo::VolumeRepository;
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendInstanceId, VolumeHandle};
use uuid::Uuid;

pub async fn create_backup(st: &AppState, volume_id: Uuid, target_id: Uuid) -> Result<Uuid> {
    let backup_repo = BackupRepository::new(st.db.clone());
    let target_repo = BackupTargetRepository::new(st.db.clone());
    let target_row = target_repo
        .get(target_id)
        .await?
        .ok_or_else(|| anyhow!("target {target_id} not found"))?;

    // Resolve volume → backend → host.
    let vol: (Uuid, String, i64, Option<Uuid>, Uuid) = sqlx::query_as(
        r#"SELECT v.id, v.path, v.size_bytes, v.host_id, v.backend_id FROM volume v WHERE v.id = $1"#,
    )
    .bind(volume_id)
    .fetch_one(&st.db)
    .await
    .context("looking up volume")?;
    let (vol_id, locator, size_bytes, host_id_opt, backend_id) =
        (vol.0, vol.1, vol.2, vol.3, vol.4);

    let backend = st
        .registry
        .get(backend_id)
        .ok_or_else(|| anyhow!("registry has no backend with id {backend_id}"))?
        .clone();
    let host_id = host_id_opt.ok_or_else(|| {
        anyhow!("volume has no home host (network-attached not supported by backup service yet)")
    })?;
    let host = st.hosts.get(host_id).await.context("getting host row")?;

    let volume_handle = VolumeHandle {
        volume_id: vol_id,
        backend_id: BackendInstanceId(backend_id),
        backend_kind: backend.kind(),
        locator,
        size_bytes: size_bytes as u64,
    };
    let snap_name = format!("backup-{}", Uuid::new_v4());
    let snap = backend
        .snapshot(&volume_handle, &snap_name)
        .await
        .context("control-plane snapshot")?;

    let backup_row = backup_repo
        .insert_running(volume_id, snap.snapshot_id, target_id)
        .await?;

    let secret_access_key = envelope::unwrap_to_string(&target_row.encrypted_secret_access_key)
        .context("decrypt secret_access_key")?;
    let target_key = envelope::unwrap_to_array::<32>(&target_row.encrypted_target_key)
        .context("decrypt target_key")?;

    let target_config = BackupTargetConfig {
        endpoint: target_row.endpoint.clone(),
        region: target_row.region.clone(),
        bucket: target_row.bucket.clone(),
        prefix: target_row.prefix.clone(),
        access_key_id: target_row.access_key_id.clone(),
        secret_access_key,
    };

    let req = BackupReq {
        backup_id: backup_row.id,
        snapshot: snap.clone(),
        backend_kind: backend.kind(),
        target: target_config,
        encryption_key: target_key,
        chunker_params: ChunkerParams::default(),
    };

    match agent_rpc::agent_backup(&host.addr, req).await {
        Ok(resp) => {
            backup_repo
                .mark_completed(
                    backup_row.id,
                    &resp.manifest_object_key,
                    resp.bytes_written as i64,
                    resp.bytes_unique as i64,
                    resp.chunk_count as i64,
                )
                .await?;
            let _ = backend.delete_snapshot(snap).await;
            let _ = enforce_retention(st, volume_id, &backup_repo).await;
            Ok(backup_row.id)
        }
        Err(e) => {
            backup_repo
                .mark_failed(backup_row.id, &format!("{e:#}"))
                .await?;
            let _ = backend.delete_snapshot(snap).await;
            Err(e)
        }
    }
}

async fn enforce_retention(
    st: &AppState,
    volume_id: Uuid,
    backup_repo: &BackupRepository,
) -> Result<()> {
    let retain: Option<i32> =
        sqlx::query_scalar(r#"SELECT backup_retain_count FROM volume WHERE id = $1"#)
            .bind(volume_id)
            .fetch_one(&st.db)
            .await?;
    let Some(retain) = retain else {
        return Ok(());
    };
    if retain <= 0 {
        return Ok(());
    }

    let mut completed = backup_repo.list_completed_oldest_first(volume_id).await?;
    while completed.len() as i32 > retain {
        let oldest = completed.remove(0);
        // Best-effort delete the manifest from S3.
        if let Some(mkey) = oldest.manifest_object_key.as_deref() {
            if let Some(t) = BackupTargetRepository::new(st.db.clone())
                .get(oldest.target_id)
                .await
                .ok()
                .flatten()
            {
                if let Ok(secret) = envelope::unwrap_to_string(&t.encrypted_secret_access_key) {
                    let creds = aws_credential_types::Credentials::new(
                        &t.access_key_id,
                        &secret,
                        None,
                        None,
                        "nqrust-mgr-prune",
                    );
                    let region = aws_sdk_s3::config::Region::new(
                        t.region.clone().unwrap_or_else(|| "us-east-1".into()),
                    );
                    let s3_cfg = aws_sdk_s3::config::Builder::new()
                        .behavior_version_latest()
                        .endpoint_url(&t.endpoint)
                        .credentials_provider(creds)
                        .region(region)
                        .force_path_style(true)
                        .build();
                    let client = aws_sdk_s3::Client::from_conf(s3_cfg);
                    let _ = client
                        .delete_object()
                        .bucket(&t.bucket)
                        .key(mkey)
                        .send()
                        .await;
                }
            }
        }
        backup_repo.delete_row(oldest.id).await?;
    }
    Ok(())
}

pub async fn restore_backup(
    st: &AppState,
    backup_id: Uuid,
    target_backend_id: Uuid,
) -> Result<Uuid> {
    let backup_repo = BackupRepository::new(st.db.clone());
    let target_repo = BackupTargetRepository::new(st.db.clone());
    let backup = backup_repo
        .get(backup_id)
        .await?
        .ok_or_else(|| anyhow!("backup {backup_id} not found"))?;
    if backup.status != "completed" {
        return Err(anyhow!(
            "backup is in status '{}', expected 'completed'",
            backup.status
        ));
    }
    let manifest_key = backup
        .manifest_object_key
        .ok_or_else(|| anyhow!("backup has no manifest_object_key"))?;
    let target_row = target_repo
        .get(backup.target_id)
        .await?
        .ok_or_else(|| anyhow!("target {} no longer exists", backup.target_id))?;

    let backend = st
        .registry
        .get(target_backend_id)
        .ok_or_else(|| anyhow!("registry has no backend {target_backend_id}"))?
        .clone();
    let new_volume = backend
        .provision(nexus_storage::CreateOpts {
            name: format!("restore-{}", backup_id),
            size_bytes: backup.size_bytes as u64,
            description: Some(format!("restored from backup {backup_id}")),
        })
        .await
        .context("provision restore target")?;

    let kind_str = backend.kind().as_db_str();
    // Pick a host that supports the chosen backend.
    let candidate_host_id: Option<Uuid> = {
        let active = st.hosts.list_healthy().await?;
        let mut chosen = None;
        for h in active {
            let kinds = st
                .hosts
                .supported_backend_kinds(h.id)
                .await
                .unwrap_or_default();
            if kinds.is_empty() || kinds.iter().any(|k| k == kind_str) {
                chosen = Some(h.id);
                break;
            }
        }
        chosen
    };
    let host_id =
        candidate_host_id.ok_or_else(|| anyhow!("no host supports backend kind '{kind_str}'"))?;
    let host = st.hosts.get(host_id).await?;

    let attached = agent_rpc::agent_attach(&host.addr, &new_volume).await?;

    let secret = envelope::unwrap_to_string(&target_row.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target_row.encrypted_target_key)?;

    let req = RestoreReq {
        target_volume: new_volume.clone(),
        target_attached: attached.clone(),
        manifest_object_key: manifest_key.clone(),
        target: BackupTargetConfig {
            endpoint: target_row.endpoint,
            region: target_row.region,
            bucket: target_row.bucket,
            prefix: target_row.prefix,
            access_key_id: target_row.access_key_id,
            secret_access_key: secret,
        },
        encryption_key: target_key,
    };

    match agent_rpc::agent_restore(&host.addr, req).await {
        Ok(_) => {
            let volume_repo = VolumeRepository::new(st.db.clone());
            let inserted = volume_repo
                .create(
                    &format!("restore-{}", backup_id),
                    Some(&format!("Restored from backup {backup_id}")),
                    &new_volume.locator,
                    new_volume.size_bytes as i64,
                    "raw",
                    Some(host_id),
                    target_backend_id,
                )
                .await?;
            Ok(inserted.id)
        }
        Err(e) => {
            let _ = agent_rpc::agent_detach(&host.addr, &new_volume, &attached).await;
            let _ = backend.destroy(new_volume.clone()).await;
            Err(e)
        }
    }
}
