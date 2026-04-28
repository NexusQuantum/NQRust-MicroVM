//! Pluggable storage backend abstraction for NQRust-MicroVM.
//!
//! Two traits split across manager (`ControlPlaneBackend`) and agent
//! (`HostBackend`) processes. See
//! `docs/superpowers/specs/2026-04-28-storage-hci-design.md`.

pub mod control_plane;
pub mod error;
pub mod handle;
pub mod host;
pub mod types;

pub use control_plane::ControlPlaneBackend;
pub use error::StorageError;
pub use handle::{AttachedPath, VolumeHandle, VolumeSnapshotHandle};
pub use host::HostBackend;
pub use types::{BackendInstanceId, BackendKind, Capabilities, CreateOpts};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_round_trips_through_serde() {
        let kinds = [
            BackendKind::LocalFile,
            BackendKind::Iscsi,
            BackendKind::TrueNasIscsi,
        ];
        for k in kinds {
            let json = serde_json::to_string(&k).unwrap();
            let back: BackendKind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn capabilities_default_is_pessimistic() {
        let c = Capabilities::default();
        assert!(!c.supports_native_snapshots);
        assert!(!c.supports_concurrent_attach);
        assert!(!c.supports_live_migration);
        assert!(!c.supports_clone_from_image);
    }

    #[test]
    fn attached_path_exposes_path_for_each_variant() {
        use std::path::PathBuf;
        let f = AttachedPath::File(PathBuf::from("/tmp/x"));
        let b = AttachedPath::BlockDevice(PathBuf::from("/dev/sdb"));
        let v = AttachedPath::VhostUserSock(PathBuf::from("/run/spdk.sock"));
        assert_eq!(f.path(), std::path::Path::new("/tmp/x"));
        assert_eq!(b.path(), std::path::Path::new("/dev/sdb"));
        assert_eq!(v.path(), std::path::Path::new("/run/spdk.sock"));
    }

    #[test]
    fn already_attached_displays_clearly() {
        let e = StorageError::AlreadyAttached;
        assert_eq!(e.to_string(), "volume already attached");
    }

    #[test]
    fn not_supported_displays_clearly() {
        let e = StorageError::NotSupported("clone_from_image".into());
        assert!(e.to_string().contains("clone_from_image"));
    }

    /// A trait-shape compile test. If this compiles, the trait is object-safe
    /// (the registry stores `Arc<dyn ControlPlaneBackend>`).
    #[test]
    fn control_plane_backend_is_object_safe() {
        fn _assert<T: ControlPlaneBackend + ?Sized>() {}
        _assert::<dyn ControlPlaneBackend>();
    }

    #[test]
    fn host_backend_is_object_safe() {
        fn _assert<T: HostBackend + ?Sized>() {}
        _assert::<dyn HostBackend>();
    }

    /// T26: A backend that advertises `supports_clone_from_image: false` must
    /// return `StorageError::NotSupported` when `clone_from_image` is called.
    /// This validates the capability flag contract described in the trait doc.
    mod t26 {
        use super::*;
        use crate::error::StorageError;
        use crate::handle::{VolumeHandle, VolumeSnapshotHandle};
        use crate::types::{BackendInstanceId, CreateOpts};
        use async_trait::async_trait;
        use std::path::Path;
        use uuid::Uuid;

        struct UnsupportedBackend;

        #[async_trait]
        impl ControlPlaneBackend for UnsupportedBackend {
            fn kind(&self) -> BackendKind {
                BackendKind::Iscsi
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities {
                    supports_clone_from_image: false,
                    ..Default::default()
                }
            }
            async fn provision(&self, _o: CreateOpts) -> Result<VolumeHandle, StorageError> {
                Ok(VolumeHandle {
                    volume_id: Uuid::new_v4(),
                    backend_id: BackendInstanceId(Uuid::new_v4()),
                    backend_kind: BackendKind::Iscsi,
                    locator: "fake".into(),
                    size_bytes: 0,
                })
            }
            async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> {
                Ok(())
            }
            async fn clone_from_image(
                &self,
                _: &Path,
                _: CreateOpts,
            ) -> Result<VolumeHandle, StorageError> {
                Err(StorageError::NotSupported("clone_from_image".into()))
            }
            async fn snapshot(
                &self,
                _: &VolumeHandle,
                _: &str,
            ) -> Result<VolumeSnapshotHandle, StorageError> {
                Err(StorageError::NotSupported("snapshot".into()))
            }
            async fn clone_from_snapshot(
                &self,
                _: &VolumeSnapshotHandle,
            ) -> Result<VolumeHandle, StorageError> {
                Err(StorageError::NotSupported("clone_from_snapshot".into()))
            }
            async fn delete_snapshot(&self, _: VolumeSnapshotHandle) -> Result<(), StorageError> {
                Ok(())
            }
        }

        #[tokio::test]
        async fn clone_from_image_returns_not_supported_when_capability_is_false() {
            let b = UnsupportedBackend;
            assert!(!b.capabilities().supports_clone_from_image);
            let err = b
                .clone_from_image(
                    Path::new("/dev/null"),
                    CreateOpts {
                        name: "x".into(),
                        size_bytes: 0,
                        description: None,
                    },
                )
                .await
                .unwrap_err();
            assert!(matches!(err, StorageError::NotSupported(_)));
        }
    }
}
