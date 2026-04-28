//! Generic iSCSI control-plane backend. Provisioning is no-op (operator must
//! pre-create LUNs on the target). The volume's locator stores a JSON object
//! with iqn, lun, and optional portal. clone_from_image is unsupported; the
//! slow path in rootfs_allocator handles populate via the agent.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::Deserialize;
use std::path::Path;

/// Task 15 wires this into AppState — suppress dead-code warnings until then.
#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct IscsiGenericConfig {
    pub target_iqn: String,
    #[serde(default)]
    pub portal: Option<String>,
}

/// Task 15 wires this into AppState — suppress dead-code warnings until then.
#[allow(dead_code)]
pub struct IscsiGenericControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: IscsiGenericConfig,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for IscsiGenericControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Iscsi
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            ..Default::default()
        } // all false
    }

    async fn provision(&self, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "generic iscsi: operator must pre-create LUNs and register them via API".into(),
        ))
    }

    async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> {
        Ok(())
    }

    async fn clone_from_image(
        &self,
        _src: &Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image".into()))
    }

    async fn snapshot(
        &self,
        _v: &VolumeHandle,
        _name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported("snapshot".into()))
    }

    async fn clone_from_snapshot(
        &self,
        _s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_snapshot".into()))
    }

    async fn delete_snapshot(&self, _s: VolumeSnapshotHandle) -> Result<(), StorageError> {
        Ok(())
    }
}
