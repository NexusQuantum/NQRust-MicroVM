use crate::features::storage::registry::Registry;
use anyhow::{anyhow, Context, Result};
#[allow(unused_imports)]
use nexus_storage::{AttachedPath, ControlPlaneBackend, CreateOpts, VolumeHandle};
use std::path::Path;
use uuid::Uuid;

/// Outcome of allocating a rootfs from a source image. The `volume_handle`
/// is always returned. When the slow path is taken (provision + agent
/// attach/populate), `attached_for_caller` is `Some(...)` so the caller can
/// reuse the attachment for VM start without a second agent round-trip.
/// For the fast path (`clone_from_image`), `attached_for_caller` is `None`.
#[allow(dead_code)]
pub struct AllocOutcome {
    pub volume_handle: VolumeHandle,
    pub attached_for_caller: Option<AttachedPath>,
}

/// Allocate a rootfs by:
///   1. If backend supports clone_from_image → call it. `attached_for_caller`
///      will be `None`.
///   2. Otherwise (slow path) → provision empty volume, agent-attach it,
///      agent-populate it from `source_image`, optionally run `resize2fs` if
///      the image is an ext4 rootfs. `attached_for_caller` will be
///      `Some(...)` so the caller can reuse the attachment.
#[allow(dead_code)]
pub async fn allocate_rootfs(
    registry: &Registry,
    backend_id: Uuid,
    host_addr: &str,
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
        return Ok(AllocOutcome {
            volume_handle: h,
            attached_for_caller: None,
        });
    }

    // Slow path: provision → agent attach → agent populate → optional resize2fs
    let h = backend
        .provision(opts)
        .await
        .with_context(|| format!("provision failed on backend {backend_id}"))?;
    let attached = crate::features::storage::agent_rpc::agent_attach(host_addr, &h)
        .await
        .context("agent attach")?;
    crate::features::storage::agent_rpc::agent_populate(
        host_addr,
        h.backend_kind,
        &attached,
        &source_image.to_path_buf(),
        target_size_bytes,
    )
    .await
    .context("agent populate_streaming")?;

    if image_is_ext4_rootfs(source_image).await.unwrap_or(false) {
        if let Err(e) =
            crate::features::storage::agent_rpc::agent_resize2fs(host_addr, &attached).await
        {
            tracing::warn!("resize2fs failed (non-fatal): {e:#}");
        }
    }

    Ok(AllocOutcome {
        volume_handle: h,
        attached_for_caller: Some(attached),
    })
}

async fn image_is_ext4_rootfs(path: &Path) -> Result<bool> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
    let mut f = tokio::fs::File::open(path).await?;
    if f.seek(SeekFrom::Start(1080)).await.is_err() {
        return Ok(false);
    }
    let mut buf = [0u8; 2];
    if f.read_exact(&mut buf).await.is_err() {
        return Ok(false);
    }
    // ext4 magic 0xEF53, little-endian
    Ok(buf[0] == 0x53 && buf[1] == 0xEF)
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

        let out = allocate_rootfs(
            &reg,
            default_id,
            "127.0.0.1:9090",
            &src,
            4 * 1024 * 1024,
            "test",
        )
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
            other => panic!("expected unique-violation 23505 for double attach, got {other:?}"),
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

    #[tokio::test]
    async fn detects_ext4_magic() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ext4.img");
        // Write 1082 bytes: pad with zeros, then magic 0xEF53 at offset 1080.
        let mut buf = vec![0u8; 1082];
        buf[1080] = 0x53;
        buf[1081] = 0xEF;
        std::fs::write(&p, &buf).unwrap();
        assert!(super::image_is_ext4_rootfs(&p).await.unwrap());
    }

    #[tokio::test]
    async fn rejects_non_ext4() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("plain.bin");
        std::fs::write(&p, vec![0u8; 4096]).unwrap();
        assert!(!super::image_is_ext4_rootfs(&p).await.unwrap());
    }

    #[tokio::test]
    async fn handles_short_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("short.bin");
        std::fs::write(&p, b"too short").unwrap();
        // Should return false, not panic
        assert!(!super::image_is_ext4_rootfs(&p).await.unwrap());
    }
}
