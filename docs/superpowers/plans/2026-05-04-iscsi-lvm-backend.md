# iSCSI-LVM Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `iscsi_lvm` backend kind that lets operators register any iSCSI target (vendor-agnostic) and auto-provision per-VM block devices on top of it via LVM. Mirrors Proxmox's `lvm` storage with `base = iscsi:...`, the canonical solution that gives "carve one big LUN, get N auto-provisioned VM disks" without a per-vendor REST adapter.

**Architecture:** New control-plane backend (`iscsi_lvm`) that delegates all privileged operations to the agent over HTTP. Agent owns iSCSI session lifecycle (`iscsiadm` login persistent across restarts), VG metadata operations (`pvcreate`/`vgcreate` once at backend init), and per-VM LV lifecycle (`lvcreate`/`lvremove`/`lvchange`). The trait gains `activate_volume`/`deactivate_volume` so the manager can gate per-VM block-device exclusivity — the safety mechanism that lets a single LUN serve N VMs across N hosts without a cluster filesystem. Reference implementation: `~/refs/pve-storage/src/PVE/Storage/LVMPlugin.pm` (1475 lines) + `ISCSIPlugin.pm` (706 lines). We follow Proxmox's command shapes and lifecycle exactly.

**Tech Stack:** Rust (manager + agent), `iscsiadm` (open-iscsi), `pvcreate`/`vgcreate`/`lvcreate`/`lvchange`/`lvremove`/`lvs`/`vgs` (lvm2), `qemu-img` (clone), Postgres (advisory locks for HA-safety), Next.js (UI form + initialize button).

---

## Pre-flight: Scope Check

