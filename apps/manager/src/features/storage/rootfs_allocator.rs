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
    }
}
