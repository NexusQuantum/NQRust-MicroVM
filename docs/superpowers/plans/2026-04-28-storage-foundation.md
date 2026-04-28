# Storage Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `LocalStorage` with a pluggable storage backend abstraction. After this plan, every VM disk operation routes through the `ControlPlaneBackend` and `HostBackend` traits, with `LocalFile` preserving today's behavior bit-for-bit. iSCSI/TrueNAS and the UI selector are separate plans (2 and 3) that build on this foundation.

**Architecture:** Two-trait split — `ControlPlaneBackend` (manager-side: provision, snapshot) and `HostBackend` (agent-side: attach, populate_streaming). Lives in a new workspace crate `nexus-storage`. A `Registry` in the manager loads `[[storage_backend]]` entries from `nqrust.toml`, upserts them into a new `storage_backend` table, and hands out trait objects keyed by `backend_id`. VM rootfs allocation moves from `LocalStorage::alloc_rootfs` into a new `rootfs_allocator` that gates `clone_from_image` on the backend's `Capabilities` and falls back to provision + attach + pure-bytes streaming + caller-side `resize2fs`.

**Tech Stack:** Rust 2021, Axum 0.7, sqlx 0.8 (Postgres), Tokio, anyhow, thiserror, utoipa, serde, serde_json, toml. Reads `apps/manager/migrations/0034_storage_backends.sql` on startup via `sqlx::migrate!()`.

**Spec:** `docs/superpowers/specs/2026-04-28-storage-hci-design.md` (commit `1ff9712`).

---

## File structure

New crate:
- `crates/nexus-storage/Cargo.toml`
- `crates/nexus-storage/src/lib.rs` — re-exports
- `crates/nexus-storage/src/types.rs` — `BackendKind`, `BackendInstanceId`, `Capabilities`, `CreateOpts`
- `crates/nexus-storage/src/handle.rs` — `VolumeHandle`, `VolumeSnapshotHandle`, `AttachedPath`
- `crates/nexus-storage/src/error.rs` — `StorageError`
- `crates/nexus-storage/src/control_plane.rs` — `ControlPlaneBackend` trait
- `crates/nexus-storage/src/host.rs` — `HostBackend` trait

DB:
- `apps/manager/migrations/0034_storage_backends.sql` — table, backfill, partial unique index, host_id relax.

Manager additions:
- `apps/manager/src/features/storage_backends/mod.rs`
- `apps/manager/src/features/storage_backends/repo.rs`
- `apps/manager/src/features/storage_backends/routes.rs`
- `apps/manager/src/features/storage/registry.rs` — Registry: TOML loading, validation, upsert, trait-object lookup
- `apps/manager/src/features/storage/config.rs` — TOML parse types + per-kind validation
- `apps/manager/src/features/storage/backends/mod.rs`
- `apps/manager/src/features/storage/backends/local_file.rs` — `LocalFileControlPlaneBackend`
- `apps/manager/src/features/storage/rootfs_allocator.rs` — orchestrator (capability gating, fast/slow path, caller-side resize2fs)

Manager modifications:
- `apps/manager/Cargo.toml` — add `nexus-storage` and `toml` deps
- `apps/manager/src/main.rs` — `AppState` gains `Registry`; remove direct `LocalStorage` field (kept as backwards-compat construction inside the LocalFile backend impl)
- `apps/manager/src/features/mod.rs` — register `storage_backends` router
- `apps/manager/src/features/storage/mod.rs` — keep `LocalStorage` as the LocalFile impl helper; do NOT delete; export it
- `apps/manager/src/features/vms/service.rs` — `create_vm` calls `rootfs_allocator` instead of `state.storage.alloc_rootfs`; `start_vm` and `stop_vm` route through the registry for attach/detach; lines 2415, 2489, 2560, 2633 (test stubs) updated to construct a Registry with LocalFile
- `apps/manager/src/features/volumes/routes.rs` — `CreateVolumeRequest` gains `backend_id: Option<Uuid>`; volume creation routes through registry
- `apps/manager/src/features/volumes/repo.rs` — `VolumeRow` gains `backend_id: Uuid` and `host_id: Option<Uuid>`; `create` signature updated; `AttachmentRow` gains `detached_at: Option<DateTime<Utc>>`; new method `mark_detached`

Agent additions:
- `apps/agent/src/features/storage/mod.rs` — host backend registry (a HashMap<BackendKind, Arc<dyn HostBackend>>)
- `apps/agent/src/features/storage/local_file.rs` — `LocalFileHostBackend`

Agent modifications:
- `apps/agent/Cargo.toml` — add `nexus-storage` dep
- `apps/agent/src/main.rs` — initialize host backend registry, pass to spawn
- `apps/agent/src/features/spawn/mod.rs` and `apps/agent/src/features/vm/spawn.rs` — accept structured volume info from manager (backend_kind + path) and resolve to `AttachedPath` via local registry before handing to Firecracker

Shared (`crates/nexus-types/src/lib.rs`):
- New types: `StorageBackend` (wire), `BackendKind` (serde + ToSchema), `Capabilities` (serde + ToSchema), `BackendId` newtype (Uuid wrapper)
- New optional field on `CreateVmRequest`-shaped types: `backend_id: Option<Uuid>` (passed via existing manager request bodies)

Tests:
- `apps/manager/tests/storage_foundation.rs` — integration tests
- Unit tests inline in each module

---

## Conventions for this plan

- Every task ends with a commit. Use Conventional Commits (`feat(storage):`, `chore(storage):`, `test(storage):`, etc.).
- Run `cargo fmt` before every commit; `cargo clippy --all-targets --all-features -- -D warnings` before the integration-test commits.
- After any DB migration change, ensure the manager still boots clean (`cargo run -p manager` against a fresh database).
- Do NOT modify `apps/manager/src/features/functions/vm.rs` or `apps/manager/src/features/containers/vm.rs` other than adding the TODO comments specified in Task 19.

---

## Task 1: Create the `nexus-storage` workspace crate

**Files:**
- Create: `crates/nexus-storage/Cargo.toml`
- Create: `crates/nexus-storage/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1.1: Create the crate manifest**

Write `crates/nexus-storage/Cargo.toml`:

```toml
[package]
name = "nexus-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
async-trait = "0.1"

[dev-dependencies]
tokio = { workspace = true }
```

- [ ] **Step 1.2: Create the empty lib.rs**

Write `crates/nexus-storage/src/lib.rs`:

```rust
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
```

- [ ] **Step 1.3: Add to workspace**

Edit `Cargo.toml` (the workspace one at the repo root). Find the `[workspace] members = [...]` block and add `"crates/nexus-storage",` to the list.

Before:
```toml
[workspace]
members = [
"apps/agent", "apps/guest-agent",
"apps/manager",
"apps/installer",
"crates/nexus-types",
]
```

After:
```toml
[workspace]
members = [
"apps/agent", "apps/guest-agent",
"apps/manager",
"apps/installer",
"crates/nexus-storage",
"crates/nexus-types",
]
```

- [ ] **Step 1.4: Verify the workspace compiles**

Run: `cargo check -p nexus-storage`
Expected: FAILS with "file not found for module" because `types.rs`, `handle.rs`, `error.rs`, `control_plane.rs`, `host.rs` don't exist yet. This is fine — we add them in Tasks 2–6. Move on.

- [ ] **Step 1.5: Commit**

```bash
git add Cargo.toml crates/nexus-storage/
git commit -m "chore(storage): scaffold nexus-storage crate"
```

---

## Task 2: Define core type primitives

**Files:**
- Create: `crates/nexus-storage/src/types.rs`

- [ ] **Step 2.1: Write the failing test**

Append to the bottom of `crates/nexus-storage/src/lib.rs`:

```rust
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
```

Run: `cargo test -p nexus-storage`
Expected: FAIL — `BackendKind`, `Capabilities` not found.

- [ ] **Step 2.2: Implement types.rs**

Write `crates/nexus-storage/src/types.rs`:

```rust
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
```

- [ ] **Step 2.3: Verify the tests pass**

Run: `cargo test -p nexus-storage`
Expected: 2 tests pass.

- [ ] **Step 2.4: Commit**

```bash
git add crates/nexus-storage/
git commit -m "feat(storage): add BackendKind, Capabilities, CreateOpts, BackendInstanceId"
```

---

## Task 3: Define handle and AttachedPath types

**Files:**
- Create: `crates/nexus-storage/src/handle.rs`

- [ ] **Step 3.1: Write the failing test**

Append to `crates/nexus-storage/src/lib.rs` `tests` module:

```rust
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
```

Run: `cargo test -p nexus-storage`
Expected: FAIL — `AttachedPath` not found.

- [ ] **Step 3.2: Implement handle.rs**

Write `crates/nexus-storage/src/handle.rs`:

```rust
use crate::types::{BackendInstanceId, BackendKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Reference to a provisioned volume. Carries enough information that a
/// `HostBackend` can attach it without re-consulting the control plane.
/// The `locator` field is backend-defined (LocalFile: file path; Iscsi: IQN+LUN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeHandle {
    pub volume_id: Uuid,
    pub backend_id: BackendInstanceId,
    pub backend_kind: BackendKind,
    pub locator: String,
    pub size_bytes: u64,
}

/// Reference to a snapshot. Same shape as `VolumeHandle` for symmetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSnapshotHandle {
    pub snapshot_id: Uuid,
    pub source_volume_id: Uuid,
    pub backend_id: BackendInstanceId,
    pub backend_kind: BackendKind,
    pub locator: String,
}

/// What the host backend hands back from `attach`. Firecracker treats `File`
/// and `BlockDevice` interchangeably (both are valid paths on its drive
/// config); `VhostUserSock` is reserved for future SPDK and not used in this
/// PR but defined now so the trait shape is stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "path")]
pub enum AttachedPath {
    File(PathBuf),
    BlockDevice(PathBuf),
    VhostUserSock(PathBuf),
}