This plan assumes:
- Single manager process, one or more agents (current architecture).
- We do **not** ship live migration in this plan (that's a separate plan that builds on this one).
- Multi-manager HA deferred. We add Postgres advisory locks around metadata mutations so HA is safe later, but don't require multi-manager testing.
- `iscsi_generic` and `truenas_iscsi` backends remain unchanged. UI marks `iscsi_generic` as "advanced" and recommends `iscsi_lvm` for non-TrueNAS arrays.

---

## File Structure

### New files

- `crates/nexus-storage/src/control_plane.rs` — extend (no new file): add `activate_volume` / `deactivate_volume` trait methods.
- `apps/manager/src/features/storage/backends/iscsi_lvm.rs` — manager-side `IscsiLvmControlPlaneBackend` (NEW).
- `apps/agent/src/features/storage/iscsi_lvm.rs` — agent-side `IscsiLvmHostBackend` + helpers (NEW).
- `apps/agent/src/features/storage/iscsi_lvm/routes.rs` — `/v1/storage/iscsi_lvm/*` HTTP routes (NEW, optional split if `iscsi_lvm.rs` grows past ~400 lines).
- `apps/manager/migrations/0038_iscsi_lvm_backend.sql` — extend `BackendKind` CHECK constraint + indexes (NEW).
- `apps/manager/src/features/storage_backends/initialize.rs` — POST `/v1/storage_backends/:id/initialize` route for one-time `pvcreate + vgcreate` (NEW).
- `apps/ui/components/storage/lvm-initialize-dialog.tsx` — destructive-warning dialog before pvcreate (NEW).
- `docs/runbooks/iscsi-lvm-troubleshooting.md` — operational doc: how to recover from common failure modes (NEW).

### Modified files

- `crates/nexus-storage/src/control_plane.rs` — add 2 new trait methods (default no-op).
- `crates/nexus-storage/src/types.rs` (or wherever `BackendKind` lives) — add `IscsiLvm` variant.
- `apps/manager/src/features/storage/registry.rs` — `build_backend` arm for `iscsi_lvm`.
- `apps/manager/src/features/storage/config.rs` — validate `iscsi_lvm` config (portal, IQN, vg_name).
- `apps/manager/src/features/storage_backends/routes.rs` — probe arm for iscsi_lvm; expose initialize route.
- `apps/manager/src/features/storage_backends/health.rs` — health probe arm: session alive + VG present + free space.
- `apps/manager/src/features/vms/service.rs` — call `activate_volume(handle)` before Firecracker spawn; `deactivate_volume` after VM stop. Wire `host_path_for` for iscsi_lvm.
- `apps/manager/src/features/vms/routes.rs` — VM stop handler triggers deactivate.
- `apps/agent/src/main.rs` — register `IscsiLvmHostBackend` if iSCSI tools available.
- `apps/agent/src/features/storage/mod.rs` — wire iscsi_lvm router under `/v1/storage/iscsi_lvm`.
- `apps/manager/storage.toml.example` — add commented `iscsi_lvm` example.
- `apps/ui/lib/types/index.ts` — add `iscsi_lvm` to `BackendKind` enum.
- `apps/ui/lib/queries.ts` — add `useInitializeBackend(id)` mutation.
- `apps/ui/components/storage/backend-create-dialog.tsx` — add `iscsi_lvm` form fields (portal, IQN, vg_name).
- `apps/ui/components/storage/backend-table.tsx` — show "Initialize" button for `iscsi_lvm` rows that aren't yet initialized.
- `CHANGELOG.md` — entry for new backend.

---

## Task Breakdown

Each task is independent enough to ship + test on its own. Tasks 1–3 establish the trait extension + types. Tasks 4–8 build the agent side bottom-up (parsers, then session mgmt, then VG/LV ops, then routes). Tasks 9–13 build the manager side. Tasks 14–17 are UI + plumbing. Task 18 is end-to-end verification.

---

### Task 1: `BackendKind::IscsiLvm` enum variant

**Files:**
- Modify: `crates/nexus-storage/src/types.rs`
- Modify: `crates/nexus-types/src/lib.rs` (the wire-types `BackendKind` mirror)
- Modify: `apps/ui/lib/types/index.ts`

- [ ] **Step 1: Add the variant**

```rust
// crates/nexus-storage/src/types.rs
pub enum BackendKind {
    LocalFile,
    Iscsi,
    TrueNasIscsi,
    SpdkLvol,
    Nfs,
    IscsiLvm, // <-- add
}

impl BackendKind {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            // existing arms...
            BackendKind::IscsiLvm => "iscsi_lvm",
        }
    }
}
```

- [ ] **Step 2: Add the wire-side mirror**

Same shape in `crates/nexus-types/src/lib.rs`. Make sure `serde(rename_all = "snake_case")` keeps the JSON encoding `"iscsi_lvm"`.

- [ ] **Step 3: TS mirror**

```ts
// apps/ui/lib/types/index.ts
export type BackendKind = "local_file" | "iscsi" | "truenas_iscsi" | "spdk_lvol" | "nfs" | "iscsi_lvm";
```

- [ ] **Step 4: Build to confirm exhaustive matches caught at compile time**

Run: `cargo build --release -p manager -p agent`
Expected: errors in every `match BackendKind` site that isn't exhaustive — e.g. `registry.rs::build_backend`, `health.rs::check_backend_health`, `config.rs::validate`. Add placeholder `IscsiLvm => Err(anyhow!("not yet implemented"))` arms so the build is green; subsequent tasks fill them in.

- [ ] **Step 5: Commit**

```bash
git add crates/nexus-storage/src/types.rs crates/nexus-types/src/lib.rs apps/ui/lib/types/index.ts apps/manager/src/features/storage/registry.rs apps/manager/src/features/storage_backends/health.rs apps/manager/src/features/storage/config.rs
git commit -m "feat(storage): add IscsiLvm backend kind variant + placeholder arms"
```

---

### Task 2: `activate_volume` / `deactivate_volume` trait methods

Adds the lifecycle hook for backends that need explicit per-VM block-device gating. Default no-op for stateless backends. iscsi_lvm overrides.

**Files:**
- Modify: `crates/nexus-storage/src/control_plane.rs`
- Test: `crates/nexus-storage/src/control_plane.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing trait test**

```rust
// crates/nexus-storage/src/control_plane.rs (in #[cfg(test)] mod)
#[tokio::test]
async fn default_activate_deactivate_are_noop_ok() {
    struct Stub;
    #[async_trait::async_trait]
    impl ControlPlaneBackend for Stub {
        fn kind(&self) -> BackendKind { BackendKind::LocalFile }
        fn capabilities(&self) -> Capabilities { Capabilities::default() }
        async fn provision(&self, _: CreateOpts) -> Result<VolumeHandle, StorageError> { unimplemented!() }
        async fn destroy(&self, _: VolumeHandle) -> Result<(), StorageError> { unimplemented!() }
        async fn clone_from_image(&self, _: &std::path::Path, _: CreateOpts) -> Result<VolumeHandle, StorageError> { unimplemented!() }
        async fn snapshot(&self, _: &VolumeHandle, _: &str) -> Result<VolumeSnapshotHandle, StorageError> { unimplemented!() }
        async fn clone_from_snapshot(&self, _: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> { unimplemented!() }
        async fn delete_snapshot(&self, _: VolumeSnapshotHandle) -> Result<(), StorageError> { unimplemented!() }
    }
    let s = Stub;
    let h = VolumeHandle { volume_id: uuid::Uuid::new_v4(), locator: String::new(), size_bytes: 0 };
    s.activate_volume(&h).await.unwrap();
    s.deactivate_volume(&h).await.unwrap();
}
```

- [ ] **Step 2: Run — should fail to compile (`activate_volume` undefined)**

Run: `cargo test -p nexus-storage default_activate`
Expected: compilation error: no method named `activate_volume`

- [ ] **Step 3: Add the trait methods with default no-op**

```rust
#[async_trait]
pub trait ControlPlaneBackend: Send + Sync {
    // ... existing methods ...

    /// Make a volume usable on this host. Backends with shared block
    /// storage (LVM-on-iSCSI, FC LUNs) override this to do exclusive
    /// activation (`lvchange -aey`). Default no-op for backends where
    /// every host can access the file independently (NFS, local_file).
    async fn activate_volume(&self, _handle: &VolumeHandle) -> Result<(), StorageError> {
        Ok(())
    }

    /// Inverse of `activate_volume`. Called when the VM stops on this
    /// host so another host can activate the same volume (live migration).
    async fn deactivate_volume(&self, _handle: &VolumeHandle) -> Result<(), StorageError> {
        Ok(())
    }
}
```

- [ ] **Step 4: Run — should pass**

Run: `cargo test -p nexus-storage`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/nexus-storage/src/control_plane.rs
git commit -m "feat(storage): add activate_volume/deactivate_volume trait methods"
```

---

### Task 3: Migration 0038 — extend backend kinds

**Files:**
- Create: `apps/manager/migrations/0038_iscsi_lvm_backend.sql`

- [ ] **Step 1: Find the existing CHECK constraint or enum**

Run: `grep -rn "BackendKind\|backend_kind\|kind.*CHECK" apps/manager/migrations/`
Expected: shows the constraint or absence thereof.

- [ ] **Step 2: Write the migration**

```sql
-- apps/manager/migrations/0038_iscsi_lvm_backend.sql
-- Add 'iscsi_lvm' to the allowed storage_backend.kind values. The CHECK
-- constraint guards against typos at insert time. host_backend_kinds
-- is updated similarly so the agent can advertise it.

ALTER TABLE storage_backend
    DROP CONSTRAINT IF EXISTS storage_backend_kind_check;

ALTER TABLE storage_backend
    ADD CONSTRAINT storage_backend_kind_check
    CHECK (kind IN ('local_file', 'iscsi', 'truenas_iscsi', 'spdk_lvol', 'nfs', 'iscsi_lvm'));
```

- [ ] **Step 3: Verify migration runs cleanly**

Restart manager. Look for the migration in startup log:
```
INFO sqlx::postgres::notice: running migration 0038_iscsi_lvm_backend
```
No errors. Test by inserting a placeholder row from psql:
```sql
INSERT INTO storage_backend (name, kind, config_json, capabilities_json, is_default, source)
VALUES ('test-iscsi-lvm', 'iscsi_lvm', '{}'::jsonb, '{}'::jsonb, false, 'ui')
RETURNING id;
DELETE FROM storage_backend WHERE name = 'test-iscsi-lvm';
```
Expected: insert succeeds, delete succeeds.

- [ ] **Step 4: Commit**

```bash
git add apps/manager/migrations/0038_iscsi_lvm_backend.sql
git commit -m "feat(storage): migration 0038 — allow iscsi_lvm backend kind"
```

---

### Task 4: Agent — pure-logic LVM output parsers

The agent needs to parse `pvs`, `vgs`, `lvs` output. Pure-logic parsers tested without root or LVM installed.

**Files:**
- Create: `apps/agent/src/features/storage/iscsi_lvm.rs`
- Test: same file (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing parser tests**

```rust
// apps/agent/src/features/storage/iscsi_lvm.rs
#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parse_pvs_extracts_vg_for_device() {
        // pvs --separator : --noheadings --units k --nosuffix --options pv_name,pv_size,vg_name,pv_uuid
        let out = "  /dev/sdb:104857600:vg-nqrust:abcd-1234-uuid";
        let info = parse_pv_info(out).expect("parsed");
        assert_eq!(info.pv_name, "/dev/sdb");
        assert_eq!(info.size_kb, 104857600);
        assert_eq!(info.vg_name.as_deref(), Some("vg-nqrust"));
    }

    #[test]
    fn parse_pvs_no_vg_when_unitialized() {
        let out = "  /dev/sdc:104857600::xyz-uuid";
        let info = parse_pv_info(out).expect("parsed");
        assert!(info.vg_name.is_none());
    }

    #[test]
    fn parse_vgs_returns_size_free() {
        // vgs --separator : --noheadings --units b --nosuffix --options vg_name,vg_size,vg_free,lv_count
        let out = "vg-nqrust:107374182400:96636764160:3";
        let info = parse_vg_info(out).expect("parsed");
        assert_eq!(info.name, "vg-nqrust");
        assert_eq!(info.size_bytes, 107374182400);
        assert_eq!(info.free_bytes, 96636764160);
        assert_eq!(info.lv_count, 3);
    }

    #[test]
    fn parse_lvs_extracts_lvs_with_tags() {
        // lvs --separator : --noheadings --options lv_name,lv_size,lv_tags,lv_attr
        let out = "vm-100-disk-0:10737418240:nqrust-vm-100:-wi-a-----";
        let info = parse_lv_info(out).expect("parsed");
        assert_eq!(info.name, "vm-100-disk-0");
        assert_eq!(info.size_bytes, 10737418240);
        assert_eq!(info.tags, vec!["nqrust-vm-100".to_string()]);
        assert!(!info.is_active);
    }

    #[test]
    fn parse_lvs_active_volume_has_a_in_attr() {
        let out = "vm-100-disk-0:10737418240:nqrust-vm-100:-wi-ao----";
        let info = parse_lv_info(out).expect("parsed");
        assert!(info.is_active);
    }
}
```

- [ ] **Step 2: Run — should fail (parsers not defined)**

Run: `cargo test -p agent parser_tests`
Expected: compile error.

- [ ] **Step 3: Implement parsers**

```rust
// apps/agent/src/features/storage/iscsi_lvm.rs

#[derive(Debug, Clone)]
pub struct PvInfo {
    pub pv_name: String,
    pub size_kb: u64,
    pub vg_name: Option<String>,
    pub uuid: String,
}

#[derive(Debug, Clone)]
pub struct VgInfo {
    pub name: String,
    pub size_bytes: u64,
    pub free_bytes: u64,
    pub lv_count: u32,
}

#[derive(Debug, Clone)]
pub struct LvInfo {
    pub name: String,
    pub size_bytes: u64,
    pub tags: Vec<String>,
    pub is_active: bool,
}

pub fn parse_pv_info(line: &str) -> Option<PvInfo> {
    let parts: Vec<&str> = line.trim().split(':').collect();
    if parts.len() != 4 { return None; }
    Some(PvInfo {
        pv_name: parts[0].to_string(),
        size_kb: parts[1].parse().ok()?,
        vg_name: if parts[2].is_empty() { None } else { Some(parts[2].to_string()) },
        uuid: parts[3].to_string(),
    })
}

pub fn parse_vg_info(line: &str) -> Option<VgInfo> {
    let parts: Vec<&str> = line.trim().split(':').collect();
    if parts.len() < 4 { return None; }
    Some(VgInfo {
        name: parts[0].to_string(),
        size_bytes: parts[1].parse().ok()?,
        free_bytes: parts[2].parse().ok()?,
        lv_count: parts[3].parse().ok()?,
    })
}

pub fn parse_lv_info(line: &str) -> Option<LvInfo> {
    let parts: Vec<&str> = line.trim().split(':').collect();
    if parts.len() < 4 { return None; }
    let tags: Vec<String> = if parts[2].is_empty() {
        Vec::new()
    } else {
        parts[2].split(',').map(|s| s.to_string()).collect()
    };
    // lv_attr 5th char: 'a' = active, '-' = not
    let attr = parts[3];
    let is_active = attr.chars().nth(4).map(|c| c == 'a').unwrap_or(false);
    Some(LvInfo {
        name: parts[0].to_string(),
        size_bytes: parts[1].parse().ok()?,
        tags,
        is_active,
    })
}
```

- [ ] **Step 4: Run — should pass**

Run: `cargo test -p agent parser_tests`
Expected: 5 passing.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/features/storage/iscsi_lvm.rs
git commit -m "feat(agent): pure-logic parsers for pvs/vgs/lvs output"
```

---

### Task 5: Agent — iSCSI session lifecycle helpers

Persistent iSCSI sessions across agent restarts. Mirrors `ISCSIPlugin::iscsi_login`/`iscsi_logout` and uses `--op update -n node.startup -v automatic` so sessions reconnect after host reboot.

**Files:**
- Modify: `apps/agent/src/features/storage/iscsi_lvm.rs`

- [ ] **Step 1: Write tests for session helpers (mock command exec)**

```rust
#[cfg(test)]
mod session_tests {
    use super::*;
    // We can't easily mock std::process::Command without a trait abstraction.
    // Strategy: unit-test the *argument construction* via a helper that
    // returns the command vector, run live tests under #[ignore] gated on
    // NQRUST_ISCSI_LVM_LIVE_PORTAL env var.

    #[test]
    fn build_iscsi_login_args_includes_persistent_flag() {
        let args = build_iscsi_login_args("iqn.foo:bar", "192.168.1.10:3260");
        let s = args.join(" ");
        assert!(s.contains("--mode node"));
        assert!(s.contains("--targetname iqn.foo:bar"));
        assert!(s.contains("--portal 192.168.1.10:3260"));
        assert!(s.contains("--login"));
    }

    #[test]
    fn build_iscsi_make_persistent_args() {
        let args = build_iscsi_persistent_args("iqn.foo:bar");
        let s = args.join(" ");
        assert!(s.contains("--op update"));
        assert!(s.contains("node.startup"));
        assert!(s.contains("automatic"));
    }
}
```

- [ ] **Step 2: Implement the arg builders**

```rust
fn build_iscsi_login_args(iqn: &str, portal: &str) -> Vec<String> {
    vec![
        "--mode".into(), "node".into(),
        "--targetname".into(), iqn.into(),
        "--portal".into(), portal.into(),
        "--login".into(),
    ]
}

fn build_iscsi_persistent_args(iqn: &str) -> Vec<String> {
    vec![
        "--mode".into(), "node".into(),
        "--targetname".into(), iqn.into(),
        "--op".into(), "update".into(),
        "--name".into(), "node.startup".into(),
        "--value".into(), "automatic".into(),
    ]
}

pub async fn iscsi_login(iqn: &str, portal: &str) -> Result<(), StorageError> {
    // Discovery first.
    tokio::process::Command::new("iscsiadm")
        .args(["--mode", "discovery", "--type", "sendtargets", "--portal", portal])
        .status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("iscsiadm discovery: {e}"))))?;
    // Login (idempotent: returns 15 if already logged in, treat as ok).
    let status = tokio::process::Command::new("iscsiadm")
        .args(&build_iscsi_login_args(iqn, portal))
        .status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("iscsiadm login spawn: {e}"))))?;
    if !status.success() && status.code() != Some(15) {
        return Err(StorageError::backend(std::io::Error::other(format!("iscsiadm login failed: exit {:?}", status.code()))));
    }
    // Make persistent.
    tokio::process::Command::new("iscsiadm")
        .args(&build_iscsi_persistent_args(iqn))
        .status().await.ok();
    Ok(())
}

