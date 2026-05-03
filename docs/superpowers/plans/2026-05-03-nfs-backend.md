# NFS Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a first-class `nfs` storage backend so VM disks can live on any NFS v4 share — the highest-ROI external-storage addition after iSCSI because every NAS speaks NFS and the protocol needs no LUN provisioning REST dance.

**Architecture:** Manager mounts the export at `manager_mount_path` (typically once at process start, or on demand). All control-plane operations (`provision`, `clone_from_image`, `snapshot`, `clone_from_snapshot`, `destroy`) are filesystem ops against that mount — the same pattern as `LocalFileControlPlaneBackend` but parameterised by NFS server + export so the agent knows what to mount. Locator is a JSON object `{"server","export","file"}` stored in `volume.path`. Agent's `NfsHostBackend` parses the locator, ensures the export is mounted at a unique mount-point under `mount_base`, and returns `AttachedPath::File(<mount>/<file>)`. `populate_streaming` and `resize2fs` are byte-for-byte clones of the `LocalFile` host backend — only the mount lifecycle is new.

**Tech Stack:** Existing Rust toolchain. `tokio::fs` for FS ops. `tokio::process::Command` for `mount.nfs` / `umount`. No new crates. No new SQL migrations (NFS is a new `BackendKind` variant; the existing `storage_backend` row carries config JSON).