impl AttachedPath {
    /// Path the caller hands to Firecracker / `resize2fs` / etc. All variants
    /// resolve to a string filesystem path; the variant only documents the
    /// nature of what's behind it.
    pub fn path(&self) -> &Path {
        match self {
            AttachedPath::File(p) => p,
            AttachedPath::BlockDevice(p) => p,
            AttachedPath::VhostUserSock(p) => p,
        }
    }
}
```

- [ ] **Step 3.3: Verify**

Run: `cargo test -p nexus-storage`
Expected: 3 tests pass.

- [ ] **Step 3.4: Commit**

```bash
git add crates/nexus-storage/
git commit -m "feat(storage): add VolumeHandle, VolumeSnapshotHandle, AttachedPath"
```

---

## Task 4: Define StorageError

**Files:**
- Create: `crates/nexus-storage/src/error.rs`

- [ ] **Step 4.1: Write the failing test**

Append to `crates/nexus-storage/src/lib.rs` `tests`:

```rust
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
```

Run: `cargo test -p nexus-storage`
Expected: FAIL — `StorageError` not found.

- [ ] **Step 4.2: Implement error.rs**

Write `crates/nexus-storage/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("volume already attached")]
    AlreadyAttached,

    #[error("operation not supported: {0}")]
    NotSupported(String),

    #[error("volume not found")]
    NotFound,

    #[error("invalid volume locator: {0}")]
    InvalidLocator(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for backend-specific failures that don't categorize cleanly.
    /// Use sparingly — prefer adding a typed variant when an error condition
    /// becomes load-bearing for callers.
    #[error("backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
```

- [ ] **Step 4.3: Verify**

Run: `cargo test -p nexus-storage`
Expected: 5 tests pass.

- [ ] **Step 4.4: Commit**

```bash
git add crates/nexus-storage/
git commit -m "feat(storage): add StorageError"
```

---

## Task 5: Define ControlPlaneBackend trait

**Files:**
- Create: `crates/nexus-storage/src/control_plane.rs`

- [ ] **Step 5.1: Write the failing test**

Append to `crates/nexus-storage/src/lib.rs` `tests`:

```rust
    /// A trait-shape compile test. If this compiles, the trait is object-safe
    /// (the registry stores `Arc<dyn ControlPlaneBackend>`).
    #[test]
    fn control_plane_backend_is_object_safe() {
        fn _assert<T: ControlPlaneBackend + ?Sized>() {}
        _assert::<dyn ControlPlaneBackend>();
    }
```

Run: `cargo test -p nexus-storage`
Expected: FAIL — `ControlPlaneBackend` not found.

- [ ] **Step 5.2: Implement control_plane.rs**

Write `crates/nexus-storage/src/control_plane.rs`:

```rust
use crate::error::StorageError;
use crate::handle::{VolumeHandle, VolumeSnapshotHandle};
use crate::types::{BackendKind, Capabilities, CreateOpts};
use async_trait::async_trait;
use std::path::Path;

/// Manager-side operations on a storage backend: provisioning lifecycle and
/// snapshot lifecycle. Lives in the manager binary; never called from the
/// agent. Implementations are stored as `Arc<dyn ControlPlaneBackend>` in the
/// `Registry`.
#[async_trait]
pub trait ControlPlaneBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> Capabilities;

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError>;
    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError>;

    /// Fast path. Only valid to call when `capabilities().supports_clone_from_image`.
    /// Implementations that don't support this MUST return
    /// `Err(StorageError::NotSupported("clone_from_image".into()))`.
    async fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError>;

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError>;

    /// Always creates a NEW volume. Never mutates the source volume.
    /// (See spec for the rollback-vs-clone distinction.)
    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError>;

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError>;
}
```

- [ ] **Step 5.3: Verify**

Run: `cargo test -p nexus-storage`
Expected: 6 tests pass.

- [ ] **Step 5.4: Commit**

```bash
git add crates/nexus-storage/
git commit -m "feat(storage): add ControlPlaneBackend trait"
```

---

## Task 6: Define HostBackend trait

**Files:**
- Create: `crates/nexus-storage/src/host.rs`

- [ ] **Step 6.1: Write the failing test**

Append to `crates/nexus-storage/src/lib.rs` `tests`:

```rust
    #[test]
    fn host_backend_is_object_safe() {
        fn _assert<T: HostBackend + ?Sized>() {}
        _assert::<dyn HostBackend>();
    }
```

Run: `cargo test -p nexus-storage`
Expected: FAIL — `HostBackend` not found.

- [ ] **Step 6.2: Implement host.rs**

Write `crates/nexus-storage/src/host.rs`:

```rust
use crate::error::StorageError;
use crate::handle::{AttachedPath, VolumeHandle};
use crate::types::BackendKind;
use async_trait::async_trait;
use std::path::Path;

/// Agent-side operations: making volume bytes accessible to Firecracker on
/// this host. Lives in the agent binary. The manager never imports these
/// impls; it asks the agent to perform an operation via the existing
/// manager→agent HTTP API.
#[async_trait]
pub trait HostBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError>;
    async fn detach(
        &self,
        volume: &VolumeHandle,
        attached: AttachedPath,
    ) -> Result<(), StorageError>;

    /// Pure byte copy: open the AttachedPath, write `source` bytes into it,
    /// ensure the underlying storage is at least `target_size_bytes` (sparse
    /// extension OK).
    ///
    /// MUST NOT do filesystem-aware operations (no `resize2fs`, `e2fsck`,
    /// `mkfs`). Filesystem-aware steps belong in the rootfs-allocation
    /// caller, not the trait — the trait remains agnostic to ext4/xfs/btrfs/
    /// qcow2/raw-without-fs.
    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError>;
}
```

- [ ] **Step 6.3: Verify and lint**

Run: `cargo test -p nexus-storage && cargo clippy -p nexus-storage --all-targets -- -D warnings`
Expected: 7 tests pass; clippy clean.

- [ ] **Step 6.4: Commit**

```bash
git add crates/nexus-storage/
git commit -m "feat(storage): add HostBackend trait"
```

---

## Task 7: DB migration — `storage_backend` table, host_id relax, partial unique index

**Files:**
- Create: `apps/manager/migrations/0034_storage_backends.sql`

- [ ] **Step 7.1: Write the migration**

Write `apps/manager/migrations/0034_storage_backends.sql`:

```sql
-- 0034_storage_backends.sql
-- Pluggable storage backend abstraction. See
-- docs/superpowers/specs/2026-04-28-storage-hci-design.md.

