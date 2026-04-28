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
