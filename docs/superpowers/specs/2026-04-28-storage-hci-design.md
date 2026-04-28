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
- **LocalFile volume placement across multiple agents in a clustered deployment.** This PR treats LocalFile as single-host: `volume.host_id` is set to the manager host, and there is no scheduling logic that picks which agent owns a new LocalFile volume. Multi-agent LocalFile placement (round-robin, capacity-aware, operator-pinned) is a future PR concern and depends on the clustered control plane.
- **`rollback_to_snapshot`.** The trait exposes `clone_from_snapshot` (always creates a new volume); rolling an existing volume back to a snapshot's state is a separate operation with different blast radius and is deliberately omitted. Add it only if a concrete need arises.

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

    /// Fast path. Only valid to call when `capabilities().supports_clone_from_image == true`.
    /// Calling it on a backend that doesn't support it returns `StorageError::NotSupported`.
    fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError>;

    fn snapshot(&self, volume: &VolumeHandle, name: &str) -> Result<VolumeSnapshotHandle, StorageError>;

    /// Always creates a NEW volume from the snapshot. Never mutates the source volume.
    /// Backends that have a native rollback primitive (ZFS rollback, RBD snap rollback)
    /// MUST NOT use it here — rollback is a different operation with a different blast
    /// radius and will be added as `rollback_to_snapshot` in a future PR if needed.
    fn clone_from_snapshot(&self, snap: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError>;

    fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError>;
}

pub trait HostBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    /// Make the volume's bytes accessible to Firecracker on this host.
    fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError>;
    fn detach(&self, volume: &VolumeHandle, attached: AttachedPath) -> Result<(), StorageError>;

    /// Pure byte copy: open the AttachedPath, write `source` bytes into it, ensure the
    /// underlying storage is at least `target_size_bytes` (sparse extension OK).
    /// MUST NOT do filesystem-aware operations (no resize2fs, no fsck, no mkfs).
    /// Filesystem-aware steps belong in the rootfs-allocation caller, not the trait —
    /// the trait must remain agnostic to ext4/xfs/btrfs/qcow2/raw-without-fs.
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
    pub supports_clone_from_image: bool,   // gate for calling clone_from_image
}
```

`LocalFile` implements both traits and runs in both processes for backwards compatibility (the manager process *is* the agent process for single-host deployments). `IscsiBackend` ships as two impls in two crates: `IscsiControlPlaneBackend` linked into the manager binary, `IscsiHostBackend` linked into the agent binary. They share types via the new `nexus-storage` crate.

### Image population — generic streaming with optional fast path

The rootfs-allocation flow lives in `apps/manager/src/features/storage/` (replacing the old `LocalStorage::alloc_rootfs`). It is the only place that knows about ext4 — the trait does not.

```
if backend.capabilities().supports_clone_from_image:
    volume = control_plane.clone_from_image(source, opts)   // fast path: native clone
else:
    volume = control_plane.provision(opts)                  // empty volume
    attached = host.attach(volume)                          // gain bytes-level access
    host.populate_streaming(attached, source, target_size)  // pure byte copy
    if rootfs_needs_filesystem_resize(image_kind):          // ext4-only branch
        run resize2fs against attached path                 // caller, not trait
    // attached stays for VM start; otherwise host.detach