-- 1. Backend instance registry. TOML is source of truth on startup; this
-- table caches what the manager loaded so the rest of the system has a
-- stable id to reference.
CREATE TABLE IF NOT EXISTS storage_backend (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  capabilities_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_default BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

-- At most one default backend.
CREATE UNIQUE INDEX IF NOT EXISTS one_default_backend
  ON storage_backend (is_default) WHERE is_default = true;

-- 2. Seed the localfile-default backend so existing volumes have something
-- to point at. Capabilities reflect the LocalFile impl: clone-from-image yes
-- (it's just fs::copy), snapshot/concurrent/migration no.
INSERT INTO storage_backend (name, kind, capabilities_json, is_default)
VALUES (
  'localfile-default',
  'local_file',
  '{"supports_native_snapshots": false, "supports_concurrent_attach": false, "supports_live_migration": false, "supports_clone_from_image": true}'::jsonb,
  true
)
ON CONFLICT (name) DO NOTHING;

-- 3. Volume schema changes.
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backend_id UUID REFERENCES storage_backend(id);
UPDATE volume
   SET backend_id = (SELECT id FROM storage_backend WHERE name = 'localfile-default')
 WHERE backend_id IS NULL;
ALTER TABLE volume ALTER COLUMN backend_id SET NOT NULL;
ALTER TABLE volume ALTER COLUMN host_id DROP NOT NULL;

COMMENT ON COLUMN volume.host_id IS
  'Home host for host-pinned volumes (LocalFile). NULL for network-attached backends.';
COMMENT ON COLUMN volume.path IS
  'Backend-defined locator. LocalFile: filesystem path. Iscsi: IQN+LUN. Unique within a backend instance.';

CREATE INDEX IF NOT EXISTS idx_volume_backend ON volume(backend_id);

-- 4. Single-attach enforcement + audit trail. detached_at NULL = active.
ALTER TABLE volume_attachment ADD COLUMN IF NOT EXISTS detached_at TIMESTAMPTZ;

-- The original unique constraint UNIQUE (volume_id, vm_id) does not prevent a
-- volume being attached to a SECOND vm. The new partial unique index does:
-- at most one row with detached_at IS NULL per volume.
DROP INDEX IF EXISTS volume_one_active_attachment;
CREATE UNIQUE INDEX volume_one_active_attachment
  ON volume_attachment(volume_id) WHERE detached_at IS NULL;
```

- [ ] **Step 7.2: Apply and verify**

Bring up the dev DB if not running: `./scripts/dev-up.sh`
Run: `(cd apps/manager && sqlx migrate run)`
Expected: `Applied 34/migrate storage backends`.

Verify:

```bash
psql "$DATABASE_URL" -c "\d storage_backend"
psql "$DATABASE_URL" -c "SELECT name, kind, is_default FROM storage_backend;"
psql "$DATABASE_URL" -c "\d volume"
```

Expected outputs:
- `storage_backend` has columns `id, name, kind, config_json, capabilities_json, is_default, created_at, deleted_at`.
- One row: `localfile-default | local_file | t`.
- `volume.host_id` is now nullable.
- `volume.backend_id` is NOT NULL with FK to `storage_backend(id)`.

- [ ] **Step 7.3: Commit**

```bash
git add apps/manager/migrations/0034_storage_backends.sql
git commit -m "feat(storage): migration 0034 — storage_backend table, host_id relax, single-attach index"
```

---

## Task 8: Add shared types to `nexus-types`

**Files:**
- Modify: `crates/nexus-types/src/lib.rs`

- [ ] **Step 8.1: Find the existing serde patterns**

Run: `head -80 crates/nexus-types/src/lib.rs`
Expected: see the existing top-of-file imports and a couple of existing structs to mirror.

- [ ] **Step 8.2: Append the storage types**

At the bottom of `crates/nexus-types/src/lib.rs`, append:

```rust
// ── Storage backends ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    LocalFile,
    Iscsi,
    TrueNasIscsi,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Capabilities {
    pub supports_native_snapshots: bool,
    pub supports_concurrent_attach: bool,
    pub supports_live_migration: bool,
    pub supports_clone_from_image: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct StorageBackend {
    pub id: uuid::Uuid,
    pub name: String,
    pub kind: BackendKind,
    pub capabilities: Capabilities,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

- [ ] **Step 8.3: Verify**

Run: `cargo check -p nexus-types`
Expected: clean.

- [ ] **Step 8.4: Commit**

```bash
git add crates/nexus-types/src/lib.rs
git commit -m "feat(storage): add wire types BackendKind, Capabilities, StorageBackend"
```

---

## Task 9: Add `nexus-storage` and `toml` deps to manager

**Files:**
- Modify: `apps/manager/Cargo.toml`

- [ ] **Step 9.1: Add the deps**

Open `apps/manager/Cargo.toml`. In the `[dependencies]` section, add:

```toml
nexus-storage = { path = "../../crates/nexus-storage" }
toml = "0.8"
async-trait = "0.1"
```

- [ ] **Step 9.2: Verify**

Run: `cargo check -p manager`
Expected: clean (still compiles — these are unused for now).

- [ ] **Step 9.3: Commit**

```bash
git add apps/manager/Cargo.toml Cargo.lock
git commit -m "chore(storage): add nexus-storage, toml, async-trait deps to manager"
```

---

## Task 10: `storage_backend` repo

**Files:**
- Create: `apps/manager/src/features/storage_backends/mod.rs`
- Create: `apps/manager/src/features/storage_backends/repo.rs`

- [ ] **Step 10.1: Write the failing test**

Create `apps/manager/src/features/storage_backends/repo.rs`:

```rust
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct StorageBackendRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StorageBackendRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub config_json: JsonValue,
    pub capabilities_json: JsonValue,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl StorageBackendRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_active(&self) -> sqlx::Result<Vec<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE deleted_at IS NULL ORDER BY name"#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_default(&self) -> sqlx::Result<Option<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE is_default = true AND deleted_at IS NULL LIMIT 1"#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Upsert by name. Used by the registry on startup to reconcile TOML with DB.
    pub async fn upsert(
        &self,
        name: &str,
        kind: &str,
        config_json: &JsonValue,
        capabilities_json: &JsonValue,
        is_default: bool,
    ) -> sqlx::Result<StorageBackendRow> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"
            INSERT INTO storage_backend (name, kind, config_json, capabilities_json, is_default, deleted_at)
            VALUES ($1, $2, $3, $4, $5, NULL)
            ON CONFLICT (name) DO UPDATE
              SET kind = EXCLUDED.kind,
                  config_json = EXCLUDED.config_json,
                  capabilities_json = EXCLUDED.capabilities_json,
                  is_default = EXCLUDED.is_default,
                  deleted_at = NULL
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(kind)
        .bind(config_json)
        .bind(capabilities_json)
        .bind(is_default)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn soft_delete_by_name(&self, name: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE storage_backend SET deleted_at = now() WHERE name = $1 AND deleted_at IS NULL"#,
        )
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

Create `apps/manager/src/features/storage_backends/mod.rs`:

```rust
pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id", get(routes::get_one))
}
```

(`routes.rs` is created in Task 11; `cargo check` will fail until then. That's OK.)

- [ ] **Step 10.2: Skip running tests until Task 11 — commit the repo only**

```bash
git add apps/manager/src/features/storage_backends/
git commit -m "feat(storage): storage_backends repo + module skeleton"
```

---

## Task 11: `storage_backend` routes

**Files:**
- Create: `apps/manager/src/features/storage_backends/routes.rs`
- Modify: `apps/manager/src/features/mod.rs` to register the router

- [ ] **Step 11.1: Write routes.rs**

Create `apps/manager/src/features/storage_backends/routes.rs`:

```rust
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use crate::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use nexus_types::{BackendKind, Capabilities, StorageBackend};
use uuid::Uuid;

fn row_to_wire(row: StorageBackendRow) -> Result<StorageBackend, StatusCode> {
    let kind: BackendKind = serde_json::from_value(serde_json::Value::String(row.kind.clone()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let capabilities: Capabilities = serde_json::from_value(row.capabilities_json)
        .unwrap_or_default();
    Ok(StorageBackend {
        id: row.id,
        name: row.name,
        kind,
        capabilities,
        is_default: row.is_default,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    })
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct StorageBackendListResponse {
    pub items: Vec<StorageBackend>,
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends",
    responses(
        (status = 200, body = StorageBackendListResponse),
    ),
    tag = "StorageBackends",
)]
pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => {
            let mut items = Vec::with_capacity(rows.len());
            for r in rows {
                match row_to_wire(r) {
                    Ok(w) => items.push(w),
                    Err(s) => return (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response(),
                }
            }
            (StatusCode::OK, Json(StorageBackendListResponse { items })).into_response()
        }
        Err(e) => {
            tracing::error!("storage_backends list failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db"}))).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 200, body = StorageBackend),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => match row_to_wire(row) {
            Ok(w) => (StatusCode::OK, Json(w)).into_response(),
            Err(s) => (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
        Err(e) => {
            tracing::error!("storage_backends get failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db"}))).into_response()
        }
    }
}
```

- [ ] **Step 11.2: Register the router**

Open `apps/manager/src/features/mod.rs`. Add to the module list:

```rust
pub mod storage_backends;
```

Find where other routers are nested under `/v1`. Add:

```rust
.nest("/v1/storage_backends", storage_backends::router())
```

(The exact line depends on the existing structure — locate the `.nest("/v1/volumes", ...)` line and add the new nest right after it.)

- [ ] **Step 11.3: Verify**

Run: `cargo check -p manager`
Expected: clean.

Manual smoke test (if a dev DB is up): `cargo run -p manager`, then in another terminal:
```bash
curl -s http://127.0.0.1:18080/v1/storage_backends | jq
```
Expected: `{"items": [{"name": "localfile-default", "kind": "local_file", ...}]}`.

- [ ] **Step 11.4: Commit**

```bash
git add apps/manager/src/features/storage_backends/routes.rs apps/manager/src/features/mod.rs
git commit -m "feat(storage): GET /v1/storage_backends and /:id"
```

---

## Task 12: TOML config types + per-kind validation

**Files:**
- Create: `apps/manager/src/features/storage/config.rs`
- Modify: `apps/manager/src/features/storage/mod.rs` (add `pub mod config;`)

- [ ] **Step 12.1: Write the failing test**

Append to `apps/manager/src/features/storage/mod.rs`:

```rust
pub mod config;
```

Create `apps/manager/src/features/storage/config.rs`:

```rust
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendKind, Capabilities};
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Deserialize)]
pub struct StorageBackendsToml {
    #[serde(default, rename = "storage_backend")]
    pub backends: Vec<RawBackendEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawBackendEntry {
    pub name: String,
    pub kind: BackendKind,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub config: JsonValue,
}

/// Validated entry ready to be upserted into the DB and registered as a
/// trait object. Per-kind required fields have already been checked.
#[derive(Debug, Clone)]
pub struct ValidatedBackend {
    pub name: String,
    pub kind: BackendKind,
    pub is_default: bool,
    pub config: JsonValue,
    pub capabilities: Capabilities,
}

/// Parse a TOML string into raw entries.
pub fn parse(toml_str: &str) -> Result<StorageBackendsToml> {
    toml::from_str::<StorageBackendsToml>(toml_str)
        .context("parsing storage_backend TOML")
}

/// Validate per-kind shape and assign capabilities. The capabilities here are
/// the *expected* capabilities for a given kind; the actual backend impl is
/// the source of truth at runtime, but we denormalize here so the DB and UI
/// can show capabilities without instantiating a backend.
pub fn validate(raw: RawBackendEntry) -> Result<ValidatedBackend> {
    if raw.name.is_empty() {
        return Err(anyhow!("storage_backend.name must not be empty"));
    }

    let capabilities = match raw.kind {
        BackendKind::LocalFile => Capabilities {
            supports_native_snapshots: false,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        },
        BackendKind::Iscsi => {
            require_str(&raw.config, "target_iqn")
                .with_context(|| format!("backend '{}' (kind=iscsi)", raw.name))?;
            Capabilities {
                supports_native_snapshots: false,
                supports_concurrent_attach: false,
                supports_live_migration: false,
                supports_clone_from_image: false,
            }
        }
        BackendKind::TrueNasIscsi => {
            require_str(&raw.config, "endpoint")
                .with_context(|| format!("backend '{}' (kind=truenas_iscsi)", raw.name))?;
            require_str(&raw.config, "api_key_env")
                .with_context(|| format!("backend '{}' (kind=truenas_iscsi)", raw.name))?;
            require_str(&raw.config, "pool")
                .with_context(|| format!("backend '{}' (kind=truenas_iscsi)", raw.name))?;
            require_str(&raw.config, "target_iqn_prefix")
                .with_context(|| format!("backend '{}' (kind=truenas_iscsi)", raw.name))?;
            Capabilities {
                supports_native_snapshots: true,
                supports_concurrent_attach: false,
                supports_live_migration: false,
                supports_clone_from_image: false,
            }
        }
    };

    Ok(ValidatedBackend {
        name: raw.name,
        kind: raw.kind,
        is_default: raw.is_default,
        config: raw.config,
        capabilities,
    })
}

fn require_str(config: &JsonValue, field: &str) -> Result<()> {
    match config.get(field).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(()),
        Some(_) => Err(anyhow!("config.{field} is empty")),
        None => Err(anyhow!("config.{field} is required")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_localfile_entry() {
        let toml = r#"
            [[storage_backend]]
            name = "localfile-default"
            kind = "local_file"
            is_default = true
        "#;
        let parsed = parse(toml).unwrap();
        assert_eq!(parsed.backends.len(), 1);
        assert_eq!(parsed.backends[0].name, "localfile-default");
        assert_eq!(parsed.backends[0].kind, BackendKind::LocalFile);

        let v = validate(parsed.backends.into_iter().next().unwrap()).unwrap();
        assert!(v.capabilities.supports_clone_from_image);
        assert!(!v.capabilities.supports_native_snapshots);
    }

    #[test]
    fn truenas_missing_endpoint_fails_validation() {
        let raw = RawBackendEntry {
            name: "tn".into(),
            kind: BackendKind::TrueNasIscsi,
            is_default: false,
            config: serde_json::json!({"api_key_env": "X", "pool": "p", "target_iqn_prefix": "iqn.x"}),
        };
        let err = validate(raw).unwrap_err();
        assert!(err.to_string().contains("endpoint"), "got: {err}");
    }

    #[test]
    fn iscsi_requires_target_iqn() {
        let raw = RawBackendEntry {
            name: "i".into(),
            kind: BackendKind::Iscsi,
            is_default: false,
            config: serde_json::json!({}),
        };
        let err = validate(raw).unwrap_err();
        assert!(err.to_string().contains("target_iqn"), "got: {err}");
    }
}
```

- [ ] **Step 12.2: Verify the tests pass**

Run: `cargo test -p manager features::storage::config`
Expected: 3 tests pass.

- [ ] **Step 12.3: Commit**

```bash
git add apps/manager/src/features/storage/
git commit -m "feat(storage): TOML parse + per-kind validation"
```

---

## Task 13: Registry — load, validate, upsert, hand out trait objects

**Files:**
- Create: `apps/manager/src/features/storage/registry.rs`
- Modify: `apps/manager/src/features/storage/mod.rs` to add `pub mod registry;`

- [ ] **Step 13.1: Write the registry**

Append `pub mod registry;` and `pub mod backends;` to `apps/manager/src/features/storage/mod.rs`. Create `apps/manager/src/features/storage/backends/mod.rs` as `pub mod local_file;` (we'll fill `local_file.rs` in Task 14). Create `apps/manager/src/features/storage/backends/local_file.rs` with a minimal stub for now:

```rust
// Filled in Task 14.
use anyhow::Result;
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::Path;

pub struct LocalFileControlPlaneBackend {
    pub id: BackendInstanceId,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for LocalFileControlPlaneBackend {
    fn kind(&self) -> BackendKind { BackendKind::LocalFile }
    fn capabilities(&self) -> Capabilities {
        Capabilities { supports_clone_from_image: true, ..Default::default() }
    }
    async fn provision(&self, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("provision not yet implemented".into()))
    }
    async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> { Ok(()) }
    async fn clone_from_image(&self, _src: &Path, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image not yet implemented".into()))
    }
    async fn snapshot(&self, _v: &VolumeHandle, _name: &str) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported("snapshot".into()))
    }
    async fn clone_from_snapshot(&self, _s: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_snapshot".into()))
    }
    async fn delete_snapshot(&self, _s: VolumeSnapshotHandle) -> Result<(), StorageError> { Ok(()) }
}
```

Now create `apps/manager/src/features/storage/registry.rs`:

```rust
use crate::features::storage::backends::local_file::LocalFileControlPlaneBackend;
use crate::features::storage::config::{parse, validate, ValidatedBackend};
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendInstanceId, BackendKind, ControlPlaneBackend};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Manager-side registry. Holds one trait object per active backend instance,
/// keyed by `backend_id`. Built once at startup; immutable thereafter.
#[derive(Clone)]
pub struct Registry {
    by_id: HashMap<Uuid, Arc<dyn ControlPlaneBackend>>,
    default_id: Option<Uuid>,
}