pub async fn iscsi_logout(iqn: &str) -> Result<(), StorageError> {
    tokio::process::Command::new("iscsiadm")
        .args(["--mode", "node", "--targetname", iqn, "--logout"])
        .status().await.ok();
    Ok(())
}

/// Find the block device path for a logged-in target+LUN. Walks
/// `/dev/disk/by-path/` looking for `ip-<portal>-iscsi-<iqn>-lun-N`.
pub async fn resolve_iscsi_block_device(iqn: &str, portal: &str, lun: u32) -> Option<std::path::PathBuf> {
    let pattern = format!("ip-{portal}-iscsi-{iqn}-lun-{lun}");
    let mut entries = tokio::fs::read_dir("/dev/disk/by-path").await.ok()?;
    while let Ok(Some(e)) = entries.next_entry().await {
        if e.file_name().to_string_lossy() == pattern {
            return Some(e.path());
        }
    }
    None
}
```

- [ ] **Step 3: Run — pure-logic tests pass**

Run: `cargo test -p agent session_tests`
Expected: 2 passing. (Live tests gated on env var, ignored.)

- [ ] **Step 4: Commit**

```bash
git add apps/agent/src/features/storage/iscsi_lvm.rs
git commit -m "feat(agent): iscsi session helpers for iscsi_lvm backend"
```

---

### Task 6: Agent — VG initialize (`pvcreate` + `vgcreate`)

The destructive one-time setup. Mirrors `LVMPlugin::lvm_create_volume_group`.

**Files:**
- Modify: `apps/agent/src/features/storage/iscsi_lvm.rs`

- [ ] **Step 1: Write the test for VG init arg construction**

```rust
#[test]
fn pvcreate_args_use_proxmox_metadata_size() {
    // Proxmox uses --metadatasize 250k for 128k pe_start alignment (SSD-friendly).
    let args = build_pvcreate_args("/dev/sdb");
    assert_eq!(args, vec!["--metadatasize", "250k", "/dev/sdb"]);
}