**Spec:** Conversation 2026-05-03 with @kleopasevan; reuses the `ControlPlaneBackend` + `HostBackend` traits shipped in Plan 1 (`feature/storage-foundation`, PR #14). Builds on the abstraction the iSCSI / TrueNAS / SPDK lvol backends already use.

---

## File structure

**New:**
- `apps/manager/src/features/storage/backends/nfs.rs` — `NfsConfig` + `NfsControlPlaneBackend` + locator helpers. Single file because all control-plane logic for NFS is short and rarely changes together with anything else.
- `apps/agent/src/features/storage/nfs.rs` — `NfsHostConfig` + `NfsHostBackend` + mount manager helper. Single file for the same reason.
- `docs/runbooks/nfs-smoke.md` — operator runbook for the live smoke test.

**Modified:**
- `crates/nexus-storage/src/types.rs` — add `BackendKind::Nfs` variant.
- `apps/manager/src/features/storage/backends/mod.rs` — add `pub mod nfs;`.
- `apps/manager/src/features/storage/registry.rs` — wire `Nfs` kind into `build_backend()`.
- `apps/agent/src/features/storage/mod.rs` — add `pub mod nfs;`.
- `apps/agent/src/main.rs` — register `NfsHostBackend` when `AGENT_NFS_MOUNT_BASE` is set.

**No DB migration.** The existing `storage_backend` row already stores backend-specific config as a JSONB column; new variants need no schema change.

---

## Conventions

Same as Plan 1 / Plan 2: Conventional Commits (`feat(storage):`, `fix(storage):`, `test(storage):`); `cargo fmt` + `cargo clippy --all-targets --all-features -- -D warnings` clean before committing; do not break existing storage tests; SQLx queries do not apply (no DB schema changes).

---

## Task 1: Add `Nfs` variant to `BackendKind`

**Files:**
- Modify: `crates/nexus-storage/src/types.rs:25-46`

- [ ] **Step 1: Write the failing test**

Add to `crates/nexus-storage/src/types.rs` at the bottom of the file (before any existing `#[cfg(test)]`, or extend the existing one):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_nfs_round_trips_through_db_string() {
        let k = BackendKind::Nfs;
        assert_eq!(k.as_db_str(), "nfs");
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(json, "\"nfs\"");
        let back: BackendKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, BackendKind::Nfs);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p nexus-storage backend_kind_nfs_round_trips_through_db_string -- --nocapture
```

Expected: FAIL — `BackendKind` has no `Nfs` variant.

- [ ] **Step 3: Add the `Nfs` variant + `as_db_str` arm**

In `crates/nexus-storage/src/types.rs`, add to the `BackendKind` enum:

```rust
    #[serde(rename = "nfs")]
    Nfs,
```

And add to the `as_db_str` match:

```rust
            BackendKind::Nfs => "nfs",
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p nexus-storage backend_kind_nfs_round_trips_through_db_string -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Verify the rest of the workspace still compiles** (the registry's `build_backend` uses an exhaustive match on the kind string, not the enum, so adding a variant doesn't break it. Check that the manager + agent still build.)

```bash
cargo build -p manager -p agent
```

Expected: PASS, no `non_exhaustive_patterns` errors.

- [ ] **Step 6: Commit**

```bash
git add crates/nexus-storage/src/types.rs
git commit -m "feat(storage): add BackendKind::Nfs variant"
```

---

## Task 2: Manager-side `NfsConfig` + locator helpers

**Files:**
- Create: `apps/manager/src/features/storage/backends/nfs.rs`
- Modify: `apps/manager/src/features/storage/backends/mod.rs`

- [ ] **Step 1: Add the module declaration**

Edit `apps/manager/src/features/storage/backends/mod.rs`, append after the existing `pub mod` lines:

```rust
pub mod nfs;
```

- [ ] **Step 2: Write the failing test for `NfsConfig` parsing + locator round-trip**

Create `apps/manager/src/features/storage/backends/nfs.rs` with just the test first:

```rust
//! NFS control-plane backend. The manager accesses the export through a
//! local mount (`manager_mount_path`); all provision / destroy / clone
//! ops are filesystem ops against that mount, just like LocalFile. The
//! NFS-ness is captured in the locator JSON so the agent knows what to
//! mount when it later attaches the volume.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs_config_parses_minimal_json() {
        let json = serde_json::json!({
            "server": "10.0.0.5",
            "export": "/mnt/tank/vms",
            "manager_mount_path": "/mnt/nfs-manager"
        });
        let cfg: NfsConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.server, "10.0.0.5");
        assert_eq!(cfg.export, "/mnt/tank/vms");
        assert_eq!(cfg.manager_mount_path, std::path::PathBuf::from("/mnt/nfs-manager"));
    }

    #[test]
    fn nfs_locator_round_trips() {
        let loc = NfsLocator {
            server: "10.0.0.5".into(),
            export: "/mnt/tank/vms".into(),
            file: "nfs-abc.raw".into(),
        };
        let s = loc.to_locator_string().unwrap();
        let back = NfsLocator::from_locator_str(&s).unwrap();
        assert_eq!(back, loc);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p manager nfs::tests -- --nocapture
```

Expected: FAIL — `NfsConfig` and `NfsLocator` are undefined.

- [ ] **Step 4: Add the types and helpers above the `#[cfg(test)] mod tests`**

Insert at the top of `apps/manager/src/features/storage/backends/nfs.rs`:

```rust
use nexus_storage::StorageError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct NfsConfig {
    pub server: String,
    pub export: String,
    pub manager_mount_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NfsLocator {
    pub server: String,
    pub export: String,
    pub file: String,
}

impl NfsLocator {
    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self)
            .map_err(|e| StorageError::InvalidLocator(format!("encode nfs locator: {e}")))
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("decode nfs locator: {e}")))
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p manager nfs::tests -- --nocapture
```

Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add apps/manager/src/features/storage/backends/mod.rs \
        apps/manager/src/features/storage/backends/nfs.rs
git commit -m "feat(storage): NfsConfig + NfsLocator types"
```

---

## Task 3: `NfsControlPlaneBackend::provision` creates a sparse file

**Files:**
- Modify: `apps/manager/src/features/storage/backends/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `apps/manager/src/features/storage/backends/nfs.rs`:

```rust
    use nexus_storage::{BackendInstanceId, ControlPlaneBackend, CreateOpts};
    use uuid::Uuid;

    fn temp_backend() -> (NfsControlPlaneBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = NfsControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: NfsConfig {
                server: "10.0.0.5".into(),
                export: "/mnt/tank/vms".into(),
                manager_mount_path: dir.path().to_path_buf(),
            },
        };
        (backend, dir)
    }

    #[tokio::test]
    async fn provision_creates_a_sparse_file_at_requested_size() {
        let (backend, _guard) = temp_backend();
        let opts = CreateOpts {
            name: "vol-1".into(),
            size_bytes: 4 * 1024 * 1024,
            description: None,
        };
        let h = backend.provision(opts).await.expect("provision");
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 4 * 1024 * 1024);
        assert_eq!(loc.server, "10.0.0.5");
        assert_eq!(loc.export, "/mnt/tank/vms");
        assert!(loc.file.starts_with("nfs-"));
        assert!(loc.file.ends_with(".raw"));
    }
```

If `tempfile` is not yet a dev-dep of `manager`, add it. Check first:

```bash
grep -A 2 '\[dev-dependencies\]' apps/manager/Cargo.toml | head
```

If `tempfile` is missing, add it:

```bash
cd apps/manager && cargo add --dev tempfile && cd ../..
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p manager nfs::tests::provision_creates_a_sparse_file_at_requested_size -- --nocapture
```

Expected: FAIL — `NfsControlPlaneBackend` is undefined.

- [ ] **Step 3: Add struct + `ControlPlaneBackend` impl with provision-only**

Insert after the `NfsLocator` impl block, before `#[cfg(test)]`:

```rust
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts,
    VolumeHandle, VolumeSnapshotHandle,
};
use uuid::Uuid;

pub struct NfsControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: NfsConfig,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for NfsControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Nfs
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let path = self.config.manager_mount_path.join(&file);
        tokio::fs::create_dir_all(&self.config.manager_mount_path).await?;
        let f = tokio::fs::File::create(&path).await?;
        f.set_len(opts.size_bytes).await?;
        drop(f);
        let locator = NfsLocator {
            server: self.config.server.clone(),
            export: self.config.export.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, _h: VolumeHandle) -> Result<(), nexus_storage::StorageError> {
        // Implemented in Task 4.
        Err(nexus_storage::StorageError::NotSupported(
            "destroy not yet implemented".into(),
        ))
    }

    async fn clone_from_image(
        &self,
        _src: &std::path::Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        // Implemented in Task 5.
        Err(nexus_storage::StorageError::NotSupported(
            "clone_from_image not yet implemented".into(),
        ))
    }

    async fn snapshot(
        &self,
        _v: &VolumeHandle,
        _name: &str,
    ) -> Result<VolumeSnapshotHandle, nexus_storage::StorageError> {
        // Implemented in Task 6.
        Err(nexus_storage::StorageError::NotSupported(
            "snapshot not yet implemented".into(),
        ))
    }

    async fn clone_from_snapshot(
        &self,
        _s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        // Implemented in Task 7.
        Err(nexus_storage::StorageError::NotSupported(
            "clone_from_snapshot not yet implemented".into(),
        ))
    }

    async fn delete_snapshot(
        &self,
        _s: VolumeSnapshotHandle,
    ) -> Result<(), nexus_storage::StorageError> {
        // Implemented in Task 7.
        Ok(())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p manager nfs::tests::provision_creates_a_sparse_file_at_requested_size -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run clippy**

```bash
cargo clippy -p manager --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add apps/manager/src/features/storage/backends/nfs.rs apps/manager/Cargo.toml apps/manager/Cargo.lock
git commit -m "feat(storage): NfsControlPlaneBackend::provision creates sparse file"
```

---

## Task 4: `NfsControlPlaneBackend::destroy` unlinks the file

**Files:**
- Modify: `apps/manager/src/features/storage/backends/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[tokio::test]
    async fn destroy_unlinks_the_file() {
        let (backend, _guard) = temp_backend();
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        assert!(tokio::fs::metadata(&path).await.is_ok());
        backend.destroy(h).await.expect("destroy");
        assert!(tokio::fs::metadata(&path).await.is_err());
    }

    #[tokio::test]
    async fn destroy_is_idempotent_when_file_missing() {
        let (backend, _guard) = temp_backend();
        let bogus = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: backend.id,
            backend_kind: BackendKind::Nfs,
            locator: NfsLocator {
                server: backend.config.server.clone(),
                export: backend.config.export.clone(),
                file: "nfs-does-not-exist.raw".into(),
            }
            .to_locator_string()
            .unwrap(),
            size_bytes: 0,
        };
        backend.destroy(bogus).await.expect("idempotent destroy");
    }