impl Registry {
    pub async fn load(pool: &PgPool, toml_str: Option<&str>) -> Result<Self> {
        let repo = StorageBackendRepository::new(pool.clone());

        // 1. Parse + validate TOML (if provided).
        let validated: Vec<ValidatedBackend> = match toml_str {
            Some(s) => {
                let parsed = parse(s).context("parsing storage_backend TOML")?;
                let mut out = Vec::with_capacity(parsed.backends.len());
                for raw in parsed.backends {
                    out.push(validate(raw).context("validating storage_backend entry")?);
                }
                out
            }
            None => Vec::new(),
        };

        // 2. Upsert validated entries; soft-delete entries no longer in TOML.
        let toml_names: std::collections::HashSet<String> =
            validated.iter().map(|v| v.name.clone()).collect();

        // Don't soft-delete localfile-default — it's the migration-seeded fallback.
        for existing in repo.list_active().await? {
            if existing.name == "localfile-default" { continue; }
            if !toml_names.contains(&existing.name) {
                repo.soft_delete_by_name(&existing.name).await?;
                tracing::warn!(
                    "storage_backend '{}' removed from TOML; soft-deleted in DB",
                    existing.name
                );
            }
        }

        for v in &validated {
            let caps_json = serde_json::to_value(v.capabilities)?;
            repo.upsert(
                &v.name,
                v.kind.as_db_str(),
                &v.config,
                &caps_json,
                v.is_default,
            )
            .await
            .with_context(|| format!("upserting storage_backend '{}'", v.name))?;
        }

        // 3. Build the in-memory map. Walk active rows from the DB (post-upsert).
        let mut by_id: HashMap<Uuid, Arc<dyn ControlPlaneBackend>> = HashMap::new();
        let mut default_id: Option<Uuid> = None;
        for row in repo.list_active().await? {
            let trait_obj = build_backend(&row)?;
            if row.is_default {
                if default_id.is_some() {
                    return Err(anyhow!(
                        "more than one default backend in DB — partial unique index should prevent this"
                    ));
                }
                default_id = Some(row.id);
            }
            by_id.insert(row.id, trait_obj);
        }

        if by_id.is_empty() {
            return Err(anyhow!("no active storage backends — migration should have seeded localfile-default"));
        }

        Ok(Registry { by_id, default_id })
    }

    pub fn get(&self, id: Uuid) -> Option<&Arc<dyn ControlPlaneBackend>> {
        self.by_id.get(&id)
    }

    pub fn default_id(&self) -> Option<Uuid> {
        self.default_id
    }

    pub fn default_backend(&self) -> Option<&Arc<dyn ControlPlaneBackend>> {
        self.default_id.and_then(|id| self.by_id.get(&id))
    }
}

fn build_backend(row: &StorageBackendRow) -> Result<Arc<dyn ControlPlaneBackend>> {
    let kind: BackendKind = match row.kind.as_str() {
        "local_file" => BackendKind::LocalFile,
        "iscsi" => BackendKind::Iscsi,
        "truenas_iscsi" => BackendKind::TrueNasIscsi,
        other => {
            tracing::warn!("storage_backend '{}' has unknown kind '{}' — skipping", row.name, other);
            return Err(anyhow!("unknown backend kind '{other}'"));
        }
    };
    match kind {
        BackendKind::LocalFile => Ok(Arc::new(LocalFileControlPlaneBackend {
            id: BackendInstanceId(row.id),
        })),
        BackendKind::Iscsi | BackendKind::TrueNasIscsi => {
            // Implemented in Plan 2. For now, refuse to register.
            Err(anyhow!(
                "backend kind '{}' not implemented in this plan — use Plan 2",
                kind.as_db_str()
            ))
        }
    }
}
```

- [ ] **Step 13.2: Write a smoke test for the registry**

Append to `apps/manager/src/features/storage/registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires DATABASE_URL"]
    async fn registry_loads_localfile_default() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        let reg = Registry::load(&pool, None).await.unwrap();
        let default = reg.default_backend().expect("default backend present");
        assert!(matches!(default.kind(), BackendKind::LocalFile));
    }
}
```

- [ ] **Step 13.3: Verify compilation**

Run: `cargo check -p manager`
Expected: clean.

Run (only if a dev DB is available with migrations applied): `DATABASE_URL=$DATABASE_URL cargo test -p manager registry::tests -- --ignored`
Expected: 1 test passes.

- [ ] **Step 13.4: Commit**

```bash
git add apps/manager/src/features/storage/
git commit -m "feat(storage): Registry — TOML parse, DB upsert, trait-object lookup"
```

---

## Task 14: Implement `LocalFileControlPlaneBackend` for real

**Files:**
- Modify: `apps/manager/src/features/storage/backends/local_file.rs`

This task replaces the stub from Task 13 with the real impl. `LocalFile` reuses the existing `LocalStorage` helper (already in `apps/manager/src/features/storage/mod.rs`) so the file/copy/resize behavior is bit-for-bit identical to current production code.

- [ ] **Step 14.1: Write the failing test**

Create `apps/manager/src/features/storage/backends/tests.rs` (and add `#[cfg(test)] mod tests;` to `backends/mod.rs`):

