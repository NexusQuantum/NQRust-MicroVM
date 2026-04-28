# NQRust-MicroVM: Pluggable Storage Architecture

**Status:** Design
**Date:** 2026-04-28
**Owner:** kleopasevan
**Scope:** Foundation PR for pluggable storage backends. Unlocks but does not include SPDK, Ceph RBD, NFS, dedup'd snapshot pipeline, live migration, replication.

## Context

`apps/manager/src/features/storage/mod.rs` currently hardcodes local files:

- `alloc_rootfs`: `fs::copy` from `/srv/images/...` into `/srv/fc/vms/<uuid>/storage/rootfs-*.ext4`, then `e2fsck` + `resize2fs`
- `alloc_data_disk`: `File::create().set_len()` for sparse raw
- Volume registry in `0017_volumes.sql` with `path TEXT` + `host_id NOT NULL`
- VMs are pinned to one host; no shared/replicated/external storage
- Functions and containers bypass `LocalStorage` entirely and `cp` directly under hardcoded `/srv/images/{functions,containers}/<vm-id>.ext4`

This is a fine baseline but blocks every higher-order storage feature we want (mobility, HA, external SAN, snapshot/backup pipeline, SPDK, clustering).

## Intent

Replace the hardcoded local-file path with a **pluggable storage backend abstraction**. After this work, "where the bytes live" is a swappable backend chosen per-volume, configured per-cluster. The control plane talks to traits, never to filesystems or storage protocols directly.

This is the **foundation PR**. It unlocks — but does not include — SPDK, NVMe-oF, Ceph RBD, NFS, the dedup'd S3 snapshot pipeline, distributed replication, live migration, and clustered control plane. Those are additive work on top of the abstraction this PR establishes.

## Why this matters

- **Today**: VM = local file = pinned host. Lose host, lose VM. Cannot integrate customer storage.
- **After this PR**: VM = volume on backend X. Backend X can be local (default, current behavior preserved), or iSCSI (TrueNAS, generic SAN). Future backends slot in without touching the control plane.
- **Strategic**: Government/enterprise customers will arrive with existing Pure Storage, Ceph, NetApp, TrueNAS. We need to consume those as block storage. Without this abstraction, every integration is a special case that rots the codebase.
- **Forward-compatible with clustering**: the trait split chosen below puts a clean seam where future distributed coordination (leader election, raft) inserts in front of provisioning, without disturbing host-local attach/detach.

## Scope

### In this PR

1. **Two traits** (manager-side and agent-side) plus supporting types: `ControlPlaneBackend`, `HostBackend`, `VolumeHandle`, `VolumeSnapshotHandle`, `AttachedPath`, `Capabilities`, `BackendKind`, `CreateOpts`, `StorageError`.
2. **Schema migration** for multi-backend volumes:
   - New `storage_backend` table (id, name, kind, config_json, capabilities_json, is_default, created_at, deleted_at).
   - `volume.host_id` becomes nullable; semantics documented as "home host for host-pinned volumes; NULL for network-attached."
   - `volume.backend_id UUID NOT NULL` with backfill to a `localfile-default` row.
   - `volume_attachment.detached_at TIMESTAMPTZ` + partial unique index `UNIQUE (volume_id) WHERE detached_at IS NULL` to enforce single-attach cluster-wide.
   - Existing rows become `LocalFile` volumes — zero data migration pain for existing deployments.
3. **`LocalFileBackend`** — preserves current behavior exactly. Refactor, not feature change. Existing VMs and existing tests must keep passing.
4. **`IscsiBackend`** — generic iSCSI initiator (`open-iscsi` userspace) on the host side, plus a TrueNAS variant whose control-plane half provisions LUNs via TrueNAS REST API. Proves the abstraction holds for a non-trivial second backend and exercises both halves of the split.
5. **Backend registry + config loading** from `nqrust.toml` on manager startup. TOML is source of truth; `INSERT … ON CONFLICT (name) DO UPDATE` upserts into `storage_backend`. TOML removal triggers soft delete (`deleted_at`); existing volumes still resolve. Multiple backends configurable simultaneously, including multiple instances of the same kind (e.g., `truenas-prod`, `truenas-dr`).
6. **VM lifecycle wired through the traits**: `vm create`, `vm delete`, `vm start`. Firecracker drive config is derived from whatever `AttachedPath` the host backend returns.
7. **API + UI for backend selection.**
   - `POST /v1/vms` and `POST /v1/volumes` accept `backend_id: Option<Uuid>`. Default = the row flagged `is_default = true`.
   - New manager feature module `apps/manager/src/features/storage_backends/{mod,routes,repo}.rs` exposing `GET /v1/storage_backends` and `GET /v1/storage_backends/:id` (read-only; CUD via TOML).
   - New shared types in `crates/nexus-types`: `StorageBackend`, `BackendKind`, `Capabilities` (all `ToSchema`-derived for OpenAPI).
   - UI: `useStorageBackends()` hook in `apps/ui/lib/queries.ts`; `BackendSelector` component; dropdown in VM-create and volume-create forms; hidden when only one backend exists.