```

`rootfs_needs_filesystem_resize` is `true` only when the source image is a known ext4 rootfs and the target size is larger than the source. xfs/btrfs/qcow2/raw-without-fs all skip this step. Future filesystem types add branches here without touching `HostBackend`.

For LocalFile, `supports_clone_from_image` is `true` (using `fs::copy` directly is faster than detour-through-stream and matches existing behavior bit-for-bit; the LocalFile fast-path implementation does the `resize2fs` itself for backwards compatibility with the current code path). For Iscsi in this PR, `supports_clone_from_image` is `false` — generic streaming + caller-side resize is used.

The streaming runs **on the agent that will host the VM**. Image source is agent-local (`/srv/images` is already populated per-agent by the install flow). Cross-host image distribution is a separate problem.

`AttachedPath` variants (`File` vs `BlockDevice`) differ only in the path string the caller hands to `resize2fs` and to Firecracker — both tools accept either. Callers do not branch on the variant for normal flow.

### Capabilities drives placement decisions (eventually)

`Capabilities` is consumed by the control plane for placement, not just metadata. Even though we're not making placement decisions in this PR, the data is in `storage_backend.capabilities_json` so future code can:

- "VM needs live migration → only schedule on backends with `supports_live_migration`"
- "Instant snapshot requested → check `supports_native_snapshots` before falling back to chunk pipeline"
- "Read-only multi-attach (future) → require `supports_concurrent_attach`"

### Backend identity is per-volume, not per-cluster

A single deployment must support multiple backends simultaneously. `volume.backend_id` records which backend instance the volume belongs to; operations route via the registry on every call.

### `host_id` semantics for new volumes

For LocalFile-backend volumes, the control-plane half sets `volume.host_id` at provision time to the host where the file lives. In this PR LocalFile is single-host (the manager host); cross-agent placement is explicitly out of scope (see non-goals). For Iscsi/network backends, `host_id` is left NULL. Existing rows are unaffected: the migration preserves their `host_id` values verbatim.

### `volume.path` semantics across backends

The existing `volume.path TEXT NOT NULL` column stays. For LocalFile it is the canonical local file path (current behavior). For Iscsi/network backends it stores a backend-defined identifier (e.g., the iSCSI IQN + LUN number) — this is the same string the control-plane backend can later parse to reconstruct the resource on attach. Per-backend deserialization rules live in each backend's impl; the registry hands the volume row to the backend, the backend interprets `path` (and any backend-specific JSON in a future column if needed). The unique constraint on `path` stays — within a single backend instance the identifier is unique.

### VM start lifecycle through the trait

On VM start, the manager iterates the VM's `volume_attachment` rows. For each, it resolves `volume.backend_id` to a `HostBackend` impl on the chosen agent, calls `attach`, and inserts the resulting `AttachedPath` into the Firecracker drive config the manager hands to the agent's spawn flow. On VM stop or delete, the manager calls `detach` for each attachment and updates `volume_attachment.detached_at`.

### `volume_attachment` row lifecycle vs populate-time attach

These are deliberately distinct:

- **Populate-time attach is in-memory only.** During `alloc_rootfs`, the host backend's `attach` returns an `AttachedPath` that exists only in the running process. No `volume_attachment` row is written. If the VM-create flow fails before VM start, the caller calls `host.detach` and the volume is left clean.
- **`volume_attachment` rows are written by the VM lifecycle, not by storage operations.** A row is INSERTed only when `vm start` succeeds and Firecracker is running with the drive open. The partial unique index `WHERE detached_at IS NULL` therefore only constrains real, in-use attachments — not transient populate-time access.
- **VM start reuses the existing populate-time attach when on the same host** (no detach/reattach round-trip). The agent process holds the `AttachedPath` from populate, hands it to spawn, and the manager INSERTs the `volume_attachment` row once spawn returns success.
- **VM start on a different host** (e.g., volume populated on host A but VM scheduled on host B — not in this PR but the lifecycle must not preclude it) would call `host.detach` on A, then `host.attach` on B before INSERTing the row. The trait already supports this; the scheduling logic does not exist yet.

### `AlreadyAttached` error translation

The single-attach partial unique index returns Postgres error code `23505` (`unique_violation`). The volume-attach handler in the manager catches this specific error code via sqlx (`sqlx::Error::Database` with `code() == "23505"` and constraint name `volume_one_active_attachment`) and translates it to `StorageError::AlreadyAttached`, surfaced as HTTP `409 Conflict`. Without this explicit translation, callers see a generic database error and the contract leaks.

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
- How backend errors surface. (Recommendation: backend-specific error types behind a common `StorageError` enum with a `Backend(Box<dyn Error>)` variant for non-categorical failures; use `anyhow` only at the route layer. `StorageError` includes `AlreadyAttached`, `NotSupported`, `Backend(...)` at minimum.)
- **iSCSI session lifecycle on VM stop.** (Recommendation: **aggressive logout** per-volume on detach. iSCSI login is ~100ms — the simplicity of zero state beats fragile in-process refcounting that gets lost across agent restarts. If profiling shows VM-start latency pain from repeat logins to the same target, revisit with a DB-driven refcount over `volume_attachment` rows on that host.)
- Where TrueNAS API credentials live. (Recommendation: env-var reference in TOML — `api_key_env = "TRUENAS_PROD_API_KEY"` — manager reads env on startup; secrets never in the DB.)
- CHAP authentication for generic iSCSI. (Recommendation: optional fields in TOML config_json, passed through to `iscsiadm`.)
- **Agent handshake reports supported `BackendKind` set.** Both traits expose `kind()`, but nothing currently links "manager has Iscsi control-plane" to "agent N has Iscsi host backend." On agent registration (or heartbeat), the agent should report its installed `HostBackend` kinds; the manager refuses to route an `attach` call to an agent that doesn't have the matching kind. Implementation deferred to a future PR; for this PR, all agents are assumed to have `LocalFile` and `Iscsi` host backends compiled in. Document the seam in the PR description.
- **`config_json` startup validation.** Each `BackendKind` defines the required shape of its `config_json` (TrueNAS: `endpoint`, `api_key_env`, `pool`, `target_iqn_prefix`; generic Iscsi: `target_iqn`, optional CHAP fields). The registry validates the JSON against the kind's schema at manager startup and refuses to boot if any backend is malformed. Per-backend schema lives next to each backend impl.

Resolve these as you go; document the choices in the PR description.

## File-level outline of the change

Manager:
- `crates/nexus-storage/` (new) — trait definitions and shared types.
- `apps/manager/src/features/storage/mod.rs` — refactored: `LocalStorage` is renamed/wrapped as `LocalFileBackend` implementing both traits.
- `apps/manager/src/features/storage/backends/local_file.rs` (new) — control-plane half (single-host: same impl, just one process).
- `apps/manager/src/features/storage/backends/iscsi.rs` (new) — control-plane half (TrueNAS REST + generic).
- `apps/manager/src/features/storage_backends/` (new) — `mod.rs`, `routes.rs`, `repo.rs` for `GET /v1/storage_backends*`.
- `apps/manager/src/features/storage/registry.rs` (new) — loads TOML, upserts `storage_backend`, hands out trait objects keyed by `backend_id`.
- `apps/manager/src/features/vms/service.rs` — `create_vm` calls a new `rootfs_allocator` helper that orchestrates: capability check → either `clone_from_image` (fast path) or `provision` + `host.attach` + `host.populate_streaming` + caller-side `resize2fs` (slow path). The `resize2fs` step is gated by image-kind detection and lives in the allocator, not in any backend impl. Replaces `LocalStorage::alloc_rootfs`.
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

- **Unit tests** per backend impl: provision/destroy round-trip; snapshot + clone_from_snapshot round-trip (LocalFile uses tmpdir; Iscsi tests gated behind `--ignored` and require a TrueNAS sim/instance).
- **Trait-purity test**: assert that calling `populate_streaming` on a fresh volume produces a byte-for-byte copy of the source image and does NOT modify any filesystem metadata (e.g., source ext4 image's free-block count is unchanged after populate).
- **Capability gating test**: calling `clone_from_image` on a backend with `supports_clone_from_image == false` returns `StorageError::NotSupported`.
- **Integration test** (`apps/manager/tests/storage_abstraction.rs`): boot manager + in-process LocalFile backend; create VM → verify Firecracker config sees correct rootfs path → verify volume_attachment row exists → stop VM → verify detached_at set.
- **Populate-time attach test**: trigger a `create_vm` failure mid-populate (after `host.attach`, before VM start). Verify that no `volume_attachment` row was written, the volume is `host.detach`'d, and a retry creates a new clean volume.
- **Migration test**: load a fixture with pre-migration `volume` rows, run migration, verify `backend_id` populated and `host_id` preserved.
- **Single-attach test**: attempt double-attach, expect `409` + `AlreadyAttached` (verifies the `23505` translation works end-to-end).
- **Config validation test**: malformed `[[storage_backend]]` TOML entry (e.g., `kind = "truenas_iscsi"` missing `endpoint`) causes manager startup to fail with a clear error.
- **No regression**: existing `cargo test -p manager` and `cargo test -p agent` suites pass without modification.

## Glossary

- **VmCheckpoint** — Firecracker memfile + state file written by `apps/agent/src/features/vm/snapshot.rs`. Used for pause/resume of a *running* VM. Unchanged in this PR.
- **VolumeSnapshot** — block-level point-in-time copy of a volume's bytes. The new trait's `snapshot` method creates these (returning a `VolumeSnapshotHandle`). ZFS/RBD/TrueNAS instant; LocalFile slow-but-correct via `fs::copy`.
- **clone_from_snapshot** — creates a NEW volume from a `VolumeSnapshotHandle`. Never mutates the source volume. Distinct from a future `rollback_to_snapshot` (out of scope).
- **AttachedPath** — what the host backend hands back to whoever is going to feed it to Firecracker. File path, block device path, or future vhost-user socket path.
- **Backend instance** — a row in `storage_backend`. Identified by `backend_id`. There can be multiple instances of the same kind (`truenas-prod`, `truenas-dr`).