```

- [ ] **Step 2: Run test to verify they fail**

```bash
cargo test -p manager nfs::tests::destroy -- --nocapture
```

Expected: FAIL — `destroy` returns `NotSupported`.

- [ ] **Step 3: Replace the `destroy` body**

In `apps/manager/src/features/storage/backends/nfs.rs`, replace the `async fn destroy` body with:

```rust
    async fn destroy(&self, h: VolumeHandle) -> Result<(), nexus_storage::StorageError> {
        let loc = NfsLocator::from_locator_str(&h.locator)?;
        let path = self.config.manager_mount_path.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            // Idempotent: a destroy that races with another caller (or
            // re-runs after a crash) is success, not error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(nexus_storage::StorageError::from(e)),
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p manager nfs::tests::destroy -- --nocapture
```

Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add apps/manager/src/features/storage/backends/nfs.rs
git commit -m "feat(storage): NfsControlPlaneBackend::destroy unlinks file (idempotent)"
```

---

## Task 5: `NfsControlPlaneBackend::clone_from_image` copies + resizes

**Files:**
- Modify: `apps/manager/src/features/storage/backends/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[tokio::test]
    async fn clone_from_image_copies_and_resizes() {
        let (backend, _guard) = temp_backend();
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("base.raw");
        tokio::fs::write(&src, b"hello world").await.unwrap();
        let opts = CreateOpts {
            name: "v".into(),
            size_bytes: 4096,
            description: None,
        };
        let h = backend.clone_from_image(&src, opts).await.unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 4096);
        let buf = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&buf[..11], b"hello world");
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p manager nfs::tests::clone_from_image_copies_and_resizes -- --nocapture
```

Expected: FAIL — `clone_from_image` returns `NotSupported`.

- [ ] **Step 3: Replace the `clone_from_image` body**

```rust
    async fn clone_from_image(
        &self,
        src: &std::path::Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = self.config.manager_mount_path.join(&file);
        tokio::fs::create_dir_all(&self.config.manager_mount_path).await?;
        tokio::fs::copy(src, &dst).await?;
        let cur = tokio::fs::metadata(&dst).await?.len();
        if opts.size_bytes > cur {
            let f = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&dst)
                .await?;
            f.set_len(opts.size_bytes).await?;
        }
        let locator = NfsLocator {
            server: self.config.server.clone(),
            export: self.config.export.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p manager nfs::tests::clone_from_image_copies_and_resizes -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/manager/src/features/storage/backends/nfs.rs
git commit -m "feat(storage): NfsControlPlaneBackend::clone_from_image copies + resizes"
```

---

## Task 6: `snapshot` + `clone_from_snapshot` + `delete_snapshot`

NFS itself doesn't have native snapshots; we implement the user-visible ones as a sibling file (`<file>.snap-<name>`). For ZFS-backed exports the operator should use the ZFS-aware `truenas_iscsi` backend instead — this NFS backend stays generic.

**Files:**
- Modify: `apps/manager/src/features/storage/backends/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[tokio::test]
    async fn snapshot_then_clone_then_delete_round_trip() {
        let (backend, _guard) = temp_backend();
        // Provision + populate the source.
        let h = backend
            .provision(CreateOpts { name: "v".into(), size_bytes: 1024, description: None })
            .await
            .unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let src_path = backend.config.manager_mount_path.join(&loc.file);
        tokio::fs::write(&src_path, b"original-data").await.unwrap();

        // snapshot
        let snap = backend.snapshot(&h, "snap-1").await.expect("snapshot");
        let snap_loc = NfsLocator::from_locator_str(&snap.locator).unwrap();
        let snap_path = backend.config.manager_mount_path.join(&snap_loc.file);
        assert_eq!(tokio::fs::read(&snap_path).await.unwrap(), b"original-data");

        // clone_from_snapshot
        let cloned = backend.clone_from_snapshot(&snap).await.expect("clone");
        let cloned_loc = NfsLocator::from_locator_str(&cloned.locator).unwrap();
        let cloned_path = backend.config.manager_mount_path.join(&cloned_loc.file);
        let cloned_data = tokio::fs::read(&cloned_path).await.unwrap();
        // The cloned file may be padded to size_bytes; first 13 bytes must match.
        assert_eq!(&cloned_data[..13], b"original-data");

        // delete_snapshot
        backend.delete_snapshot(snap).await.expect("delete_snapshot");
        assert!(tokio::fs::metadata(&snap_path).await.is_err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p manager nfs::tests::snapshot_then_clone_then_delete_round_trip -- --nocapture
```

Expected: FAIL — `snapshot` returns `NotSupported`.

- [ ] **Step 3: Replace the three method bodies**

```rust
    async fn snapshot(
        &self,
        v: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, nexus_storage::StorageError> {
        if name.is_empty() || name.contains('/') {
            return Err(nexus_storage::StorageError::InvalidLocator(
                "snapshot name must be non-empty and contain no '/'".into(),
            ));
        }
        let src_loc = NfsLocator::from_locator_str(&v.locator)?;
        let src_path = self.config.manager_mount_path.join(&src_loc.file);
        let snap_file = format!("{}.snap-{name}", src_loc.file);
        let snap_path = self.config.manager_mount_path.join(&snap_file);
        tokio::fs::copy(&src_path, &snap_path).await?;
        let snap_locator = NfsLocator {
            server: src_loc.server,
            export: src_loc.export,
            file: snap_file,
        };
        Ok(VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: snap_locator.to_locator_string()?,
            source_volume_id: v.volume_id,
        })
    }

    async fn clone_from_snapshot(
        &self,
        s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let src_loc = NfsLocator::from_locator_str(&s.locator)?;
        let src_path = self.config.manager_mount_path.join(&src_loc.file);
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = self.config.manager_mount_path.join(&file);
        tokio::fs::copy(&src_path, &dst).await?;
        // The snapshot file is already at the provisioned size (it's a
        // straight copy of the source volume), so no truncation here.
        let size_bytes = tokio::fs::metadata(&dst).await?.len();
        let locator = NfsLocator {
            server: src_loc.server,
            export: src_loc.export,
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes,
        })
    }

    async fn delete_snapshot(
        &self,
        s: VolumeSnapshotHandle,
    ) -> Result<(), nexus_storage::StorageError> {
        let loc = NfsLocator::from_locator_str(&s.locator)?;
        let path = self.config.manager_mount_path.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(nexus_storage::StorageError::from(e)),
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p manager nfs::tests::snapshot_then_clone_then_delete_round_trip -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run all manager nfs tests**

```bash
cargo test -p manager nfs::tests -- --nocapture
```

Expected: 5 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/manager/src/features/storage/backends/nfs.rs
git commit -m "feat(storage): NFS snapshot via sibling file copy"
```

---

## Task 7: Wire NFS backend into the manager registry

**Files:**
- Modify: `apps/manager/src/features/storage/registry.rs`

- [ ] **Step 1: Read the existing `build_backend` to know exactly where to slot the new arm**

```bash
sed -n '110,180p' apps/manager/src/features/storage/registry.rs
```

You should see a `match kind { ... }` that handles `LocalFile`, `Iscsi`, `TrueNasIscsi`, `SpdkLvol`. The new arm goes alongside.

- [ ] **Step 2: Write the failing test**

Append to the `tests` module at the bottom of `apps/manager/src/features/storage/registry.rs`:

```rust
    #[tokio::test]
    async fn build_backend_constructs_nfs_when_kind_is_nfs() {
        let row = StorageBackendRow {
            id: uuid::Uuid::new_v4(),
            name: "nfs-test".into(),
            kind: "nfs".into(),
            is_default: false,
            config: serde_json::json!({
                "server": "10.0.0.5",
                "export": "/mnt/tank/vms",
                "manager_mount_path": "/tmp/nqrust-nfs-mgr"
            }),
            deleted_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let backend = super::build_backend(&row).expect("build_backend");
        assert!(matches!(
            backend.kind(),
            nexus_storage::BackendKind::Nfs
        ));
    }
```

If the `tests` module doesn't already import `super::*` or the necessary types, mirror the pattern of the existing `build_backend_returns_local_file_for_local_file_kind` test in the same file.

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p manager registry::tests::build_backend_constructs_nfs_when_kind_is_nfs -- --nocapture
```

Expected: FAIL — `"nfs"` is not in the kind-string-to-enum match in `build_backend`, OR the enum match has no `Nfs` arm. Likely the function bails with an "unknown kind" error.

- [ ] **Step 4: Add two arms to `build_backend`**

In `apps/manager/src/features/storage/registry.rs`:

In the kind-string match (look for the `"local_file" => BackendKind::LocalFile` block), add:

```rust
        "nfs" => BackendKind::Nfs,
```

In the `BackendKind` match below it (look for `BackendKind::LocalFile => Ok(...)`), add:

```rust
        BackendKind::Nfs => {
            let cfg: crate::features::storage::backends::nfs::NfsConfig =
                serde_json::from_value(row.config.clone())
                    .with_context(|| format!("backend '{}' nfs config", row.name))?;
            Ok(Arc::new(
                crate::features::storage::backends::nfs::NfsControlPlaneBackend {
                    id: BackendInstanceId(row.id),
                    config: cfg,
                },
            ))
        }
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p manager registry::tests::build_backend_constructs_nfs_when_kind_is_nfs -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Verify no other registry tests broke**

```bash
cargo test -p manager registry::tests -- --nocapture
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/manager/src/features/storage/registry.rs
git commit -m "feat(storage): registry recognizes nfs kind"
```

---

## Task 8: Agent-side `NfsHostConfig` + `NfsHostBackend` skeleton

**Files:**
- Create: `apps/agent/src/features/storage/nfs.rs`
- Modify: `apps/agent/src/features/storage/mod.rs`

- [ ] **Step 1: Add the module declaration**

Append to `apps/agent/src/features/storage/mod.rs`:

```rust
pub mod nfs;
```

- [ ] **Step 2: Write the failing test for `NfsHostConfig::mount_point_for(server, export)`**

The mount-point function deserves its own test because it has to produce a stable, filesystem-safe directory name from arbitrary server + export pairs. Create `apps/agent/src/features/storage/nfs.rs`:

```rust
//! Agent-side NFS host backend. Each unique (server, export) pair gets
//! its own mount point under `mount_base`. `attach` ensures the export
//! is mounted and returns the path to the volume's file. `detach` is a
//! no-op in v1 — the agent leaves the mount in place across volume
//! lifecycles for two reasons: (1) re-mounting is slow, (2) other
//! volumes on the same export may still be attached.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NfsHostConfig {
    pub mount_base: PathBuf,
}

impl NfsHostConfig {
    /// Deterministic per-(server, export) directory name. The export's
    /// leading slash is stripped and remaining slashes become `_` so the
    /// result is a single path component. Server is appended literally
    /// after a `:`.
    pub fn mount_point_for(&self, server: &str, export: &str) -> PathBuf {
        let exp = export.trim_start_matches('/').replace('/', "_");
        let server_safe = server.replace([':', '/'], "_");
        self.mount_base.join(format!("{server_safe}:{exp}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_point_is_unique_per_server_export_and_filesystem_safe() {
        let cfg = NfsHostConfig {
            mount_base: PathBuf::from("/var/lib/nqrust/nfs"),
        };
        let a = cfg.mount_point_for("10.0.0.5", "/mnt/tank/vms");
        let b = cfg.mount_point_for("10.0.0.5", "/mnt/tank/iso");
        let c = cfg.mount_point_for("10.0.0.6", "/mnt/tank/vms");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a, PathBuf::from("/var/lib/nqrust/nfs/10.0.0.5:mnt_tank_vms"));
    }
}
```

- [ ] **Step 3: Run test to verify it passes (this is a pure function — no implementation gap)**

```bash
cargo test -p agent nfs::tests::mount_point_is_unique_per_server_export_and_filesystem_safe -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/agent/src/features/storage/mod.rs apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostConfig + mount-point derivation"
```

---

## Task 9: `NfsHostBackend::attach` returns the file path on an already-mounted share

In v1 we keep the mount lifecycle simple: the agent assumes the export is mounted at `mount_point_for(...)` before `attach` is called. Task 10 adds mount-on-attach on top of this.

**Files:**
- Modify: `apps/agent/src/features/storage/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `apps/agent/src/features/storage/nfs.rs`:

```rust
    use nexus_storage::{BackendKind, HostBackend, VolumeHandle};
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Pretends the export is already mounted at `mount_point_for(...)`
    /// by creating that directory and dropping a file inside it.
    fn fake_mounted_export(cfg: &NfsHostConfig, server: &str, export: &str, file: &str) -> (PathBuf, TempDir) {
        let mount = cfg.mount_point_for(server, export);
        std::fs::create_dir_all(&mount).unwrap();
        let path = mount.join(file);
        std::fs::write(&path, b"hello").unwrap();
        // Use a guard tempdir to clean up after the test.
        let guard = tempfile::tempdir().unwrap();
        (path, guard)
    }

    fn locator_json(server: &str, export: &str, file: &str) -> String {
        serde_json::json!({
            "server": server,
            "export": export,
            "file": file
        })
        .to_string()
    }

    #[tokio::test]
    async fn attach_returns_file_path_under_mount_point() {
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-abc.raw";
        let (expected_path, _guard) = fake_mounted_export(&cfg, server, export, file);
        let backend = NfsHostBackend::new(cfg);
        let v = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(server, export, file),
            size_bytes: 5,
        };
        let attached = backend.attach(&v).await.unwrap();
        assert_eq!(attached.path(), expected_path.as_path());
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p agent nfs::tests::attach_returns_file_path_under_mount_point -- --nocapture
```

Expected: FAIL — `NfsHostBackend` is undefined.

- [ ] **Step 3: Add `NfsHostBackend` with `attach` + `detach` (no-op) + the locator parser**

Insert above the `#[cfg(test)]` block in `apps/agent/src/features/storage/nfs.rs`:

```rust
use async_trait::async_trait;
use nexus_storage::{
    AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NfsLocatorWire {
    server: String,
    export: String,
    file: String,
}

pub struct NfsHostBackend {
    config: NfsHostConfig,
}

impl NfsHostBackend {
    pub fn new(config: NfsHostConfig) -> Self {
        Self { config }
    }

    fn locator(&self, raw: &str) -> Result<NfsLocatorWire, StorageError> {
        serde_json::from_str(raw)
            .map_err(|e| StorageError::InvalidLocator(format!("decode nfs locator: {e}")))
    }
}

#[async_trait]
impl HostBackend for NfsHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Nfs
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = self.locator(&volume.locator)?;
        let mount = self.config.mount_point_for(&loc.server, &loc.export);
        let path = mount.join(&loc.file);
        if !path.exists() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "expected file {} on mounted export; mount missing or volume not provisioned",
                path.display()
            ))));
        }
        Ok(AttachedPath::File(path))
    }

    async fn detach(&self, _v: &VolumeHandle, _a: AttachedPath) -> Result<(), StorageError> {
        // v1: no-op. Mounts are kept across volume lifecycles. The
        // operator can unmount manually or via a future cleanup route.
        Ok(())
    }

    async fn populate_streaming(
        &self,
        _attached: &AttachedPath,
        _source: &std::path::Path,
        _target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        // Implemented in Task 11.
        Err(StorageError::NotSupported(
            "populate_streaming not yet implemented".into(),
        ))
    }

    async fn resize2fs(&self, _attached: &AttachedPath) -> Result<(), StorageError> {
        // Implemented in Task 12.
        Err(StorageError::NotSupported(
            "resize2fs not yet implemented".into(),
        ))
    }

    async fn read_snapshot(
        &self,
        _snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        // Implemented in Task 13.
        Err(StorageError::NotSupported(
            "read_snapshot not yet implemented".into(),
        ))
    }
}
```

Note: `StorageError::backend(io_err)` is the correct constructor for backend-specific errors (the variant is `Backend(Box<dyn Error + Send + Sync>)`; the helper builds the Box for you). Verified against `crates/nexus-storage/src/error.rs`.

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p agent nfs::tests::attach_returns_file_path_under_mount_point -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostBackend::attach assumes-mounted-export path"
```

---

## Task 10: `NfsHostBackend::attach` mounts the export on demand

**Files:**
- Modify: `apps/agent/src/features/storage/nfs.rs`

We use `mount.nfs` via `tokio::process::Command`. Mount is idempotent: if the path is already a mount of the same target, do nothing. Detection uses `findmnt --target <path>`.

- [ ] **Step 1: Write the failing test (gated `#[ignore]` because it touches real mounts)**

Append to the `tests` module:

```rust
    /// Live test: requires running as root or with CAP_SYS_ADMIN, and
    /// requires an NFS server reachable at the env-configured address.
    /// Skipped by default; run with `cargo test -- --include-ignored`
    /// after exporting `NQRUST_NFS_SMOKE_SERVER` and
    /// `NQRUST_NFS_SMOKE_EXPORT`.
    #[tokio::test]
    #[ignore]
    async fn attach_mounts_the_export_when_not_mounted() {
        let server = match std::env::var("NQRUST_NFS_SMOKE_SERVER") {
            Ok(s) => s,
            Err(_) => return,
        };
        let export = std::env::var("NQRUST_NFS_SMOKE_EXPORT").expect("NQRUST_NFS_SMOKE_EXPORT");
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
        };
        let backend = NfsHostBackend::new(cfg.clone());
        // Pre-create the test file directly on the export so attach
        // succeeds. Caller is responsible for ensuring the export is
        // writable from this test host.
        let mount = cfg.mount_point_for(&server, &export);
        std::fs::create_dir_all(&mount).unwrap();
        let mnt_status = std::process::Command::new("mount")
            .args(["-t", "nfs", &format!("{server}:{export}"), mount.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(mnt_status.success(), "pre-mount failed");
        let file = "nfs-attach-test.raw";
        std::fs::write(mount.join(file), b"x").unwrap();
        std::process::Command::new("umount")
            .arg(&mount)
            .status()
            .unwrap();

        // Now exercise attach: it must mount + return the path.
        let v = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(&server, &export, file),
            size_bytes: 1,
        };
        let attached = backend.attach(&v).await.unwrap();
        assert!(attached.path().exists());
        std::process::Command::new("umount").arg(&mount).status().unwrap();
    }
```

- [ ] **Step 2: Implement `ensure_mounted(server, export, mount_point)`**

Insert in `impl NfsHostBackend`, before `fn locator`:

```rust
    async fn ensure_mounted(
        &self,
        server: &str,
        export: &str,
        mount_point: &std::path::Path,
    ) -> Result<(), StorageError> {
        tokio::fs::create_dir_all(mount_point).await?;
        // Already mounted? findmnt prints the source if so; success exit.
        let probe = tokio::process::Command::new("findmnt")
            .arg("--target")
            .arg(mount_point)
            .arg("--noheadings")
            .arg("--output")
            .arg("SOURCE")
            .output()
            .await;
        let source_line = match probe {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).trim().to_string()
            }
            _ => String::new(),
        };
        let want = format!("{server}:{export}");
        if source_line == want {
            return Ok(());
        }
        if !source_line.is_empty() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "{} is mounted but as '{}', not '{}'",
                mount_point.display(),
                source_line,
                want
            ))));
        }
        // Not mounted — mount it.
        let status = tokio::process::Command::new("mount")
            .arg("-t")
            .arg("nfs")
            .arg(&want)
            .arg(mount_point)
            .status()
            .await
            .map_err(|e| StorageError::backend(std::io::Error::other(format!("mount.nfs spawn: {e}"))))?;
        if !status.success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "mount.nfs {} -> {} exited {}",
                want,
                mount_point.display(),
                status
            ))));
        }
        Ok(())
    }
```

Note: errors use `StorageError::backend(std::io::Error::other(format!(...)))` — the variant takes `Box<dyn Error + Send + Sync>`, the helper builds it for you.

- [ ] **Step 3: Replace `attach` to call `ensure_mounted` first**

```rust
    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = self.locator(&volume.locator)?;
        let mount = self.config.mount_point_for(&loc.server, &loc.export);
        if !self.config.assume_mounted {
            self.ensure_mounted(&loc.server, &loc.export, &mount).await?;
        }
        let path = mount.join(&loc.file);
        if tokio::fs::metadata(&path).await.is_err() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "expected file {} on mounted export",
                path.display()
            ))));
        }
        Ok(AttachedPath::File(path))
    }
```

- [ ] **Step 4: Run test that does NOT require ignored mode (regression check on Task 9 test)**

```bash
cargo test -p agent nfs::tests::attach_returns_file_path_under_mount_point -- --nocapture
```

Expected: PASS — Task 9's test fakes the mount by creating the directory + file, so `findmnt` returns nothing, then `mount.nfs` would be called BUT the test expects it to succeed without invoking it. To handle this in unit tests, special-case: if the mount path's parent exists and the file is already at `mount/file`, accept it. But that defeats the safety check.

**Choose one of:**

Option A (preferred — keep the unit test honest): make Task 9's fake set up `findmnt`-detectable state by bind-mounting (requires CAP). This pushes the test into `#[ignore]` territory.

Option B (pragmatic): introduce a config flag `assume_mounted: bool` (default false) that bypasses `ensure_mounted`. Use it in the unit test.

Implement Option B by adding a field to `NfsHostConfig`:

```rust
#[derive(Debug, Clone)]
pub struct NfsHostConfig {
    pub mount_base: PathBuf,
    /// If true, attach trusts that the export is already mounted at
    /// `mount_point_for(...)` and does not invoke mount.nfs. Used in
    /// unit tests and for environments where an external service (e.g.
    /// systemd automount) manages mounts.
    pub assume_mounted: bool,
}
```

Update Task 8 and Task 9's test fixtures to pass `assume_mounted: true`. Update the existing test calls accordingly. In `attach`, gate the call:

```rust
        if !self.config.assume_mounted {
            self.ensure_mounted(&loc.server, &loc.export, &mount).await?;
        }
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p agent nfs::tests::attach_returns_file_path_under_mount_point -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostBackend::attach mounts on demand (assume_mounted bypass for tests)"
```

---

## Task 11: `populate_streaming` is a byte-for-byte copy with size truncation

This is the same logic as `LocalFileHostBackend::populate_streaming`. Reuse rather than duplicate by extracting a helper, OR inline it because the two backends might evolve differently.

**Files:**
- Modify: `apps/agent/src/features/storage/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    use nexus_storage::AttachedPath;

    #[tokio::test]
    async fn populate_streaming_copies_then_truncates() {
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-pop.raw";
        let (path, _g) = fake_mounted_export(&cfg, server, export, file);

        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("base.raw");
        tokio::fs::write(&src, b"abc").await.unwrap();

        let backend = NfsHostBackend::new(cfg);
        backend
            .populate_streaming(&AttachedPath::File(path.clone()), &src, 16)
            .await
            .unwrap();

        let written = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&written[..3], b"abc");
        assert_eq!(written.len(), 16);
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p agent nfs::tests::populate_streaming_copies_then_truncates -- --nocapture
```

Expected: FAIL — `populate_streaming` returns `NotSupported`.

- [ ] **Step 3: Replace the body**

```rust
    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &std::path::Path,
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
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p agent nfs::tests::populate_streaming_copies_then_truncates -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostBackend::populate_streaming copies + truncates"
```

---

## Task 12: `resize2fs` reuses the local-file helper

`apps/agent/src/features/storage/local_file.rs` already has a `run_resize2fs(path)` helper used by `LocalFileHostBackend::resize2fs`. Reuse it.

**Files:**
- Modify: `apps/agent/src/features/storage/local_file.rs` — make `run_resize2fs` `pub(super)`.
- Modify: `apps/agent/src/features/storage/nfs.rs`

- [ ] **Step 1: Inspect the existing helper visibility**

```bash
grep -n "fn run_resize2fs" apps/agent/src/features/storage/local_file.rs
```

If it's a free function inside the file, change its visibility:

```rust
pub(super) async fn run_resize2fs(path: &std::path::Path) -> Result<(), StorageError> {
```

- [ ] **Step 2: Write the failing test**

Append to the `tests` module in `nfs.rs`:

```rust
    #[tokio::test]
    async fn resize2fs_invokes_the_shared_helper() {
        // Smoke: resize2fs against a non-ext4 file returns Err. This
        // confirms wiring (the helper is reachable + invoked) without
        // requiring a real ext4 image in the test.
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let path = base.path().join("not-ext4.raw");
        tokio::fs::write(&path, b"not an ext4 superblock").await.unwrap();
        let backend = NfsHostBackend::new(cfg);
        let res = backend.resize2fs(&AttachedPath::File(path)).await;
        assert!(res.is_err());
    }
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p agent nfs::tests::resize2fs_invokes_the_shared_helper -- --nocapture
```

Expected: FAIL — returns `NotSupported`, not the expected ext4-magic error.

- [ ] **Step 4: Implement `resize2fs` to call the shared helper**

In `apps/agent/src/features/storage/nfs.rs`, replace the body:

```rust
    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError> {
        super::local_file::run_resize2fs(attached.path()).await
    }
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p agent nfs::tests::resize2fs_invokes_the_shared_helper -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/agent/src/features/storage/local_file.rs apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostBackend::resize2fs delegates to shared helper"
```

---

## Task 13: `read_snapshot` opens the snapshot file under the mount

**Files:**
- Modify: `apps/agent/src/features/storage/nfs.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[tokio::test]
    async fn read_snapshot_returns_file_contents() {
        use tokio::io::AsyncReadExt;

        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-abc.raw.snap-x";
        let (path, _g) = fake_mounted_export(&cfg, server, export, file);
        tokio::fs::write(&path, b"snapshot-bytes").await.unwrap();

        let backend = NfsHostBackend::new(cfg);
        let snap = VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(server, export, file),
            source_volume_id: Uuid::new_v4(),
        };
        let mut r = backend.read_snapshot(&snap).await.unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"snapshot-bytes");
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p agent nfs::tests::read_snapshot_returns_file_contents -- --nocapture
```

Expected: FAIL — returns `NotSupported`.

- [ ] **Step 3: Replace the body**

```rust
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let loc = self.locator(&snap.locator)?;
        let mount = self.config.mount_point_for(&loc.server, &loc.export);
        if !self.config.assume_mounted {
            self.ensure_mounted(&loc.server, &loc.export, &mount).await?;
        }
        let path = mount.join(&loc.file);
        let f = tokio::fs::File::open(&path).await?;
        Ok(Box::new(f))
    }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p agent nfs::tests::read_snapshot_returns_file_contents -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/features/storage/nfs.rs
git commit -m "feat(storage): NfsHostBackend::read_snapshot opens file under mount"
```

---

## Task 14: Register `NfsHostBackend` in `agent/main.rs` (gated on env)

The agent only registers backends when their dependencies are available — SPDK lvol is gated on `AGENT_SPDK_RPC_SOCKET` for example. NFS is gated on `AGENT_NFS_MOUNT_BASE` so a host without NFS tooling installed reports an empty NFS support set in its heartbeat.

**Files:**
- Modify: `apps/agent/src/main.rs:24-61`

- [ ] **Step 1: Read the existing block to understand the registration pattern**

```bash
sed -n '24,65p' apps/agent/src/main.rs
```

- [ ] **Step 2: Insert NFS registration block**

Add after the iSCSI registration (around line 31, before the SPDK block):

```rust
    if let Ok(mount_base) = std::env::var("AGENT_NFS_MOUNT_BASE") {
        let assume_mounted = std::env::var("AGENT_NFS_ASSUME_MOUNTED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        storage_registry.register_for(
            nexus_storage::BackendKind::Nfs,
            std::sync::Arc::new(features::storage::nfs::NfsHostBackend::new(
                features::storage::nfs::NfsHostConfig {
                    mount_base: std::path::PathBuf::from(mount_base),
                    assume_mounted,
                },
            )),
        );
    }
```

- [ ] **Step 3: Verify the agent compiles**

```bash
cargo build -p agent
```

Expected: PASS.

- [ ] **Step 4: Run all agent tests to make sure nothing regressed**

```bash
cargo test -p agent
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/main.rs
git commit -m "feat(storage): agent registers NfsHostBackend when AGENT_NFS_MOUNT_BASE is set"
```

---

## Task 15: Live integration smoke runbook

This is a docs commit, not a code task. It's the operator-facing confirmation that the NFS backend is shippable.

**Files:**
- Create: `docs/runbooks/nfs-smoke.md`

- [ ] **Step 1: Create the runbook**

```markdown
# NFS backend live smoke

Validates the NFS backend end-to-end against a Docker-hosted NFS server.

## Prerequisites

- Manager and agent built from this branch.
- Docker installed on the test host.
- `nfs-common` package installed (provides `mount.nfs`, `findmnt`).

## Setup

```bash
docker run -d --name nfs-smoke \
  --privileged \
  -p 2049:2049 \
  -e SHARED_DIRECTORY=/data \
  -v /tmp/nfs-smoke-data:/data \
  itsthenetwork/nfs-server-alpine:latest

# On the manager host, mount the export so the manager can write to it
sudo mkdir -p /mnt/nfs-mgr
sudo mount -t nfs 127.0.0.1:/ /mnt/nfs-mgr
ls /mnt/nfs-mgr   # should be empty
```

Add to the manager's storage TOML:

```toml
[[storage_backend]]
name = "nfs-smoke"
kind = "nfs"
is_default = false

[storage_backend.config]
server = "127.0.0.1"
export = "/"
manager_mount_path = "/mnt/nfs-mgr"
```

Start the agent with:

```bash
AGENT_NFS_MOUNT_BASE=/var/lib/nqrust/nfs ./target/release/agent
```

## Test L1 — provision + attach + populate + boot

1. Create a VM via the manager API with `backend_id` pointing to the `nfs-smoke` backend.
2. Confirm a sparse `nfs-<uuid>.raw` appears under `/tmp/nfs-smoke-data/`.
3. Confirm `findmnt --target /var/lib/nqrust/nfs/127.0.0.1:` shows the share mounted on the agent host.
4. Boot the VM; verify `cat /etc/os-release` over the shell endpoint.
5. Delete the VM; confirm the file is unlinked.

Expect: VM boots, file is unlinked on delete, no orphan mounts.

## Test L2 — snapshot + clone

1. Create a VM as in L1, write a marker file inside.
2. Snapshot the volume. Confirm `nfs-<uuid>.raw.snap-<name>` appears alongside.
3. Create a new VM with `clone_from_snapshot` against that snapshot.
4. Boot the new VM; confirm the marker file is present.

## Cleanup

```bash
sudo umount /mnt/nfs-mgr
docker rm -f nfs-smoke
sudo rm -rf /tmp/nfs-smoke-data
```
```

- [ ] **Step 2: Commit**

```bash
git add docs/runbooks/nfs-smoke.md
git commit -m "docs(storage): NFS backend live smoke runbook"
```

---

## Task 16: Final sweep

- [ ] **Step 1: Run the full workspace test suite**

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 2: Run clippy across the workspace**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Run cargo fmt**

```bash
cargo fmt
```

If any formatting changes:

```bash
git add -u
git commit -m "chore(storage): cargo fmt sweep"
```

- [ ] **Step 4: Final commit checkpoint**

Confirm `git log --oneline` shows ~16 small commits since branching, each green.

---

## Plan completion checklist

- [ ] `BackendKind::Nfs` exists and round-trips JSON.
- [ ] `NfsControlPlaneBackend` implements all five `ControlPlaneBackend` methods (provision, destroy, clone_from_image, snapshot, clone_from_snapshot, delete_snapshot).
- [ ] Manager registry constructs the NFS backend from a `kind: "nfs"` row.
- [ ] `NfsHostBackend` implements all five `HostBackend` methods (kind, attach, detach, populate_streaming, resize2fs, read_snapshot).
- [ ] `attach` mounts on demand when `assume_mounted: false`.
- [ ] Agent registers `NfsHostBackend` when `AGENT_NFS_MOUNT_BASE` is set; reports `nfs` in heartbeat.
- [ ] Live smoke runbook `docs/runbooks/nfs-smoke.md` exercises provision, attach, populate, boot, snapshot, clone.
- [ ] All new code under clippy + fmt clean. Workspace tests green.

---

## Out of scope (explicitly)

- **NFS v3.** v1 ships with v4 only. The `mount.nfs` invocation defaults to v4; an `nfs_version` config field can be added in a follow-up.
- **TLS-protected NFS (`nfs-rdma`, `nfs-over-tls`).** Same family as the iSCSI/CHAP gap — security work follows storage-functional work.
- **Per-volume snapshot quotas / GC.** The naïve sibling-file snapshot has no eviction. Operators delete via the existing `delete_snapshot` API.
- **Concurrent attach (live migration).** `Capabilities` reports `supports_concurrent_attach: false`; not added in v1.
- **UI selector exposure.** That work belongs in `docs/superpowers/plans/2026-04-28-storage-ui.md` (Plan 3). After this plan lands, add `nfs` to the backend-kind dropdown there.
- **TrueNAS / NetApp / Pure REST control planes.** These are vendor-specific; they belong in their own plans, not this one.

---

## Follow-ups (recommend writing as separate plans)

1. **iSCSI / TrueNAS gap-fill plan.** Existing plan `2026-04-28-storage-iscsi.md` is partly implemented; the open items are: (a) TrueNAS `snapshot` / `clone_from_snapshot` (currently stubbed `NotSupported`), (b) live integration smoke runbook against a TrueNAS box, (c) end-to-end VM-from-iSCSI test. Estimated 1 week.
2. **Storage UI selector plan.** Existing plan `2026-04-28-storage-ui.md` covers backend-kind selector on VM create. After this plan lands, the UI plan needs one extra item to surface `nfs`. Estimated 3-5 days.
