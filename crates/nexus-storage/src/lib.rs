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
}
