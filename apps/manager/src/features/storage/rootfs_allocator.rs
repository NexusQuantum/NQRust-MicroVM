use crate::features::storage::registry::Registry;
use anyhow::{anyhow, Context, Result};
#[allow(unused_imports)]
use nexus_storage::{ControlPlaneBackend, CreateOpts, VolumeHandle};
use std::path::Path;
use uuid::Uuid;

/// Outcome of allocating a rootfs from a source image. The `volume_handle` is
/// always returned; `attached_for_caller` is `Some` only on the slow path
/// where the caller still holds an attachment that should be reused for VM
/// start (avoids detach/reattach round-trip).
#[allow(dead_code)]
pub struct AllocOutcome {
    pub volume_handle: VolumeHandle,
}

/// Allocate a rootfs by:
///   1. If backend supports clone_from_image → call it. Done.
///   2. Otherwise → provision empty volume; caller is responsible for
///      attach + populate_streaming + filesystem-aware resize on the agent.
///
/// **Filesystem-aware** here means: this function does NOT run `resize2fs`,
/// `mkfs`, or `e2fsck`. Those are caller responsibilities that depend on the
/// kind of source image (ext4 rootfs vs raw data disk vs qcow2 etc.). The
/// trait is, by spec, agnostic to filesystem types — see
/// `HostBackend::populate_streaming` doc.
#[allow(dead_code)]
pub async fn allocate_rootfs(
    registry: &Registry,
    backend_id: Uuid,
    source_image: &Path,
    target_size_bytes: u64,
    opts_name: &str,
) -> Result<AllocOutcome> {
    let backend = registry
        .get(backend_id)
        .ok_or_else(|| anyhow!("no backend with id {backend_id}"))?;
    let opts = CreateOpts {
        name: opts_name.to_string(),
        size_bytes: target_size_bytes,
        description: None,
    };
    if backend.capabilities().supports_clone_from_image {
        let h = backend
            .clone_from_image(source_image, opts)
            .await
            .with_context(|| format!("clone_from_image failed on backend {backend_id}"))?;
        return Ok(AllocOutcome { volume_handle: h });
    }
    // Slow path is implemented in Plan 2 once iSCSI exists. For now: refuse.
    Err(anyhow!(
        "backend {backend_id} does not support clone_from_image and the slow path is implemented in Plan 2"
    ))
}

/// Provision a blank data disk on the chosen backend.
#[allow(dead_code)]
pub async fn allocate_data_disk(
    registry: &Registry,
    backend_id: Uuid,
    size_bytes: u64,
    opts_name: &str,
) -> Result<VolumeHandle> {
    let backend = registry
        .get(backend_id)
        .ok_or_else(|| anyhow!("no backend with id {backend_id}"))?;
    let opts = CreateOpts {
        name: opts_name.to_string(),
        size_bytes,
        description: None,
    };
    backend
        .provision(opts)
        .await
        .with_context(|| format!("provision failed on backend {backend_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::storage::registry::Registry;

    #[tokio::test]
    #[ignore = "requires DATABASE_URL"]
    async fn allocate_rootfs_uses_fast_path_for_localfile() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        let reg = Registry::load(&pool, None).await.unwrap();
        let default_id = reg.default_id().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("img.bin");
        std::fs::write(&src, vec![0u8; 4 * 1024 * 1024]).unwrap();
        std::env::set_var("MANAGER_STORAGE_ROOT", dir.path());

        let out = allocate_rootfs(&reg, default_id, &src, 4 * 1024 * 1024, "test")
            .await
            .unwrap();
        assert_eq!(out.volume_handle.size_bytes, 4 * 1024 * 1024);
        // T23: verify locator path actually exists on disk after allocation
        assert!(
            std::path::Path::new(&out.volume_handle.locator).exists(),
            "locator path does not exist: {}",
            out.volume_handle.locator
        );
    }

    /// T24: The partial unique index `volume_one_active_attachment` on
    /// `volume_attachment(volume_id) WHERE detached_at IS NULL` must reject a
    /// second concurrent attach of the same volume.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL"]
    async fn double_attach_is_rejected_at_db_level() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let p = sqlx::PgPool::connect(&url).await.unwrap();

        // Resolve the localfile-default backend id.
        let backend_id: uuid::Uuid = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"SELECT id FROM storage_backend WHERE name = 'localfile-default'"#,
        )
        .fetch_one(&p)
        .await
        .unwrap();

        let host_id: Option<uuid::Uuid> = sqlx::query_scalar(r#"SELECT id FROM host LIMIT 1"#)
            .fetch_optional(&p)
            .await
            .unwrap()
            .flatten();

        // Insert a throwaway volume row.
        let vol_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO volume (id, name, path, size_bytes, type, status, host_id, backend_id)
               VALUES ($1, $2, $3, 1024, 'raw', 'available', $4, $5)"#,
        )
        .bind(vol_id)
        .bind(format!("vol-t24-{vol_id}"))
        .bind(format!("/tmp/t24-{vol_id}.img"))
        .bind(host_id)
        .bind(backend_id)
        .execute(&p)
        .await
        .unwrap();

        let vm1 = uuid::Uuid::new_v4();
        let vm2 = uuid::Uuid::new_v4();

        // First attach — may fail with 23503 (FK to vm) if the test DB enforces
        // the vm FK strictly. In that case skip gracefully.
        let first = sqlx::query(
            r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id)
               VALUES ($1, $2, 'rootfs')"#,
        )
        .bind(vol_id)
        .bind(vm1)
        .execute(&p)
        .await;

        if let Err(sqlx::Error::Database(ref db_err)) = first {
            if db_err.code().as_deref() == Some("23503") {
                // VM FK enforced — skip test rather than fail.
                sqlx::query("DELETE FROM volume WHERE id = $1")
                    .bind(vol_id)
                    .execute(&p)
                    .await
                    .ok();
                eprintln!(
                    "double_attach test skipped: vm FK enforced; \
                     full vm row setup required in this environment"
                );
                return;
            }
        }
        first.expect("first attach should succeed");

        // Second attach of the SAME volume to a different vm_id must fail with
        // unique-violation (23505) from the partial index.
        let second = sqlx::query(
            r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id)
               VALUES ($1, $2, 'rootfs')"#,
        )
        .bind(vol_id)
        .bind(vm2)
        .execute(&p)
        .await;

        match second {
            Err(sqlx::Error::Database(db_err)) => {
                assert_eq!(
                    db_err.code().as_deref(),
                    Some("23505"),
                    "expected unique-violation 23505, got {:?}",
                    db_err.code()
                );
            }
            other => panic!(
                "expected unique-violation 23505 for double attach, got {other:?}"
            ),
        }

        // Cleanup.
        sqlx::query("DELETE FROM volume_attachment WHERE volume_id = $1")
            .bind(vol_id)
            .execute(&p)
            .await
            .ok();
        sqlx::query("DELETE FROM volume WHERE id = $1")
            .bind(vol_id)
            .execute(&p)
            .await
            .ok();
    }
}