8. **Tests** per backend: create → attach → write → detach → delete. Integration test exercising both backends end-to-end. Migration test verifying existing volumes keep booting.

### Explicitly out of scope

Design the trait so these fit cleanly later, but do **not** implement:

- SPDK backend (vhost-user-blk socket should fit `AttachedPath::VhostUserSock` — leave the variant defined, unused)
- NVMe-oF, Ceph RBD, NFS, SMB backends
- Dedup'd snapshot pipeline (FastCDC + BLAKE3 + zstd + S3)
- SeaweedFS integration
- Live migration; cross-host snapshot streaming
- Distributed/replicated storage built in-house
- Clustered control plane (leader election, raft) — the trait split makes this straightforward to add later
- Read-only multi-attach (e.g., shared ISO volumes attached to many VMs) — separate axis, separate PR
- Routing **functions** and **containers** rootfs through the backend abstraction. Hardcoded `/srv/images/{functions,containers}/<vm-id>.ext4` paths stay; runtime images (`container-runtime.ext4`, `python-runtime.ext4`, `bun-runtime.ext4`, `vmlinux-5.10.fc.bin`) stay on local `/srv/images`. A `// TODO(storage-backends): route through StorageBackend trait` comment is added at each call site (`apps/manager/src/features/functions/vm.rs:36` and `apps/manager/src/features/containers/vm.rs:34`).
- VM checkpoint orchestration (memory + state + disk together). VM checkpoint code in `apps/agent/src/features/vm/snapshot.rs` is unchanged. Future composition: `vm_checkpoint(vm_id) = VmCheckpoint(memfile + state) + VolumeSnapshot(every_attached_volume)`.

## Architectural intent (constraints, not implementation)

### The trait abstracts three things that vary between backends

1. **Provisioning** — how a volume comes into existence (file create, LUN provision via REST API, RBD create, etc.). Manager-side concern.
2. **Attachment** — how a host gains access to the bytes. Agent-side concern. The return type accommodates: a file path, a block device path, and (reserved for SPDK later) a vhost-user-blk socket path. Firecracker consumes all three the same way at the drive config level.
3. **Volume snapshot semantics** — some backends have native instant snapshots (ZFS, Ceph, Pure, SPDK lvol, TrueNAS), some don't (raw NFS files). The trait exposes snapshot as a method; backends without native snapshots can implement a slow correct fallback (e.g., LocalFile → `fs::copy`). `Capabilities.supports_native_snapshots` distinguishes "fast" from "slow."

### Two traits, not one (cluster-ready)

Storage operations physically span manager and agent processes. Modeling that as one trait with a context parameter creates panicky runtime gates; modeling it as two traits is honest and makes the manager/agent boundary explicit.

```rust
// crates/nexus-storage/src/lib.rs (new crate)
pub trait ControlPlaneBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> Capabilities;

    fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError>;
    fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError>;

    /// Optional fast path. None = "use the generic provision+stream fallback."
    fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<Option<VolumeHandle>, StorageError>;

    fn snapshot(&self, volume: &VolumeHandle, name: &str) -> Result<VolumeSnapshotHandle, StorageError>;
    fn restore_snapshot(&self, snap: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError>;
    fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError>;
}

pub trait HostBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    /// Make the volume's bytes accessible to Firecracker on this host.
    fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError>;
    fn detach(&self, volume: &VolumeHandle, attached: AttachedPath) -> Result<(), StorageError>;

    /// Used by the generic populate fallback. Writes raw bytes from `source` into the
    /// already-attached volume. Backend-agnostic implementation: open the AttachedPath
    /// and stream.
    fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError>;
}

pub enum AttachedPath {
    File(PathBuf),                  // LocalFile uses this
    BlockDevice(PathBuf),           // Iscsi uses this (e.g., /dev/disk/by-path/ip-…)
    VhostUserSock(PathBuf),         // Reserved for future SPDK backend; unused this PR
}

pub enum BackendKind {
    LocalFile,
    Iscsi,
    TrueNasIscsi,
    // future: CephRbd, Nfs, Spdk, NvmeOf, …
}

pub struct Capabilities {
    pub supports_native_snapshots: bool,
    pub supports_concurrent_attach: bool,  // false everywhere in this PR
    pub supports_live_migration: bool,     // false everywhere in this PR
    pub supports_clone_from_image: bool,   // true if clone_from_image returns Some
}
```