#[test]
fn vgcreate_args_minimal() {
    let args = build_vgcreate_args("vg-nqrust", "/dev/sdb");
    assert_eq!(args, vec!["vg-nqrust", "/dev/sdb"]);
}
```

- [ ] **Step 2: Implement**

```rust
fn build_pvcreate_args(device: &str) -> Vec<&str> {
    // Mirrors LVMPlugin.pm:120 — metadatasize 250k aligns pe_start to 128k for SSDs.
    vec!["--metadatasize", "250k", device]
}

fn build_vgcreate_args<'a>(vg: &'a str, device: &'a str) -> Vec<&'a str> {
    vec![vg, device]
}

pub async fn initialize_vg(device: &std::path::Path, vg_name: &str) -> Result<(), StorageError> {
    // Idempotency: check if device already has a VG.
    let pv_check = tokio::process::Command::new("pvs")
        .args(["--separator", ":", "--noheadings", "--units", "k",
               "--unbuffered", "--nosuffix", "--options",
               "pv_name,pv_size,vg_name,pv_uuid", device.to_str().unwrap_or("")])
        .output().await.ok();
    if let Some(out) = pv_check {
        if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
            if let Some(info) = parse_pv_info(line) {
                if let Some(existing_vg) = info.vg_name {
                    if existing_vg == vg_name {
                        // already initialized; return ok.
                        return Ok(());
                    } else {
                        return Err(StorageError::backend(std::io::Error::other(format!(
                            "device {} is already part of VG '{}'; refusing to overwrite",
                            device.display(), existing_vg
                        ))));
                    }
                }
            }
        }
    }
    // Zero the first sector before pvcreate (LVMPlugin.pm:96-103).
    let mut f = tokio::fs::OpenOptions::new().write(true).open(device).await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("open {} for zero: {e}", device.display()))))?;
    use tokio::io::AsyncWriteExt;
    f.write_all(&[0u8; 512]).await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("zero first sector: {e}"))))?;
    drop(f);
    // pvcreate
    let status = tokio::process::Command::new("pvcreate")
        .args(&build_pvcreate_args(device.to_str().unwrap_or("")))
        .status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("pvcreate spawn: {e}"))))?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!("pvcreate failed: exit {:?}", status.code()))));
    }
    // vgcreate
    let status = tokio::process::Command::new("vgcreate")
        .args(&build_vgcreate_args(vg_name, device.to_str().unwrap_or("")))
        .status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("vgcreate spawn: {e}"))))?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!("vgcreate failed: exit {:?}", status.code()))));
    }
    Ok(())
}
```

- [ ] **Step 3: Run — args tests pass**

Run: `cargo test -p agent`
Expected: all parser + session + arg tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/agent/src/features/storage/iscsi_lvm.rs
git commit -m "feat(agent): VG initialize via pvcreate + vgcreate (idempotent)"
```

---

### Task 7: Agent — LV lifecycle (`lvcreate`, `lvremove`, `lvchange`, `lvextend`)

Mirrors `LVMPlugin::lvcreate`, `free_lvm_volumes`, `activate_volume`, `volume_resize`.

**Files:**
- Modify: `apps/agent/src/features/storage/iscsi_lvm.rs`

- [ ] **Step 1: Write tests for arg construction**

