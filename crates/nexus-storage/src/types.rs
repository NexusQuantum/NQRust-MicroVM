use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifier of a configured backend instance (a row in `storage_backend`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendInstanceId(pub Uuid);

impl From<Uuid> for BackendInstanceId {
    fn from(u: Uuid) -> Self { Self(u) }
}
impl From<BackendInstanceId> for Uuid {
    fn from(id: BackendInstanceId) -> Self { id.0 }
}

/// What kind of storage system a backend speaks. New variants are added when
/// new backends are implemented; existing rows in the DB store this as a
/// snake_case string and are forward-compatible (unknown kinds at startup
/// cause the registry to log and skip the row, not crash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    LocalFile,
    Iscsi,
    TrueNasIscsi,
}

impl BackendKind {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            BackendKind::LocalFile => "local_file",
            BackendKind::Iscsi => "iscsi",
            BackendKind::TrueNasIscsi => "truenas_iscsi",
        }
    }
}

/// Capability bits the control plane consults for placement and gating.
/// `Default` is pessimistic: every flag false. Backends opt in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub supports_native_snapshots: bool,
    pub supports_concurrent_attach: bool,
    pub supports_live_migration: bool,
    pub supports_clone_from_image: bool,
}

/// Volume creation options. Add fields here when they're needed by a backend;
/// keep this struct flat — backend-specific options go through their own
/// config (registry-side, not per-call).
#[derive(Debug, Clone)]
pub struct CreateOpts {
    pub name: String,
    pub size_bytes: u64,
    /// Free-form description; not interpreted by backends.
    pub description: Option<String>,
}