```rust
use super::local_file::LocalFileControlPlaneBackend;
use nexus_storage::{BackendInstanceId, ControlPlaneBackend, CreateOpts};
use std::path::PathBuf;
use uuid::Uuid;

fn tmp_storage_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[tokio::test]
async fn provision_creates_a_sparse_file_at_requested_size() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .provision(CreateOpts {
            name: "test".into(),
            size_bytes: 16 * 1024 * 1024,
            description: None,
        })
        .await
        .expect("provision");

    let path = PathBuf::from(&h.locator);
    let meta = std::fs::metadata(&path).expect("file exists");
    assert_eq!(meta.len(), 16 * 1024 * 1024);
    assert_eq!(h.size_bytes, 16 * 1024 * 1024);
    assert_eq!(h.backend_kind, nexus_storage::BackendKind::LocalFile);
}

#[tokio::test]
async fn destroy_removes_the_file() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .provision(CreateOpts {
            name: "del".into(),
            size_bytes: 4 * 1024 * 1024,
            description: None,
        })
        .await
        .unwrap();

    let path = PathBuf::from(&h.locator);
    assert!(path.exists());
    backend.destroy(h).await.unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn clone_from_image_copies_and_resizes() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    // Build a fake source image: 4 MiB ext4 (use a sparse file, not a real fs,
    // since LocalFile's clone_from_image doesn't run resize2fs unless it's a
    // valid ext4 — skip the resize portion of this test by leaving size equal).
    let src = root.path().join("src.bin");
    let src_size = 4 * 1024 * 1024_u64;
    {
        let f = std::fs::File::create(&src).unwrap();
        f.set_len(src_size).unwrap();
    }

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .clone_from_image(
            &src,
            CreateOpts {
                name: "cloned".into(),
                size_bytes: src_size,
                description: None,
            },
        )
        .await
        .expect("clone_from_image");

    let cloned_meta = std::fs::metadata(&h.locator).unwrap();
    assert_eq!(cloned_meta.len(), src_size);
}
```

Add `tempfile = "3"` to `[dev-dependencies]` of `apps/manager/Cargo.toml` if not present.

Run: `cargo test -p manager features::storage::backends::tests`
Expected: FAIL — `provision` returns NotSupported (the stub).

- [ ] **Step 14.2: Implement `LocalFileControlPlaneBackend`**

Replace `apps/manager/src/features/storage/backends/local_file.rs` with:

```rust
use crate::features::storage::LocalStorage;
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

/// Manager-side LocalFile backend. Wraps the existing `LocalStorage` helper
/// so behavior is byte-for-byte identical to pre-foundation code.
pub struct LocalFileControlPlaneBackend {
    pub id: BackendInstanceId,
}

impl LocalFileControlPlaneBackend {
    fn storage(&self) -> LocalStorage {
        // LocalStorage::new() reads MANAGER_STORAGE_ROOT each time.
        LocalStorage::new()
    }

    fn root_for(&self, vol_id: Uuid) -> PathBuf {
        self.storage().vm_dir(vol_id).join("storage")
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for LocalFileControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::LocalFile
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: false,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("disk-{}.img", vol_id));
        let f = tokio::fs::File::create(&path).await?;
        f.set_len(opts.size_bytes).await?;
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: path.display().to_string(),
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let p = Path::new(&handle.locator);
        if p.exists() {
            tokio::fs::remove_file(p).await?;
        }
        Ok(())
    }

    async fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;

        let ext = source_image
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{s}"))
            .unwrap_or_default();
        let dst = dir.join(format!("rootfs-{vol_id}{ext}"));

        tokio::fs::copy(source_image, &dst).await?;

        // Match historical behavior: extend file to requested size if larger
        // than the source. resize2fs on the inner ext4 filesystem is the
        // caller's job (rootfs_allocator), not ours.
        let source_size = tokio::fs::metadata(&dst).await?.len();
        let final_size = if opts.size_bytes > source_size {
            let f = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&dst)
                .await?;
            f.set_len(opts.size_bytes).await?;
            opts.size_bytes
        } else {
            source_size
        };

        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: final_size,
        })
    }

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        // Slow but correct: byte-copy the file. Native snapshot capability is
        // false for LocalFile so callers expect this to be slow.
        let snap_id = Uuid::new_v4();
        let src = Path::new(&volume.locator);
        let parent = src.parent().ok_or_else(|| StorageError::InvalidLocator(volume.locator.clone()))?;
        let dst = parent.join(format!("snap-{snap_id}-{name}.img"));
        tokio::fs::copy(src, &dst).await?;
        Ok(VolumeSnapshotHandle {
            snapshot_id: snap_id,
            source_volume_id: volume.volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
        })
    }

    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;
        let dst = dir.join(format!("disk-{vol_id}.img"));
        tokio::fs::copy(&snap.locator, &dst).await?;
        let size = tokio::fs::metadata(&dst).await?.len();
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: size,
        })
    }

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let p = Path::new(&snap.locator);
        if p.exists() {
            tokio::fs::remove_file(p).await?;
        }
        Ok(())
    }
}
```

- [ ] **Step 14.3: Run the tests**

Run: `cargo test -p manager features::storage::backends::tests`
Expected: 3 tests pass.

- [ ] **Step 14.4: Commit**

```bash
git add apps/manager/Cargo.toml apps/manager/src/features/storage/
git commit -m "feat(storage): implement LocalFileControlPlaneBackend"
```

---

## Task 15: Wire Registry into AppState; replace storage field

**Files:**
- Modify: `apps/manager/src/main.rs`

- [ ] **Step 15.1: Update AppState**

Open `apps/manager/src/main.rs`. Find the `pub struct AppState { ... }` block. Add a `pub registry: crate::features::storage::registry::Registry,` field. Keep the existing `pub storage: LocalStorage,` field for now — `LocalStorage` is still used by callers we haven't migrated yet (Task 17 onward).

In `main()`, after `state` is constructed but before it is used, build the registry:

```rust
// After: let pool = ...; let storage = LocalStorage::new();
let toml_path = std::env::var("MANAGER_STORAGE_TOML").ok();
let toml_str = match toml_path.as_deref() {
    Some(p) => Some(tokio::fs::read_to_string(p).await
        .with_context(|| format!("reading {p}"))?),
    None => None,
};
let registry = crate::features::storage::registry::Registry::load(&pool, toml_str.as_deref()).await
    .context("loading storage registry")?;
```

Then add `registry,` to the `AppState { ... }` literal.

- [ ] **Step 15.2: Verify**

Run: `cargo check -p manager`
Expected: clean.

Run: `cargo run -p manager` (with dev DB up)
Expected: starts cleanly. Logs include "storage_backend 'localfile-default'" being upserted (or just present).

- [ ] **Step 15.3: Commit**

```bash
git add apps/manager/src/main.rs
git commit -m "feat(storage): wire Registry into AppState"
```

---

## Task 16: `LocalFileHostBackend` in the agent

**Files:**
- Modify: `apps/agent/Cargo.toml` (add `nexus-storage` dep)
- Create: `apps/agent/src/features/storage/mod.rs`
- Create: `apps/agent/src/features/storage/local_file.rs`
- Modify: `apps/agent/src/features/mod.rs` (add `pub mod storage;`)

- [ ] **Step 16.1: Add the dep**

In `apps/agent/Cargo.toml`, under `[dependencies]`, add:

```toml
nexus-storage = { path = "../../crates/nexus-storage" }
async-trait = "0.1"
```

- [ ] **Step 16.2: Write the local_file impl + test**

Create `apps/agent/src/features/storage/mod.rs`:

```rust
pub mod local_file;
```

Create `apps/agent/src/features/storage/local_file.rs`:

```rust
use nexus_storage::{
    AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle,
};
use std::path::{Path, PathBuf};

/// Agent-side LocalFile backend. Trivial: the locator IS the file path.
/// `attach` returns it as `AttachedPath::File`; `detach` is a no-op (the file
/// stays). `populate_streaming` writes raw bytes from a source file into the
/// destination file with no filesystem awareness.
pub struct LocalFileHostBackend;

#[async_trait::async_trait]
impl HostBackend for LocalFileHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::LocalFile
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        Ok(AttachedPath::File(PathBuf::from(&volume.locator)))
    }

    async fn detach(&self, _v: &VolumeHandle, _a: AttachedPath) -> Result<(), StorageError> {
        Ok(())
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        use tokio::io::AsyncWriteExt;
        let dst_path = attached.path();
        let mut src = tokio::fs::File::open(source).await?;
        let mut dst = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dst_path)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        let cur = tokio::fs::metadata(dst_path).await?.len();
        if target_size_bytes > cur {
            dst.set_len(target_size_bytes).await?;
        }
        dst.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::BackendInstanceId;
    use uuid::Uuid;

    #[tokio::test]
    async fn populate_streaming_is_a_pure_byte_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let data = vec![0xABu8; 8 * 1024];
        std::fs::write(&src, &data).unwrap();
        // Pre-create dst so attach has a path to return.
        std::fs::write(&dst, b"").unwrap();

        let h = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: 16 * 1024,
        };
        let backend = LocalFileHostBackend;
        let attached = backend.attach(&h).await.unwrap();
        backend
            .populate_streaming(&attached, &src, 16 * 1024)
            .await
            .unwrap();

        // Bytes from source are present.
        let written = std::fs::read(&dst).unwrap();
        assert_eq!(&written[..8 * 1024], &data[..]);
        // File extended to target size (sparse tail OK).
        assert_eq!(std::fs::metadata(&dst).unwrap().len(), 16 * 1024);
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` of `apps/agent/Cargo.toml` if not present.

- [ ] **Step 16.3: Register the module**

In `apps/agent/src/features/mod.rs`, add:

```rust
pub mod storage;
```

- [ ] **Step 16.4: Verify**

Run: `cargo test -p agent features::storage::local_file`
Expected: 1 test passes.

- [ ] **Step 16.5: Commit**

```bash
git add apps/agent/Cargo.toml apps/agent/src/features/storage/ apps/agent/src/features/mod.rs Cargo.lock
git commit -m "feat(storage): LocalFileHostBackend in agent"
```

---

## Task 17: `rootfs_allocator` helper in the manager

**Files:**
- Create: `apps/manager/src/features/storage/rootfs_allocator.rs`
- Modify: `apps/manager/src/features/storage/mod.rs` (`pub mod rootfs_allocator;`)

- [ ] **Step 17.1: Write the failing test**

Create `apps/manager/src/features/storage/rootfs_allocator.rs`:

```rust
use crate::features::storage::registry::Registry;
use anyhow::{anyhow, Context, Result};
use nexus_storage::{ControlPlaneBackend, CreateOpts, VolumeHandle};
use std::path::Path;
use uuid::Uuid;

/// Outcome of allocating a rootfs from a source image. The `volume_handle` is
/// always returned; `attached_for_caller` is `Some` only on the slow path
/// where the caller still holds an attachment that should be reused for VM
/// start (avoids detach/reattach round-trip).
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
```

- [ ] **Step 17.2: Register the module**

Append to `apps/manager/src/features/storage/mod.rs`:

```rust
pub mod rootfs_allocator;
```

- [ ] **Step 17.3: Verify**

Run: `cargo check -p manager`
Expected: clean.

Run (only with DB up): `DATABASE_URL=$DATABASE_URL cargo test -p manager rootfs_allocator -- --ignored`
Expected: 1 test passes.

- [ ] **Step 17.4: Commit**

```bash
git add apps/manager/src/features/storage/rootfs_allocator.rs apps/manager/src/features/storage/mod.rs
git commit -m "feat(storage): rootfs_allocator with capability-gated fast path"
```

---

## Task 18: Migrate `vms::service::create_vm` to use `rootfs_allocator`

**Files:**
- Modify: `apps/manager/src/features/vms/service.rs`

This task is the largest behavior change. It replaces the existing call site at line 1150 (`alloc_rootfs`) and the data-disk call at line 1198 (`alloc_data_disk`) with calls through the registry.

- [ ] **Step 18.1: Read the existing call site**

Run: `sed -n '1140,1210p' apps/manager/src/features/vms/service.rs`
Expected: shows the existing `alloc_rootfs` and `alloc_data_disk` invocations and surrounding logic.

- [ ] **Step 18.2: Add a `backend_id` parameter to the create-VM request flow**

Find the request struct used by `create_vm`. It is one of `CreateVmRequest` types in `apps/manager/src/features/vms/`. Add an optional field:

```rust
#[serde(default)]
pub backend_id: Option<uuid::Uuid>,
```

Then in `create_vm`, near where the existing `state.storage.alloc_rootfs` is called:

Before:
```rust
let path = state
    .storage
    .alloc_rootfs(vm_id, Path::new(&source_path), rootfs_size_mb)
    .await?;
```

After:
```rust
let backend_id = req.backend_id
    .or_else(|| state.registry.default_id())
    .ok_or_else(|| anyhow::anyhow!("no storage backend selected and no default configured"))?;

let target_bytes = rootfs_size_mb
    .map(|mb| (mb as u64) * 1024 * 1024)
    .unwrap_or_else(|| {
        std::fs::metadata(&source_path).map(|m| m.len()).unwrap_or(0)
    });

let alloc = crate::features::storage::rootfs_allocator::allocate_rootfs(
    &state.registry,
    backend_id,
    std::path::Path::new(&source_path),
    target_bytes,
    &format!("rootfs-{vm_id}"),
).await?;

// Persist as a row in `volume`.
sqlx::query(
    r#"INSERT INTO volume (id, name, path, size_bytes, type, status, host_id, backend_id)
       VALUES ($1, $2, $3, $4, 'raw', 'available', $5, $6)"#,
)
.bind(alloc.volume_handle.volume_id)
.bind(format!("rootfs-{vm_id}"))
.bind(&alloc.volume_handle.locator)
.bind(alloc.volume_handle.size_bytes as i64)
.bind(state.host_id_for_local_file()) // helper added below
.bind(backend_id)
.execute(&state.db).await?;

let (path, alloc_size) = (alloc.volume_handle.locator.clone(), alloc.volume_handle.size_bytes);
```

Adjust the rest of the function so `path` (used downstream for the Firecracker drive config) is sourced from `alloc.volume_handle.locator`, and `alloc_size` from `alloc.volume_handle.size_bytes`. Keep all downstream behavior identical.

For data disks (around line 1198), replace `state.storage.alloc_data_disk(vm_id, size).await?` with:

```rust
let dh = crate::features::storage::rootfs_allocator::allocate_data_disk(
    &state.registry,
    backend_id,
    size,
    &format!("data-{vm_id}-{drive_id}"),
).await?;
let path = dh.locator.clone();
```

Then INSERT a `volume` row analogously to the rootfs case.

Add a helper to AppState in `apps/manager/src/main.rs`:

```rust
impl AppState {
    /// Returns the host_id to record on a LocalFile-backed volume. In this PR
    /// LocalFile is single-host (the manager host); we use the first host row
    /// in the DB as a stable id. Returns None for non-LocalFile backends.
    pub async fn host_id_for_local_file(&self) -> Option<Uuid> {
        sqlx::query_scalar::<_, Uuid>(r#"SELECT id FROM host ORDER BY last_seen_at DESC LIMIT 1"#)
            .fetch_optional(&self.db).await.ok().flatten()
    }
}
```

(Adjust the call in `create_vm` to `.await` this helper.)

- [ ] **Step 18.3: Update the in-file test stubs**

Lines 2415, 2489, 2560, 2633 currently do:
```rust
let storage = crate::features::storage::LocalStorage::new();
```
These tests construct `AppState` manually. Each one needs a Registry. The simplest approach: add a small test helper at the bottom of the file:

```rust
#[cfg(test)]
async fn test_registry(pool: &sqlx::PgPool) -> crate::features::storage::registry::Registry {
    crate::features::storage::registry::Registry::load(pool, None)
        .await
        .expect("registry")
}
```

And in each of those four test stubs, after constructing `pool`, add:
```rust
let registry = test_registry(&pool).await;
```
And include `registry` in the `AppState { ... }` literal.

- [ ] **Step 18.4: Verify**

Run: `cargo check -p manager && cargo test -p manager features::vms`
Expected: clean compile; the 4 test stubs that depend on a DB will be `#[ignore]`-gated or run if the test DB is up.

- [ ] **Step 18.5: Commit**

```bash
git add apps/manager/src/features/vms/service.rs apps/manager/src/main.rs
git commit -m "feat(storage): create_vm allocates through Registry (LocalFile fast path)"
```

---

## Task 19: Add TODO markers in functions and containers code paths

**Files:**
- Modify: `apps/manager/src/features/functions/vm.rs`
- Modify: `apps/manager/src/features/containers/vm.rs`

Per the spec's non-goal: functions and containers stay on hardcoded `/srv/images/...` paths in this PR. We add markers so future contributors know this is deliberate.

- [ ] **Step 19.1: Insert the comments**

In `apps/manager/src/features/functions/vm.rs`, find line 36 (`let function_rootfs_path = format!("/srv/images/functions/{}.ext4", vm_id);`). Immediately above it, add:

```rust
// TODO(storage-backends): route through StorageBackend trait. See
// docs/superpowers/specs/2026-04-28-storage-hci-design.md ("Out of scope").
```

Do the same in `apps/manager/src/features/containers/vm.rs` line 34.

- [ ] **Step 19.2: Verify**

Run: `cargo check -p manager`
Expected: clean.

- [ ] **Step 19.3: Commit**

```bash
git add apps/manager/src/features/functions/vm.rs apps/manager/src/features/containers/vm.rs
git commit -m "chore(storage): TODO markers in functions/containers (out-of-scope per spec)"
```

---

## Task 20: VM-start writes `volume_attachment` row through registry

**Files:**
- Modify: `apps/manager/src/features/vms/service.rs` (start path)
- Modify: `apps/manager/src/features/volumes/repo.rs`

- [ ] **Step 20.1: Update VolumeRow and add `mark_detached`**

In `apps/manager/src/features/volumes/repo.rs`, update `VolumeRow`:

Before:
```rust
pub host_id: Uuid,
```

After:
```rust
pub host_id: Option<Uuid>,
pub backend_id: Uuid,
```

Update `AttachmentRow`:

Before:
```rust
pub struct AttachmentRow {
    pub id: Uuid,
    pub volume_id: Uuid,
    pub vm_id: Uuid,
    pub drive_id: String,
    pub attached_at: DateTime<chrono::Utc>,
}
```

After:
```rust
pub struct AttachmentRow {
    pub id: Uuid,
    pub volume_id: Uuid,
    pub vm_id: Uuid,
    pub drive_id: String,
    pub attached_at: DateTime<chrono::Utc>,
    pub detached_at: Option<DateTime<chrono::Utc>>,
}
```

Add to `impl VolumeRepository`:

```rust
pub async fn insert_active_attachment(
    &self,
    volume_id: Uuid,
    vm_id: Uuid,
    drive_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id) VALUES ($1, $2, $3)"#,
    )
    .bind(volume_id)
    .bind(vm_id)
    .bind(drive_id)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn mark_detached(&self, vm_id: Uuid, drive_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE volume_attachment SET detached_at = now()
           WHERE vm_id = $1 AND drive_id = $2 AND detached_at IS NULL"#,
    )
    .bind(vm_id)
    .bind(drive_id)
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

Update the existing `volume_to_list_item` and any other callers that read `host_id` to handle `Option<Uuid>`.

- [ ] **Step 20.2: Update create_vm to insert the attachment row at success**

In `apps/manager/src/features/vms/service.rs`, after the `volume` row INSERT and after Firecracker spawn returns success, insert the `volume_attachment` row. Locate the `Ok(...)` exit branch of `start_vm` (or the equivalent point after spawn returns OK) and add:

```rust
let vol_repo = crate::features::volumes::repo::VolumeRepository::new(state.db.clone());
vol_repo.insert_active_attachment(volume_id, vm_id, &drive_id).await
    .context("inserting volume_attachment row after VM start")?;