```rust
#[test]
fn lvcreate_args_match_proxmox_shape() {
    let args = build_lvcreate_args("vg-nqrust", "vm-100-disk-0", "10737418240B", &["nqrust-vm-100"]);
    let s = args.join(" ");
    // From LVMPlugin.pm:622-637 — the exact flags Proxmox uses.
    assert!(s.contains("-aly"));         // activate immediately
    assert!(s.contains("-Wy"));           // wipe signatures
    assert!(s.contains("--yes"));         // assume yes to prompts
    assert!(s.contains("--size 10737418240B"));
    assert!(s.contains("--name vm-100-disk-0"));
    assert!(s.contains("--setautoactivation n"));
    assert!(s.contains("--addtag nqrust-vm-100"));
    assert!(s.contains("vg-nqrust"));
}

#[test]
fn lvchange_activate_uses_exclusive_mode() {
    // From LVMPlugin.pm:960 — `-aey` (exclusive activation) is the safety
    // mechanism for shared LVM. `-aly` would allow multiple hosts active.
    let args = build_lvchange_activate_args("vg-nqrust", "vm-100-disk-0");
    let s = args.join(" ");
    assert!(s.contains("-aey"));
    assert!(s.contains("/dev/vg-nqrust/vm-100-disk-0"));
}

#[test]
fn lvchange_deactivate_args() {
    let args = build_lvchange_deactivate_args("vg-nqrust", "vm-100-disk-0");
    let s = args.join(" ");
    assert!(s.contains("-aln"));
}
```

- [ ] **Step 2: Implement arg builders + functions**

```rust
fn build_lvcreate_args<'a>(vg: &'a str, name: &'a str, size: &'a str, tags: &'a [&'a str]) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-aly".into(), "-Wy".into(), "--yes".into(),
        "--size".into(), size.into(),
        "--name".into(), name.into(),
        "--setautoactivation".into(), "n".into(),
    ];
    for t in tags {
        a.push("--addtag".into());
        a.push(t.to_string());
    }
    a.push(vg.into());
    a
}

fn build_lvchange_activate_args(vg: &str, lv: &str) -> Vec<String> {
    vec!["-aey".into(), format!("/dev/{vg}/{lv}")]
}

fn build_lvchange_deactivate_args(vg: &str, lv: &str) -> Vec<String> {
    vec!["-aln".into(), format!("/dev/{vg}/{lv}")]
}

pub async fn lvcreate(vg: &str, name: &str, size_bytes: u64, vmid: &str) -> Result<std::path::PathBuf, StorageError> {
    let size_arg = format!("{size_bytes}B");
    let tag = format!("nqrust-vm-{vmid}");
    let tags = vec![tag.as_str()];
    let args = build_lvcreate_args(vg, name, &size_arg, &tags);
    let status = tokio::process::Command::new("lvcreate")
        .args(&args).status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("lvcreate spawn: {e}"))))?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!("lvcreate failed: exit {:?}", status.code()))));
    }
    Ok(std::path::PathBuf::from(format!("/dev/{vg}/{name}")))
}

pub async fn lvremove(vg: &str, name: &str) -> Result<(), StorageError> {
    let path = format!("{vg}/{name}");
    let status = tokio::process::Command::new("lvremove")
        .args(["-f", &path]).status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("lvremove spawn: {e}"))))?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!("lvremove failed: exit {:?}", status.code()))));
    }
    Ok(())
}

pub async fn lvchange_activate(vg: &str, lv: &str) -> Result<(), StorageError> {
    let args = build_lvchange_activate_args(vg, lv);
    let status = tokio::process::Command::new("lvchange")
        .args(&args).status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("lvchange activate spawn: {e}"))))?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!("lvchange activate failed: exit {:?}", status.code()))));
    }
    // --refresh after activate (LVMPlugin.pm:970)
    tokio::process::Command::new("lvchange")
        .args(["--refresh", &format!("/dev/{vg}/{lv}")])
        .status().await.ok();
    Ok(())
}

pub async fn lvchange_deactivate(vg: &str, lv: &str) -> Result<(), StorageError> {
    let args = build_lvchange_deactivate_args(vg, lv);
    let status = tokio::process::Command::new("lvchange")
        .args(&args).status().await
        .map_err(|e| StorageError::backend(std::io::Error::other(format!("lvchange deactivate spawn: {e}"))))?;
    if !status.success() {
        // Deactivate is best-effort; log but don't fail.
        tracing::warn!("lvchange deactivate failed: exit {:?}", status.code());
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p agent`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add apps/agent/src/features/storage/iscsi_lvm.rs
git commit -m "feat(agent): LV lifecycle helpers (create/remove/activate/deactivate)"
```

---

### Task 8: Agent HTTP routes for iscsi_lvm

Mirror the NFS route shape we already shipped: per-operation POST endpoints.

**Files:**
- Modify: `apps/agent/src/features/storage/iscsi_lvm.rs` (add `pub fn router`)
- Modify: `apps/agent/src/features/storage/mod.rs` (mount under `/v1/storage/iscsi_lvm`)
- Modify: `apps/agent/src/main.rs` (register `IscsiLvmHostBackend` if iSCSI tools exist)

- [ ] **Step 1: Write request/response structs and route handlers**

```rust
// apps/agent/src/features/storage/iscsi_lvm.rs

#[derive(Deserialize)]
pub struct LoginReq { pub iqn: String, pub portal: String }

#[derive(Deserialize)]
pub struct InitVgReq { pub iqn: String, pub portal: String, pub lun: u32, pub vg_name: String }

#[derive(Deserialize)]
pub struct LvCreateReq { pub vg: String, pub name: String, pub size_bytes: u64, pub vm_id: String }

#[derive(Serialize)]
pub struct LvCreateResp { pub device: String }

#[derive(Deserialize)]
pub struct LvRemoveReq { pub vg: String, pub name: String }

#[derive(Deserialize)]
pub struct LvActivateReq { pub vg: String, pub name: String }

#[derive(Serialize)]
pub struct VgStatusResp { pub size_bytes: u64, pub free_bytes: u64, pub lv_count: u32 }

#[derive(Deserialize)]
pub struct VgStatusReq { pub vg: String }

#[derive(Deserialize)]
pub struct CloneFromPathReq { pub source_path: String, pub vg: String, pub name: String }

#[derive(Deserialize)]
pub struct LvSnapshotReq { pub vg: String, pub source_lv: String, pub snap_name: String, pub size_bytes: u64 }