`LocalFile` implements both traits and runs in both processes for backwards compatibility (the manager process *is* the agent process for single-host deployments). `IscsiBackend` ships as two impls in two crates: `IscsiControlPlaneBackend` linked into the manager binary, `IscsiHostBackend` linked into the agent binary. They share types via the new `nexus-storage` crate.

### Image population — generic streaming with optional fast path

`alloc_rootfs` becomes:

```
1. control_plane.clone_from_image(source, opts)  // try fast path
   → Some(volume) → done
   → None → fall through
2. control_plane.provision(opts) → empty volume
3. host.attach(volume) → AttachedPath
4. host.populate_streaming(attached, source, target_size) → writes bytes
   - LocalFile impl: fs::copy + resize2fs
   - Iscsi impl: open(BlockDevice path), stream image bytes, then resize2fs (in-guest filesystem)
5. (volume stays attached if it's about to be used by a VM that's starting; otherwise detach)
```

For LocalFile, `clone_from_image` returns `Some` (using `fs::copy` directly is faster than detour-through-stream and matches existing behavior bit-for-bit). For Iscsi in this PR, `clone_from_image` returns `None` — generic streaming path is used.

The streaming runs **on the agent that will host the VM**. Image source is agent-local (`/srv/images` is already populated per-agent by the install flow). Cross-host image distribution is a separate problem.

For backends whose `AttachedPath` is `BlockDevice` (Iscsi), `populate_streaming` opens the block device, writes the image bytes, then runs `resize2fs` against the device path so the in-image ext4 filesystem fills the LUN. For `File` backends (LocalFile), the same logic applies but operates on the file path. The host backend abstracts the difference; callers don't branch on `AttachedPath` variants.

Once populated, the volume stays attached if the VM is about to start; otherwise the caller calls `host.detach`.

### Capabilities drives placement decisions (eventually)

`Capabilities` is consumed by the control plane for placement, not just metadata. Even though we're not making placement decisions in this PR, the data is in `storage_backend.capabilities_json` so future code can:

- "VM needs live migration → only schedule on backends with `supports_live_migration`"
- "Instant snapshot requested → check `supports_native_snapshots` before falling back to chunk pipeline"
- "Read-only multi-attach (future) → require `supports_concurrent_attach`"

### Backend identity is per-volume, not per-cluster

A single deployment must support multiple backends simultaneously. `volume.backend_id` records which backend instance the volume belongs to; operations route via the registry on every call.

### `host_id` semantics for new volumes

For LocalFile-backend volumes, the control-plane half sets `volume.host_id` at provision time to the host where the file lives (today: the manager host; future-clustered: the agent that owns the file). For Iscsi/network backends, `host_id` is left NULL. Existing rows are unaffected by this rule because the migration preserves their `host_id` values verbatim.

### VM start lifecycle through the trait

On VM start, the manager iterates the VM's `volume_attachment` rows. For each, it resolves `volume.backend_id` to a `HostBackend` impl on the chosen agent, calls `attach`, and inserts the resulting `AttachedPath` into the Firecracker drive config the manager hands to the agent's spawn flow. On VM stop or delete, the manager calls `detach` for each attachment and updates `volume_attachment.detached_at`. If a VM is starting on the same host where a populate-streaming attach already happened, the existing attachment is reused (no detach/reattach round-trip).

### Single-attach enforced cluster-wide

A second attachment to a volume that already has an active one returns `StorageError::AlreadyAttached`. Enforcement is database-level via the partial unique index on `volume_attachment(volume_id) WHERE detached_at IS NULL`. Detach updates `detached_at` rather than deleting the row — this also gives an attachment audit trail. Capability `supports_concurrent_attach` is false for both backends shipped here. A future read-only multi-attach feature gates on a separate read-only flag and on the capability bit.

### Backwards compatibility is non-negotiable

Every existing volume row keeps working. Every existing VM keeps booting. The migration:

```sql
-- 1. New backends table
CREATE TABLE storage_backend (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  capabilities_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_default BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX one_default_backend ON storage_backend(is_default) WHERE is_default = true;

-- 2. Default localfile backend, marked as default, capabilities populated.
INSERT INTO storage_backend (name, kind, capabilities_json, is_default)
VALUES (
  'localfile-default',
  'local_file',
  '{"supports_native_snapshots": false, "supports_concurrent_attach": false, "supports_live_migration": false, "supports_clone_from_image": true}'::jsonb,
  true
);

-- 3. Backfill existing volumes onto it; relax host_id; require backend_id.
ALTER TABLE volume ADD COLUMN backend_id UUID REFERENCES storage_backend(id);
UPDATE volume SET backend_id = (SELECT id FROM storage_backend WHERE name = 'localfile-default') WHERE backend_id IS NULL;
ALTER TABLE volume ALTER COLUMN backend_id SET NOT NULL;
ALTER TABLE volume ALTER COLUMN host_id DROP NOT NULL;
COMMENT ON COLUMN volume.host_id IS 'Home host for host-pinned volumes (LocalFile). NULL for network-attached backends.';

-- 4. Single-attach enforcement + audit
ALTER TABLE volume_attachment ADD COLUMN detached_at TIMESTAMPTZ;
DROP INDEX IF EXISTS volume_one_active_attachment;
CREATE UNIQUE INDEX volume_one_active_attachment ON volume_attachment(volume_id) WHERE detached_at IS NULL;
```

## Success criteria

- Existing VMs continue to function with no operator action. `cargo test -p manager` and `cargo test -p agent` pass without changes.
- `GET /v1/storage_backends` returns at least the `localfile-default` row on a fresh install.
- New VMs can be created on either `LocalFile` or `Iscsi` backend via API request body or UI selector.
- A volume created on `truenas-iscsi-prod` is reachable from any host that can log into the target — VM mobility is now possible (even though scheduling/migration is not implemented here).
- Adding a future backend (SPDK, Ceph RBD, etc.) requires only: implementing the two traits + adding a TOML config section. Zero changes to the control plane, the VM lifecycle, or any other backend.
- The `AttachedPath` enum covers file, block device, and vhost-user socket — proven by `LocalFile` using `File` and `Iscsi` using `BlockDevice`.
- Attempting to attach an already-attached volume returns 409 with `StorageError::AlreadyAttached`. Verified by integration test.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.

## Non-goals to be explicit about

- Not building distributed/replicated storage ourselves in this PR. iSCSI delegates HA to the storage box (TrueNAS, etc.). Native replication comes later via SPDK + Raft or similar.
- Not building the snapshot/backup pipeline. The `snapshot` trait method exists so backends with native snapshots (TrueNAS ZFS) can implement it now; the chunk-based pipeline that works on backends without native snapshots is a separate future PR.
- Not changing functions or containers storage paths. Their hardcoded `/srv/images/{functions,containers}/<vm-id>.ext4` paths stay. They effectively always use LocalFile regardless of cluster default backend. Routing them through the abstraction deserves its own PR with cold-start benchmarks.
- Not orchestrating VM-checkpoint + volume-snapshot together. VM checkpoint code in `apps/agent/src/features/vm/snapshot.rs` is unchanged.

## Open questions to resolve during implementation

These are tactical decisions the implementer makes; flag them in the PR description. None of them require revisiting the trait shape:

- Whether `IscsiBackend` and `TrueNasIscsiBackend` are one type with a provisioning sub-trait, or two separate types sharing an attach implementation. (Recommendation: separate types; share via composition over a `LunProvisioner` helper trait if it gets repetitive.)
- Where iSCSI session state lives. (Recommendation: rely on `iscsid` daemon on each agent; the `HostBackend` impl runs `iscsiadm` and trusts the daemon for session persistence.)
- How backend errors surface. (Recommendation: backend-specific error types behind a common `StorageError` enum with a `Backend(Box<dyn Error>)` variant for non-categorical failures; use `anyhow` only at the route layer.)
- Lifecycle of the iSCSI session on host failure / VM stop. (Recommendation: detach eagerly on VM stop; keep `iscsid` session if other VMs on the host still need the same target — refcounted at the host backend level.)
- Where TrueNAS API credentials live. (Recommendation: env-var reference in TOML — `api_key_env = "TRUENAS_PROD_API_KEY"` — manager reads env on startup; secrets never in the DB.)
- CHAP authentication for generic iSCSI. (Recommendation: optional fields in TOML config_json, passed through to `iscsiadm`.)