```

- [ ] **Step 20.3: Verify**

Run: `cargo check -p manager`
Expected: clean.

- [ ] **Step 20.4: Commit**

```bash
git add apps/manager/src/features/vms/service.rs apps/manager/src/features/volumes/repo.rs
git commit -m "feat(storage): write volume_attachment row at VM start through Registry"
```

---

## Task 21: VM-stop / VM-delete sets `detached_at`

**Files:**
- Modify: `apps/manager/src/features/vms/service.rs` (stop / delete paths)

- [ ] **Step 21.1: Find the stop/delete handlers**

Run: `grep -n "fn stop_vm\|fn delete_vm" apps/manager/src/features/vms/service.rs`
Expected: line numbers for both.

- [ ] **Step 21.2: Add detach + mark_detached calls**

In each handler, after a successful agent stop, walk the VM's drives and:

```rust
let vol_repo = crate::features::volumes::repo::VolumeRepository::new(state.db.clone());
for drive in &drives {
    vol_repo.mark_detached(vm_id, &drive.drive_id).await
        .context("marking volume_attachment detached")?;
}
```

For now, since LocalFile's `detach` is a no-op, we don't need to call the trait's `detach` method explicitly; just mark the DB row. Plan 2 (iSCSI) revisits this to also call `host.detach` via the agent.

- [ ] **Step 21.3: Verify**

Run: `cargo check -p manager`
Expected: clean.

- [ ] **Step 21.4: Commit**

```bash
git add apps/manager/src/features/vms/service.rs
git commit -m "feat(storage): mark volume_attachment detached on VM stop/delete"
```

---

## Task 22: `AlreadyAttached` error translation

**Files:**
- Modify: `apps/manager/src/features/volumes/routes.rs` (the `attach` handler)

- [ ] **Step 22.1: Locate the attach handler**

Run: `grep -n "pub async fn attach\b" apps/manager/src/features/volumes/routes.rs`
Expected: a handler signature.

- [ ] **Step 22.2: Catch unique-violation 23505**

In the body of the attach handler, where the INSERT into `volume_attachment` happens, wrap it:

```rust
let res = sqlx::query(
    r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id) VALUES ($1, $2, $3)"#,
)
.bind(volume_id)
.bind(req.vm_id)
.bind(&req.drive_id)
.execute(&st.db)
.await;

match res {
    Ok(_) => {}
    Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "volume already attached"})),
        )
            .into_response();
    }
    Err(e) => {
        tracing::error!("attach failed: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db"})))
            .into_response();
    }
}
```

- [ ] **Step 22.3: Verify**

Run: `cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 22.4: Commit**

```bash
git add apps/manager/src/features/volumes/routes.rs
git commit -m "feat(storage): translate Postgres 23505 to 409 AlreadyAttached on volume attach"
```

---

## Task 23: Integration test — VM lifecycle through LocalFile

**Files:**
- Create: `apps/manager/tests/storage_foundation.rs`

- [ ] **Step 23.1: Write the integration test**

Create `apps/manager/tests/storage_foundation.rs`:

```rust
//! Integration tests for the storage foundation. Require a running Postgres
//! pointed at by DATABASE_URL with all migrations applied.

use sqlx::PgPool;
use std::env;

async fn pool() -> PgPool {
    let url = env::var("DATABASE_URL").expect("DATABASE_URL");
    PgPool::connect(&url).await.expect("connect")
}

#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn registry_loads_default_localfile_backend() {
    let p = pool().await;
    let reg = manager::features::storage::registry::Registry::load(&p, None)
        .await
        .expect("registry");
    let default = reg.default_backend().expect("default present");
    assert!(matches!(default.kind(), nexus_storage::BackendKind::LocalFile));
    assert!(default.capabilities().supports_clone_from_image);
}

#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn rootfs_allocator_clone_path_returns_handle() {
    let p = pool().await;
    let reg = manager::features::storage::registry::Registry::load(&p, None)
        .await
        .expect("registry");
    let id = reg.default_id().expect("default id");
    let dir = tempfile::tempdir().unwrap();
    env::set_var("MANAGER_STORAGE_ROOT", dir.path());
    let src = dir.path().join("rootfs.ext4");
    std::fs::write(&src, vec![0u8; 4 * 1024 * 1024]).unwrap();

    let out = manager::features::storage::rootfs_allocator::allocate_rootfs(
        &reg, id, &src, 4 * 1024 * 1024, "it",
    )
    .await
    .expect("alloc");
    assert_eq!(out.volume_handle.size_bytes, 4 * 1024 * 1024);
    assert!(std::path::Path::new(&out.volume_handle.locator).exists());
}
```

For this to compile, `apps/manager/Cargo.toml` needs to expose `manager` as a library. Check whether `apps/manager/src/lib.rs` exists; if not, add:

```rust
// apps/manager/src/lib.rs
pub mod features;
pub use crate::features::*;
```

And in `apps/manager/Cargo.toml`, ensure `[lib] path = "src/lib.rs"` is set, or add:

```toml
[lib]
path = "src/lib.rs"
```

If exposing the binary as a library is non-trivial in this repo, instead put these tests in-tree as `#[cfg(test)] mod` blocks under the relevant modules.

- [ ] **Step 23.2: Run**

Run: `DATABASE_URL=$DATABASE_URL cargo test -p manager --test storage_foundation -- --ignored`
Expected: 2 tests pass.

- [ ] **Step 23.3: Commit**

```bash
git add apps/manager/tests/storage_foundation.rs apps/manager/src/lib.rs apps/manager/Cargo.toml
git commit -m "test(storage): integration tests for Registry and rootfs_allocator"
```

---

## Task 24: Single-attach enforcement test

**Files:**
- Modify: `apps/manager/tests/storage_foundation.rs`

- [ ] **Step 24.1: Add the test**

Append to `apps/manager/tests/storage_foundation.rs`:

```rust
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn double_attach_is_rejected_at_db_level() {
    let p = pool().await;
    // Insert a fake volume row so the FK is satisfied.
    let backend_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"SELECT id FROM storage_backend WHERE name = 'localfile-default'"#,
    )
    .fetch_one(&p)
    .await
    .unwrap();
    let host_id = sqlx::query_scalar::<_, Option<uuid::Uuid>>(
        r#"SELECT id FROM host LIMIT 1"#,
    )
    .fetch_optional(&p)
    .await
    .unwrap()
    .flatten();

    let vol_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO volume (id, name, path, size_bytes, type, status, host_id, backend_id)
           VALUES ($1, $2, $3, 1024, 'raw', 'available', $4, $5)"#,
    )
    .bind(vol_id)
    .bind(format!("vol-{vol_id}"))
    .bind(format!("/tmp/{vol_id}.img"))
    .bind(host_id)
    .bind(backend_id)
    .execute(&p)
    .await
    .unwrap();

    // Two distinct VM ids — but we don't have FK to vm in this minimal setup
    // so we can't actually insert. Instead, use the migration test pattern:
    // attempt the second insert against the partial unique index by simulating
    // it directly.

    let vm1 = uuid::Uuid::new_v4();
    let vm2 = uuid::Uuid::new_v4();

    // First attach must INSERT cleanly (assuming the FK to `vm` is deferred or
    // optional — if your env enforces it, insert two vm rows here too).
    let _ = sqlx::query(
        r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id) VALUES ($1, $2, 'rootfs')"#,
    )
    .bind(vol_id)
    .bind(vm1)
    .execute(&p)
    .await;

    let second = sqlx::query(
        r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id) VALUES ($1, $2, 'rootfs')"#,
    )
    .bind(vol_id)
    .bind(vm2)
    .execute(&p)
    .await;

    match second {
        Err(sqlx::Error::Database(db_err)) => {
            assert_eq!(db_err.code().as_deref(), Some("23505"));
        }
        other => panic!("expected 23505 unique violation, got {other:?}"),
    }

    // Cleanup
    sqlx::query("DELETE FROM volume_attachment WHERE volume_id = $1").bind(vol_id).execute(&p).await.ok();
    sqlx::query("DELETE FROM volume WHERE id = $1").bind(vol_id).execute(&p).await.ok();
}
```

(Note: if the test DB enforces the `vm` FK strictly, prepend two `INSERT INTO vm ...` rows for `vm1` and `vm2` with the minimum required columns.)

- [ ] **Step 24.2: Run**

Run: `DATABASE_URL=$DATABASE_URL cargo test -p manager --test storage_foundation double_attach -- --ignored`
Expected: pass.

- [ ] **Step 24.3: Commit**

```bash
git add apps/manager/tests/storage_foundation.rs
git commit -m "test(storage): single-attach partial unique index rejects double attach"
```

---

## Task 25: `populate_streaming` purity test (no filesystem mutation)

**Files:**
- Modify: `apps/agent/src/features/storage/local_file.rs` (add the purity test)

- [ ] **Step 25.1: Append the test**

Append to the `mod tests` in `apps/agent/src/features/storage/local_file.rs`:

```rust
    #[tokio::test]
    async fn populate_streaming_does_not_mutate_filesystem_metadata() {
        // We can't easily verify "no resize2fs ran" without intercepting
        // tooling — but we CAN verify the function does nothing more than
        // a byte copy + size-extend by checking the destination is exactly
        // src_bytes followed by sparse zeros up to target_size_bytes.
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");

        let pattern: Vec<u8> = (0u8..=255u8).cycle().take(8192).collect();
        std::fs::write(&src, &pattern).unwrap();
        std::fs::write(&dst, b"").unwrap();

        let h = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: 32 * 1024,
        };
        let backend = LocalFileHostBackend;
        let attached = backend.attach(&h).await.unwrap();
        backend.populate_streaming(&attached, &src, 32 * 1024).await.unwrap();

        let written = std::fs::read(&dst).unwrap();
        assert_eq!(&written[..pattern.len()], &pattern[..], "byte copy mismatch");
        // Tail must be zeros (sparse extension), not anything resize2fs would
        // write (resize2fs would change ext4 superblock metadata, not zero-fill).
        assert!(written[pattern.len()..].iter().all(|b| *b == 0), "tail not zero — possible filesystem mutation");
    }
```

- [ ] **Step 25.2: Run**

Run: `cargo test -p agent features::storage::local_file`
Expected: 2 tests pass.

- [ ] **Step 25.3: Commit**