pub fn router() -> axum::Router {
    use axum::routing::post;
    axum::Router::new()
        .route("/login", post(login_handler))
        .route("/init_vg", post(init_vg_handler))
        .route("/vg_status", post(vg_status_handler))
        .route("/lv_create", post(lv_create_handler))
        .route("/lv_remove", post(lv_remove_handler))
        .route("/lv_activate", post(lv_activate_handler))
        .route("/lv_deactivate", post(lv_deactivate_handler))
        .route("/clone_from_path", post(clone_from_path_handler))
        .route("/lv_snapshot", post(lv_snapshot_handler))
}
```

For each handler, the body is:

1. Parse the request struct.
2. Call the corresponding helper.
3. Return JSON success or `(StatusCode, Json({"error": ...}))`.

(Mirror exactly what the NFS routes do — same shape, same error formatting.)

- [ ] **Step 2: Wire the router**

```rust
// apps/agent/src/features/storage/mod.rs
pub fn router() -> Router {
    Router::new()
        .nest("/nfs", nfs::router())
        .nest("/iscsi_lvm", iscsi_lvm::router())
        // existing routes...
}
```

- [ ] **Step 3: Register the host-backend in main.rs**

```rust
// apps/agent/src/main.rs — add after the NFS registration
if std::env::var("AGENT_ISCSI_AVAILABLE").is_ok() ||
   tokio::fs::metadata("/usr/bin/iscsiadm").await.is_ok() {
    storage_registry.register_for(
        nexus_storage::BackendKind::IscsiLvm,
        std::sync::Arc::new(features::storage::iscsi_lvm::IscsiLvmHostBackend),
    );
}
```

- [ ] **Step 4: Build to confirm**

Run: `cargo build --release -p agent`
Expected: builds cleanly.

- [ ] **Step 5: Commit**

```bash
git add apps/agent/src/features/storage/
git commit -m "feat(agent): HTTP routes for iscsi_lvm backend"
```

---

### Task 9: Manager — `IscsiLvmConfig` + control-plane backend

**Files:**
- Create: `apps/manager/src/features/storage/backends/iscsi_lvm.rs`
- Modify: `apps/manager/src/features/storage/backends/mod.rs` (add `pub mod iscsi_lvm;`)

- [ ] **Step 1: Define config + struct**

```rust
// apps/manager/src/features/storage/backends/iscsi_lvm.rs

use anyhow::Context as _;
use async_trait::async_trait;
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct IscsiLvmConfig {
    pub portal: String,         // "192.168.18.171:3260"
    pub iqn: String,            // "iqn.2005-10.org.freenas.ctl:vmstore"
    pub vg_name: String,        // "vg-nqrust"
    pub lun: u32,               // typically 0
    #[serde(default)]
    pub saferemove: bool,       // zero-out LV on free
    pub agent_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IscsiLvmLocator {
    pub vg: String,
    pub lv: String,
}

pub struct IscsiLvmControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: IscsiLvmConfig,
}
```

- [ ] **Step 2: Implement helper to call agent**

Mirror the NFS pattern: `agent_post<Req, Resp>(&self, endpoint: &str, body: &Req) -> Result<Resp, StorageError>`.

- [ ] **Step 3: Implement the trait**

Each method calls the corresponding agent endpoint:

| Trait method | Agent call |
|---|---|
| `provision(opts)` | POST `/iscsi_lvm/lv_create` with `{vg, name: "nqrust-vm-<vmid>-disk-<rand>", size_bytes, vm_id}` |
| `destroy(handle)` | POST `/iscsi_lvm/lv_remove` |
| `clone_from_image(src, opts)` | POST `/iscsi_lvm/lv_create` then POST `/iscsi_lvm/clone_from_path` |
| `snapshot(volume, name)` | POST `/iscsi_lvm/lv_snapshot` |
| `clone_from_snapshot(snap)` | POST `/iscsi_lvm/lv_create` (with --snapshot of orig) |
| `delete_snapshot(snap)` | POST `/iscsi_lvm/lv_remove` (snapshots are LVs too) |
| `probe()` | POST `/iscsi_lvm/login` then `/iscsi_lvm/vg_status` |
| `host_path_for(handle)` | parse `IscsiLvmLocator`, return `Some("/dev/<vg>/<lv>")` |
| `activate_volume(handle)` | POST `/iscsi_lvm/lv_activate` |
| `deactivate_volume(handle)` | POST `/iscsi_lvm/lv_deactivate` |
| `capabilities()` | `supports_clone_from_image: true, supports_native_snapshots: true, supports_concurrent_attach: false, supports_live_migration: true` |

- [ ] **Step 4: Add inline tests for config validation, locator parsing, and capabilities**

- [ ] **Step 5: Build + test**

Run: `cargo build --release -p manager && cargo test -p manager iscsi_lvm`
Expected: builds + tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/manager/src/features/storage/backends/iscsi_lvm.rs apps/manager/src/features/storage/backends/mod.rs
git commit -m "feat(manager): iscsi_lvm control-plane backend"
```

---

### Task 10: Manager — wire iscsi_lvm into registry + config validate + health

**Files:**
- Modify: `apps/manager/src/features/storage/registry.rs`
- Modify: `apps/manager/src/features/storage/config.rs`
- Modify: `apps/manager/src/features/storage_backends/health.rs`

- [ ] **Step 1: Add validate arm in config.rs**

```rust
// validate iscsi_lvm requires portal, iqn, vg_name
BackendKind::IscsiLvm => {
    require_field(&raw.config, "portal")?;
    require_field(&raw.config, "iqn")?;
    require_field(&raw.config, "vg_name")?;
    Ok(ValidatedBackend { /* ... */ })
}
```

- [ ] **Step 2: Add build_backend arm in registry.rs**

Same shape as NFS: deserialize config, fill agent_url default, construct `IscsiLvmControlPlaneBackend`.

- [ ] **Step 3: Add health probe arm in health.rs**

Probe: `iscsiadm -m session` → check session for IQN; if alive, run `vgs --units b --options vg_name,vg_size,vg_free` parsed and return `(used, total)`.

- [ ] **Step 4: Build + test**

Run: `cargo build --release -p manager`
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add apps/manager/src/features/storage/registry.rs apps/manager/src/features/storage/config.rs apps/manager/src/features/storage_backends/health.rs
git commit -m "feat(manager): wire iscsi_lvm into registry/config/health"
```

---

### Task 11: Manager — POST `/v1/storage_backends/:id/initialize` for one-time VG setup

This is the destructive `pvcreate + vgcreate` step. Separate route so the UI can put a "this will wipe data" confirmation in front of it.

**Files:**
- Create: `apps/manager/src/features/storage_backends/initialize.rs`
- Modify: `apps/manager/src/features/storage_backends/mod.rs` (register route)

- [ ] **Step 1: Define handler**

```rust
// apps/manager/src/features/storage_backends/initialize.rs

#[derive(Deserialize)]
pub struct InitializeReq {
    pub confirm: String, // must equal "I understand this wipes the LUN"
}

