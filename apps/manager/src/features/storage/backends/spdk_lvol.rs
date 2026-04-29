//! SPDK logical-volume control-plane backend.
//!
//! This first slice provisions lvols and snapshots through SPDK JSON-RPC. Image
//! import is intentionally not implemented yet because a vhost-user socket is a
//! Firecracker transport, not a writable block path.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts,
    SpdkJsonRpcClient, SpdkLvolLocator, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct SpdkLvolConfig {
    pub rpc_socket: PathBuf,
    pub lvs_name: String,
    #[allow(dead_code)]
    #[serde(default = "default_vhost_socket_dir")]
    pub vhost_socket_dir: PathBuf,
}

fn default_vhost_socket_dir() -> PathBuf {
    PathBuf::from("/var/tmp")
}

pub struct SpdkLvolControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: SpdkLvolConfig,
    pub client: SpdkJsonRpcClient,
}

impl SpdkLvolControlPlaneBackend {
    pub fn new(id: BackendInstanceId, config: SpdkLvolConfig) -> Self {
        let client = SpdkJsonRpcClient::new(config.rpc_socket.clone());
        Self { id, config, client }
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for SpdkLvolControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::SpdkLvol
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: false,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let volume_id = Uuid::new_v4();
        let lvol_name = spdk_name(&opts.name, volume_id);
        let lvol_uuid = self
            .client
            .bdev_lvol_create(&self.config.lvs_name, &lvol_name, opts.size_bytes)
            .await?;
        let locator = SpdkLvolLocator {
            lvs_name: self.config.lvs_name.clone(),
            lvol_name,
            lvol_uuid,
            size_bytes: opts.size_bytes,
        };
        Ok(VolumeHandle {
            volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::SpdkLvol,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let locator = SpdkLvolLocator::from_locator_str(&handle.locator)?;
        self.client.bdev_lvol_delete(&locator.lvol_uuid).await
    }

    async fn clone_from_image(
        &self,
        _source_image: &Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "spdk_lvol clone_from_image needs an image import path (NBD or bdev copy)".into(),
        ))
    }

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        let locator = SpdkLvolLocator::from_locator_str(&volume.locator)?;
        let snapshot_id = Uuid::new_v4();
        let snapshot_name = spdk_name(name, snapshot_id);
        let snapshot_uuid = self
            .client
            .bdev_lvol_snapshot(&locator.lvol_uuid, &snapshot_name)
            .await?;
        let snap_locator = SpdkLvolLocator {
            lvs_name: locator.lvs_name,
            lvol_name: snapshot_name,
            lvol_uuid: snapshot_uuid,
            size_bytes: locator.size_bytes,
        };
        Ok(VolumeSnapshotHandle {
            snapshot_id,
            source_volume_id: volume.volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::SpdkLvol,
            locator: snap_locator.to_locator_string()?,
        })
    }

    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        let snap_locator = SpdkLvolLocator::from_locator_str(&snap.locator)?;
        let volume_id = Uuid::new_v4();
        let lvol_name = spdk_name("clone", volume_id);
        let lvol_uuid = self
            .client
            .bdev_lvol_clone(&snap_locator.lvol_uuid, &lvol_name)
            .await?;
        let locator = SpdkLvolLocator {
            lvs_name: snap_locator.lvs_name,
            lvol_name,
            lvol_uuid,
            size_bytes: snap_locator.size_bytes,
        };
        Ok(VolumeHandle {
            volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::SpdkLvol,
            locator: locator.to_locator_string()?,
            size_bytes: snap_locator.size_bytes,
        })
    }

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let locator = SpdkLvolLocator::from_locator_str(&snap.locator)?;
        self.client.bdev_lvol_delete(&locator.lvol_uuid).await
    }
}

fn spdk_name(prefix: &str, id: Uuid) -> String {
    let mut cleaned = String::with_capacity(prefix.len());
    for ch in prefix.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            cleaned.push(ch);
        } else {
            cleaned.push('-');
        }
    }
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        format!("nq-{}", id.simple())
    } else {
        format!("nq-{cleaned}-{}", id.simple())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spdk_name_is_sanitized_and_stable() {
        let id = Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap();
        assert_eq!(
            spdk_name("rootfs / prod", id),
            "nq-rootfs---prod-018f64ba97aa70d9a7d26459256fd111"
        );
    }

    #[test]
    fn capabilities_match_single_node_spdk_lvol() {
        let backend = SpdkLvolControlPlaneBackend::new(
            BackendInstanceId(Uuid::nil()),
            SpdkLvolConfig {
                rpc_socket: "/run/spdk/rpc.sock".into(),
                lvs_name: "nexus".into(),
                vhost_socket_dir: "/var/tmp".into(),
            },
        );
        let caps = backend.capabilities();
        assert!(caps.supports_native_snapshots);
        assert!(!caps.supports_clone_from_image);
        assert!(!caps.supports_concurrent_attach);
    }
}