Resolve these as you go; document the choices in the PR description.

## File-level outline of the change

Manager:
- `crates/nexus-storage/` (new) — trait definitions and shared types.
- `apps/manager/src/features/storage/mod.rs` — refactored: `LocalStorage` is renamed/wrapped as `LocalFileBackend` implementing both traits.
- `apps/manager/src/features/storage/backends/local_file.rs` (new) — control-plane half (single-host: same impl, just one process).
- `apps/manager/src/features/storage/backends/iscsi.rs` (new) — control-plane half (TrueNAS REST + generic).
- `apps/manager/src/features/storage_backends/` (new) — `mod.rs`, `routes.rs`, `repo.rs` for `GET /v1/storage_backends*`.
- `apps/manager/src/features/storage/registry.rs` (new) — loads TOML, upserts `storage_backend`, hands out trait objects keyed by `backend_id`.
- `apps/manager/src/features/vms/service.rs` — `create_vm` calls registry → `clone_from_image` or `provision`+`populate_streaming` instead of `LocalStorage::alloc_rootfs`.
- `apps/manager/src/features/volumes/{routes,repo}.rs` — `create` accepts `backend_id`; lookup routes resolve via registry.
- `apps/manager/migrations/0034_storage_backends.sql` (new).

Agent:
- `apps/agent/src/features/storage/mod.rs` (new) — host-backend registry.
- `apps/agent/src/features/storage/backends/local_file.rs` (new) — `HostBackend` impl using local paths.
- `apps/agent/src/features/storage/backends/iscsi.rs` (new) — `HostBackend` impl shelling out to `iscsiadm`.
- `apps/agent/src/features/vm/spawn.rs` — receives `AttachedPath` from manager (new field on spawn request) and feeds it to Firecracker drive config.

Shared:
- `crates/nexus-types/src/lib.rs` — adds `StorageBackend`, `BackendKind`, `Capabilities`, `BackendId` newtype, plus the `backend_id` field on VM and volume create request types.

UI:
- `apps/ui/lib/types/index.ts` — `StorageBackend`, `BackendKind`, `Capabilities`.
- `apps/ui/lib/queries.ts` — `useStorageBackends()`, key `["storage_backends"]`.
- `apps/ui/components/storage/backend-selector.tsx` (new) — dropdown, hides when one backend.
- `apps/ui/components/vm/vm-create-form.tsx` and `apps/ui/components/volume/volume-create-form.tsx` — embed `BackendSelector`.

Configuration:
- `nqrust.toml` gains a `[[storage_backend]]` array section. Example:
  ```toml
  [[storage_backend]]
  name = "localfile-default"
  kind = "local_file"
  is_default = true

  [[storage_backend]]
  name = "truenas-prod"
  kind = "truenas_iscsi"
  is_default = false
  [storage_backend.config]
  endpoint = "https://truenas.internal"
  api_key_env = "TRUENAS_PROD_API_KEY"
  pool = "tank"
  target_iqn_prefix = "iqn.2024-01.com.example:nqrust"
  ```

## Testing strategy

- **Unit tests** per backend impl: provision/destroy round-trip; snapshot/restore round-trip (LocalFile uses tmpdir; Iscsi tests gated behind `--ignored` and require a TrueNAS sim/instance).
- **Integration test** (`apps/manager/tests/storage_abstraction.rs`): boot manager + in-process LocalFile backend; create VM → verify Firecracker config sees correct rootfs path → verify volume_attachment row exists → stop VM → verify detached_at set.
- **Migration test**: load a fixture with pre-migration `volume` rows, run migration, verify `backend_id` populated and `host_id` preserved.
- **Single-attach test**: attempt double-attach, expect `409` + `AlreadyAttached`.
- **No regression**: existing `cargo test -p manager` and `cargo test -p agent` suites pass without modification.

## Glossary

- **VmCheckpoint** — Firecracker memfile + state file written by `apps/agent/src/features/vm/snapshot.rs`. Used for pause/resume of a *running* VM. Unchanged in this PR.
- **VolumeSnapshot** — block-level point-in-time copy of a volume's bytes. The new trait's `snapshot` method creates these. ZFS/RBD/TrueNAS instant; LocalFile slow-but-correct.
- **AttachedPath** — what the host backend hands back to whoever is going to feed it to Firecracker. File path, block device path, or future vhost-user socket path.
- **Backend instance** — a row in `storage_backend`. Identified by `backend_id`. There can be multiple instances of the same kind (`truenas-prod`, `truenas-dr`).