pub async fn initialize(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<InitializeReq>,
) -> impl IntoResponse {
    if req.confirm != "I understand this wipes the LUN" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing confirm phrase"}))).into_response();
    }

    let repo = StorageBackendRepository::new(st.db.clone());
    let row = match repo.get(id).await { /* ... */ };
    if row.kind != "iscsi_lvm" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "initialize only valid for iscsi_lvm"}))).into_response();
    }

    // Build IscsiLvmControlPlaneBackend, call backend.initialize_vg() (a new
    // method on the inherent impl, not the trait). The agent does pvcreate+vgcreate.
    // ...
}
```

- [ ] **Step 2: Add route in mod.rs**

```rust
.route("/:id/initialize", post(initialize::initialize))
```

- [ ] **Step 3: Build + test**

Run: `cargo build --release -p manager`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add apps/manager/src/features/storage_backends/initialize.rs apps/manager/src/features/storage_backends/mod.rs
git commit -m "feat(manager): POST /v1/storage_backends/:id/initialize for VG setup"
```

---

### Task 12: Manager — service.rs activate/deactivate hooks in VM lifecycle

**Files:**
- Modify: `apps/manager/src/features/vms/service.rs`

- [ ] **Step 1: Locate the spawn-firecracker site**

Search for `spawn_firecracker` call. Just *before* it, add:

```rust
// Activate the rootfs volume on this host. For backends that share a
// LUN across hosts (iscsi_lvm), this issues `lvchange -aey` so this
// host gets exclusive access. No-op for local_file/NFS.
if let Some(backend_id) = req.backend_id {
    if let Some(backend) = st.registry.get(backend_id) {
        backend.activate_volume(&volume_handle).await
            .with_context(|| format!("activating volume on backend {backend_id}"))?;
    }
}
```

- [ ] **Step 2: Locate the VM-stop path**

In the stop handler (find via `grep -n "stop_vm\|kill_firecracker"`), add the deactivate after firecracker exits:

```rust
if let Some(backend) = st.registry.get(backend_id) {
    let _ = backend.deactivate_volume(&volume_handle).await; // best-effort
}
```

- [ ] **Step 3: Build + restart manager + spot-check with NFS VM (no behavior change expected)**

Expected: NFS VM still creates and runs; activate/deactivate are no-ops.

- [ ] **Step 4: Commit**

```bash
git add apps/manager/src/features/vms/service.rs
git commit -m "feat(manager): activate_volume/deactivate_volume hooks in VM lifecycle"
```

---

### Task 13: Manager — `host_path_for` arm for iscsi_lvm

Already added to the trait in Task 2 with default. Override in `IscsiLvmControlPlaneBackend` (Task 9) to return `/dev/<vg>/<lv>`. Verify the existing `provision_rootfs` call site (`vms/service.rs`) routes through `backend.host_path_for(&handle)` we wired during the NFS work.

- [ ] **Step 1: Verify the call site uses the trait method**

Run: `grep -n "host_path_for" apps/manager/src/features/vms/service.rs`
Expected: at least one site using it (added during NFS task).