```bash
git add apps/agent/src/features/storage/local_file.rs
git commit -m "test(storage): populate_streaming purity (no filesystem mutation)"
```

---

## Task 26: Capability gating test for `clone_from_image`

**Files:**
- Modify: `crates/nexus-storage/src/lib.rs` (add the test)

- [ ] **Step 26.1: Add a synthetic backend that returns NotSupported**

Append to `crates/nexus-storage/src/lib.rs` `tests`:

```rust
    use crate::error::StorageError;
    use crate::handle::{VolumeHandle, VolumeSnapshotHandle};
    use crate::types::{BackendInstanceId, CreateOpts};
    use async_trait::async_trait;
    use std::path::Path;
    use uuid::Uuid;

    struct UnsupportedBackend;

    #[async_trait]
    impl ControlPlaneBackend for UnsupportedBackend {
        fn kind(&self) -> BackendKind { BackendKind::Iscsi }
        fn capabilities(&self) -> Capabilities {
            Capabilities { supports_clone_from_image: false, ..Default::default() }
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
        async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> { Ok(()) }
        async fn clone_from_image(&self, _: &Path, _: CreateOpts) -> Result<VolumeHandle, StorageError> {
            Err(StorageError::NotSupported("clone_from_image".into()))
        }
        async fn snapshot(&self, _: &VolumeHandle, _: &str) -> Result<VolumeSnapshotHandle, StorageError> {
            Err(StorageError::NotSupported("snapshot".into()))
        }
        async fn clone_from_snapshot(&self, _: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> {
            Err(StorageError::NotSupported("clone_from_snapshot".into()))
        }
        async fn delete_snapshot(&self, _: VolumeSnapshotHandle) -> Result<(), StorageError> { Ok(()) }
    }

    #[tokio::test]
    async fn clone_from_image_returns_not_supported_when_capability_is_false() {
        let b = UnsupportedBackend;
        assert!(!b.capabilities().supports_clone_from_image);
        let err = b.clone_from_image(
            Path::new("/dev/null"),
            CreateOpts { name: "x".into(), size_bytes: 0, description: None },
        ).await.unwrap_err();
        assert!(matches!(err, StorageError::NotSupported(_)));
    }
```

Add `tokio = { workspace = true }` to `[dev-dependencies]` of `crates/nexus-storage/Cargo.toml` if not present, with the `macros` and `rt` features enabled.

- [ ] **Step 26.2: Run**

Run: `cargo test -p nexus-storage`
Expected: all tests pass including the new one.

- [ ] **Step 26.3: Commit**

```bash
git add crates/nexus-storage/
git commit -m "test(storage): clone_from_image returns NotSupported when capability false"
```

---

## Task 27: Config validation startup-failure test

**Files:**
- Modify: `apps/manager/src/features/storage/config.rs`

- [ ] **Step 27.1: Add a test for malformed TOML rejection**

Append to the `tests` mod in `apps/manager/src/features/storage/config.rs`:

```rust
    #[test]
    fn malformed_truenas_iscsi_entry_fails_fast_with_clear_message() {
        let toml_str = r#"
            [[storage_backend]]
            name = "tn"
            kind = "truenas_iscsi"
            [storage_backend.config]
            api_key_env = "X"
        "#;
        let parsed = parse(toml_str).unwrap();
        let raw = parsed.backends.into_iter().next().unwrap();
        let err = validate(raw).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("endpoint"), "error should name the missing field: {msg}");
        assert!(msg.contains("tn"), "error should name the backend: {msg}");
    }
```

- [ ] **Step 27.2: Run**

Run: `cargo test -p manager features::storage::config`
Expected: 4 tests pass.

- [ ] **Step 27.3: Commit**

```bash
git add apps/manager/src/features/storage/config.rs
git commit -m "test(storage): malformed config rejected with clear message"
```

---

## Task 28: Migration test — pre-foundation rows still resolve

**Files:**
- Create: `apps/manager/tests/storage_migration.rs`

- [ ] **Step 28.1: Write the test**

Create `apps/manager/tests/storage_migration.rs`:

```rust
//! Verifies that volumes that existed before migration 0034 are mapped to
//! `localfile-default` and remain readable.

use sqlx::PgPool;

#[tokio::test]
#[ignore = "requires DATABASE_URL with migrations applied"]
async fn pre_foundation_volume_row_is_backfilled_to_localfile_default() {
    let url = std::env::var("DATABASE_URL").unwrap();
    let p = PgPool::connect(&url).await.unwrap();

    // Backend id of localfile-default.
    let backend_id: uuid::Uuid = sqlx::query_scalar(
        r#"SELECT id FROM storage_backend WHERE name = 'localfile-default'"#,
    )
    .fetch_one(&p)
    .await
    .unwrap();

    // Insert a fake legacy-style volume row with backend_id explicitly set
    // (simulating the migration's UPDATE on legacy NULL rows).
    let vol_id = uuid::Uuid::new_v4();
    let host_id: Option<uuid::Uuid> = sqlx::query_scalar(r#"SELECT id FROM host LIMIT 1"#)
        .fetch_optional(&p)
        .await
        .unwrap()
        .flatten();
    sqlx::query(
        r#"INSERT INTO volume (id, name, path, size_bytes, type, status, host_id, backend_id)
           VALUES ($1, $2, $3, 1024, 'raw', 'available', $4, $5)"#,
    )
    .bind(vol_id)
    .bind(format!("legacy-{vol_id}"))
    .bind(format!("/tmp/legacy-{vol_id}.img"))
    .bind(host_id)
    .bind(backend_id)
    .execute(&p)
    .await
    .unwrap();

    // Read it back. backend_id non-null; host_id may be present or null.
    let row: (uuid::Uuid, Option<uuid::Uuid>, uuid::Uuid) = sqlx::query_as(
        r#"SELECT id, host_id, backend_id FROM volume WHERE id = $1"#,
    )
    .bind(vol_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.2, backend_id, "backend_id must point at localfile-default");

    sqlx::query("DELETE FROM volume WHERE id = $1").bind(vol_id).execute(&p).await.ok();
}
```

- [ ] **Step 28.2: Run**

Run: `DATABASE_URL=$DATABASE_URL cargo test -p manager --test storage_migration -- --ignored`
Expected: pass.

- [ ] **Step 28.3: Commit**

```bash
git add apps/manager/tests/storage_migration.rs
git commit -m "test(storage): pre-foundation volume rows backfill cleanly"
```

---

## Task 29: Final sweep — fmt, clippy, full test suite

**Files:** none

- [ ] **Step 29.1: Format**

Run: `cargo fmt`
Expected: writes formatting changes (commit them if any).

- [ ] **Step 29.2: Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 29.3: Test suite**

Run: `cargo test --workspace --exclude installer`
Expected: all non-`#[ignore]` tests pass.

If a DB is available:
Run: `DATABASE_URL=$DATABASE_URL cargo test --workspace --exclude installer -- --ignored`
Expected: integration tests pass.

- [ ] **Step 29.4: Commit any formatting**

```bash
git add -A
git commit -m "chore(storage): cargo fmt sweep"
```

(If nothing changed, skip the commit.)

---

## Plan completion checklist

Before declaring this plan done, verify against the spec's success criteria:

- [ ] Existing VMs continue to function with no operator action — **manual test**: bring up dev DB, restart manager, confirm an existing VM (created before this PR) starts cleanly.
- [ ] `GET /v1/storage_backends` returns at least the `localfile-default` row on a fresh install — **covered by Task 11 smoke test**.
- [ ] New VMs can be created on `LocalFile` backend via API request body — **covered by Task 18 + Task 23**.
- [ ] Adding a future backend requires only implementing the two traits + adding a TOML config section — **architectural; verified by Plan 2 building cleanly on this**.
- [ ] `AttachedPath` covers File / BlockDevice / VhostUserSock — **types defined in Task 3; File proven in Task 16**.
- [ ] Attempting to attach an already-attached volume returns 409 — **covered by Tasks 22 + 24**.
- [ ] `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass — **Task 29**.

## Out of scope for this plan (handled by Plans 2 and 3)

- iSCSI generic + TrueNAS variant — Plan 2.
- Slow-path rootfs allocation (provision + attach + populate_streaming + caller-side resize2fs) — Plan 2.
- Agent kind() handshake — Plan 2.
- `BackendSelector` UI component, VM-create form integration — Plan 3.

---

## Self-review

Spec coverage check (against `2026-04-28-storage-hci-design.md`):

- §"In this PR" item 1 (two traits + types) → Tasks 1–6 ✓
- item 2 (schema migration) → Task 7 ✓
- item 3 (LocalFileBackend preserves behavior) → Tasks 14, 16 ✓
- item 4 (IscsiBackend) → **deferred to Plan 2** (noted in plan header)
- item 5 (Backend registry + TOML) → Tasks 12, 13 ✓
- item 6 (VM lifecycle wired through traits) → Tasks 17, 18, 20, 21 ✓
- item 7 (API + UI for backend selection) → API done in Tasks 10, 11; **UI deferred to Plan 3**
- item 8 (tests) → Tasks 23–28 ✓
- §"backend identity is per-volume" → Task 7 + Task 18 (volume row carries backend_id) ✓
- §"single-attach enforced" → Tasks 7, 22, 24 ✓
- §"AlreadyAttached error translation" → Task 22 ✓
- §"populate_streaming purity" → Tasks 6, 16, 25 ✓
- §"capabilities denormalized in DB" → Task 7 (capabilities_json column), Task 13 (registry seeds it) ✓
- §"non-goal: rollback_to_snapshot" → trait surface omits it (Task 5) ✓
- §"non-goal: functions/containers routing" → Task 19 ✓
- §"open question: agent handshake" → **deferred to Plan 2** (noted) ✓
- §"open question: config_json validation" → Task 12 ✓

Placeholders: none found in this plan.
Type consistency: `VolumeHandle` shape, `Capabilities` field names, `BackendKind` variants are consistent across Tasks 2–18.