- [ ] **Step 2: Confirm iscsi_lvm impl returns /dev path** (already in Task 9's impl).

- [ ] **Step 3: Skip-commit if no change needed**

---

### Task 14: UI — `iscsi_lvm` form fields in BackendCreateDialog

**Files:**
- Modify: `apps/ui/components/storage/backend-create-dialog.tsx`

- [ ] **Step 1: Add field schema for iscsi_lvm**

```ts
const iscsiLvmFields: Field[] = [
  { name: "portal", label: "iSCSI Portal (host:port)", required: true, placeholder: "192.168.1.10:3260" },
  { name: "iqn", label: "Target IQN", required: true, placeholder: "iqn.2005-10.org.freenas.ctl:vmstore" },
  { name: "vg_name", label: "Volume Group Name", required: true, placeholder: "vg-nqrust" },
  { name: "lun", label: "LUN", required: false, placeholder: "0", advanced: true },
  { name: "saferemove", label: "Zero-out on delete (slower but secure)", type: "boolean", advanced: true },
];
```

- [ ] **Step 2: Hook into the kind switch**

When `kind === "iscsi_lvm"`, use `iscsiLvmFields` for the form.

- [ ] **Step 3: After successful create, show "Initialize Volume Group" CTA**

The backend exists in DB but the LUN isn't formatted yet. UI flow:

1. POST `/v1/storage_backends` returns the row.
2. UI shows: "Backend added. The LUN must be initialized as an LVM Volume Group before it can host VMs. **Initialize now?**"
3. Click → confirmation dialog (Task 15) → POST `/v1/storage_backends/:id/initialize`.

- [ ] **Step 4: Manual UI verification + commit**

```bash
git add apps/ui/components/storage/backend-create-dialog.tsx
git commit -m "feat(ui): iscsi_lvm form fields in backend create dialog"
```

---

### Task 15: UI — Initialize VG dialog (destructive confirmation)

**Files:**
- Create: `apps/ui/components/storage/lvm-initialize-dialog.tsx`
- Modify: `apps/ui/lib/queries.ts`

- [ ] **Step 1: Add the mutation hook**

```ts
// apps/ui/lib/queries.ts
export function useInitializeBackend() {
  return useMutation({
    mutationFn: async ({ id, confirm }: { id: string; confirm: string }) => {
      return api.post(`/storage_backends/${id}/initialize`, { confirm });
    },
  });
}
```

- [ ] **Step 2: Create the dialog**

Type-to-confirm pattern: user must type the exact phrase "I understand this wipes the LUN". Submit button is disabled until they type it.

- [ ] **Step 3: Wire into BackendTable + post-create flow**

Show "Initialize" button on iscsi_lvm rows that don't have a healthy VG (capacity_total == 0 or health.status == "needs_init"). The health probe (Task 10) reports this state.

- [ ] **Step 4: Commit**

```bash
git add apps/ui/components/storage/lvm-initialize-dialog.tsx apps/ui/lib/queries.ts
git commit -m "feat(ui): destructive Initialize VG dialog with type-to-confirm"
```

---

### Task 16: UI — show iscsi_lvm in VM-create wizard storage step

Since the backend appears in `/v1/storage_backends` like all others, the existing dropdown should pick it up automatically. Verify this is the case — no code change should be needed.

- [ ] **Step 1: Manual: create iscsi_lvm backend via UI, then go to VM create → Storage step → confirm dropdown shows it.**

- [ ] **Step 2: If it doesn't show up, find the dropdown filter and remove the kind allowlist.**

- [ ] **Step 3: Commit only if change needed.**

---

### Task 17: Documentation — runbook + plan reference

**Files:**
- Create: `docs/runbooks/iscsi-lvm-troubleshooting.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Write the runbook covering:**

- "Initialize failed — what now?" (most common: LUN already has data; recovery: `wipefs -a <device>` then retry)
- "Session keeps disconnecting" (multipath setup, `node.startup` should be `automatic`)
- "VG free space wrong" (run `vgreduce --removemissing`, restart agent)
- "lvremove says LV in use" (forgot to deactivate; `lvchange -aln` then retry)
- "Two managers fighting" (Postgres advisory lock works; check pg_locks)

- [ ] **Step 2: CHANGELOG entry**

```markdown
## Unreleased

### Added
- `iscsi_lvm` storage backend: vendor-agnostic auto-provisioning of per-VM
  block devices on top of any iSCSI target. Mirrors Proxmox VE's
  LVM-on-iSCSI mode. See `docs/runbooks/iscsi-lvm-troubleshooting.md`.
```

- [ ] **Step 3: Commit**

```bash
git add docs/runbooks/iscsi-lvm-troubleshooting.md CHANGELOG.md
git commit -m "docs: iscsi_lvm runbook + changelog entry"
```

---

### Task 18: End-to-end live verification

Requires a real iSCSI target. For testing here, we'll use TrueNAS at `192.168.18.171` with a freshly-created LUN.

**Pre-requisites:**
- One-time TrueNAS setup: create a 50G zvol → expose as iSCSI target with one LUN.
- Agent host has `open-iscsi` (`sudo pacman -S open-iscsi` on Arch) + `lvm2`.

- [ ] **Step 1: Create the backend via UI**

Storage → Add Backend → kind=iscsi_lvm → portal=192.168.18.171:3260, iqn=<the new IQN>, vg_name=vg-test → submit.
Expected: row appears with status indicator showing "Needs initialization."

- [ ] **Step 2: Initialize**

Click Initialize → type confirm phrase → submit.
Expected: progress, then green dot. `vgs` on agent host shows `vg-test`.

- [ ] **Step 3: Create a VM on iscsi_lvm**

VMs → Create → pick the kernel + alpine rootfs → Storage step → pick the iscsi_lvm backend → submit.
Expected: VM enters running state. `lvs vg-test` on host shows `vm-<vmid>-disk-<random>`.

- [ ] **Step 4: VM stop, then restart**

Stop the VM → `lvs vg-test --options lv_attr` shows the LV is **not** active.
Start the VM → `lvs` shows it active again.

- [ ] **Step 5: Snapshot via UI / API**

Take a snapshot → verify `lvs vg-test` now has the snap LV alongside the source.

- [ ] **Step 6: Delete VM**

Delete → expected: LV gone from `lvs vg-test`.

- [ ] **Step 7: Restart manager + agent → verify backend still initialized + iSCSI session reconnects automatically**

- [ ] **Step 8: If all 7 steps pass, finalize**

```bash
git push origin feature/iscsi-lvm
gh pr create --title "feat(storage): iscsi_lvm backend (Proxmox-equivalent)" --body "$(cat <<'EOF'
## Summary
- Vendor-agnostic per-VM block-device auto-provisioning on top of any iSCSI target
- New `BackendKind::IscsiLvm`, `activate_volume`/`deactivate_volume` trait methods
- Agent owns iSCSI session lifecycle (persistent across restarts) + VG metadata + LV operations
- One-time "Initialize VG" UX with destructive confirmation
- Mirrors Proxmox VE's `LVMPlugin.pm` + `ISCSIPlugin.pm` patterns exactly

## Test Plan
- [x] cargo test -p manager -p agent — all green
- [x] Live test: TrueNAS LUN → init → VM create → start → snapshot → stop → delete
- [x] Restart manager + agent → iSCSI session reconnects, backend still healthy
- [x] Probe failure: try init on a LUN that already has a VG → clear error in UI
EOF
)"
```

---

## Self-Review Checklist (before opening PR)

- [ ] Every task is self-contained — engineer reading task N can implement it without reading N-1's full text
- [ ] All commands shown verbatim with `run_command` style
- [ ] No "TBD" / "TODO in plan" markers (all the actual code is in the plan)
- [ ] Type signatures match across tasks (`IscsiLvmConfig` shape stays consistent)
- [ ] Trait additions in Task 2 match the call sites in Task 9 + Task 12
- [ ] Migration in Task 3 doesn't break existing rows (verified via test insert/delete)
- [ ] Reference points back to specific Proxmox source line numbers where we copy a pattern

## Reference Cross-Map: Proxmox source → our task

| Proxmox file | Lines | What we copy | Our task |
|---|---|---|---|
| `LVMPlugin.pm` | 106–133 | `lvm_create_volume_group` (pvcreate + vgcreate flags) | Task 6 |
| `LVMPlugin.pm` | 615–640 | `lvcreate` arg shape (-aly -Wy --yes --setautoactivation n) | Task 7 |
| `LVMPlugin.pm` | 736–745 | `alloc_image` (find_free_diskname + lvcreate) | Task 9 (provision) |
| `LVMPlugin.pm` | 769–794 | `free_image` (lvremove with optional zero-out) | Task 7 + Task 9 (destroy) |
| `LVMPlugin.pm` | 924–946 | `activate_storage` (lazy, no vgchange -aly) | Task 6 (idempotent init) |
| `LVMPlugin.pm` | 955–972 | `activate_volume` (lvchange -aey + --refresh) | Task 7 + Task 12 |
| `LVMPlugin.pm` | 974–986 | `deactivate_volume` (lvchange -aln) | Task 7 + Task 12 |
| `LVMPlugin.pm` | 988–1015 | `volume_resize` (lvextend) | future task (not in scope) |
| `LVMPlugin.pm` | 478–500 | `on_add_hook` (link iscsi → pv → vg) | Task 11 (initialize) |
| `ISCSIPlugin.pm` | 161–189 | `iscsi_login` (discovery + login + retry config) | Task 5 |
| `ISCSIPlugin.pm` | 38–64 | `iscsi_session_list` (parse `iscsiadm -m session`) | Task 10 (health probe) |

## Out of Scope (explicit punts)

These belong in follow-up plans, not this one:

1. **Live VM migration** — needs the activate/deactivate hooks (this plan adds them) but also network-level VM state transfer; separate plan.
2. **Multi-path iSCSI** — single-path only here. Multi-path is a separate operational layer.
3. **Volume resize via UI** — backend supports `lvextend` but no UI button yet.
4. **Snapshot UI** — backend supports snapshots; surfacing them in the VM detail page is a separate snapshot-UI plan.
5. **Postgres advisory locks for HA** — single-manager setup is safe by serialization; we don't need them yet.
6. **Cluster locking like Proxmox's `cluster_lock_storage`** — same as advisory locks; deferred until multi-manager.
7. **Per-vendor adapters** (NetApp, Pure, etc.) — `iscsi_lvm` covers them all generically; per-vendor only if a user asks for vendor-specific features (snapshots, dedup, replication).
