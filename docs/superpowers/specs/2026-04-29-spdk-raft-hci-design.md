# NQRust-MicroVM: SPDK + Raft HCI Storage

**Status:** Design
**Date:** 2026-04-29
**Owner:** kleopasevan
**Scope:** Path B sub-project B. Defines the SPDK lvol performance backend and the later replicated block tier. This spec intentionally separates single-node SPDK from distributed replication.

## Intent

Move NQRust-MicroVM from pluggable storage plus backups into an HCI-style storage stack:

1. **B-I: Single-node SPDK lvol backend** - fast local userspace block storage, native snapshots/clones, vhost-user-blk attachment to Firecracker.
2. **B-II: Raft-replicated block prototype and backend** - quorum-replicated writes across agent hosts, with correctness proven before production use.
3. **B-III: Cluster reconfiguration** - add/remove hosts, replica repair, rebalancing, decommissioning, and operator repair tooling.

The project must not blur these phases. Single-node SPDK improves local I/O but has the same host-loss blast radius as LocalFile. The HCI guarantee only starts when B-II is correct under failure.

## Current Foundation

Already available on `main`:

- `crates/nexus-storage` with split `ControlPlaneBackend` and `HostBackend` traits.
- Per-volume `backend_id`.
- `AttachedPath::VhostUserSock`, reserved for SPDK.
- Backup pipeline that can consume `HostBackend::read_snapshot`.
- Agent-side `supported_backend_kinds` handshake.

## B-I: SPDK Lvol Backend

### Backend Kind

Add `BackendKind::SpdkLvol` serialized as `spdk_lvol`.

### Manager TOML

```toml
[[storage_backend]]
name = "spdk-local"
kind = "spdk_lvol"
is_default = false

[storage_backend.config]
rpc_socket = "/run/spdk/rpc.sock"
lvs_name = "nexus"
vhost_socket_dir = "/var/tmp"
```

`rpc_socket` is the SPDK JSON-RPC Unix socket. `lvs_name` is the pre-created lvol store. `vhost_socket_dir` is where SPDK exposes vhost controllers.

### Capabilities

```text
supports_native_snapshots = true
supports_concurrent_attach = false
supports_live_migration = false
supports_clone_from_image = false initially
```

The initial backend cannot advertise `clone_from_image` until image import exists. A vhost-user socket is a Firecracker transport, not a writable block path, so the generic `populate_streaming` slow path cannot copy an image into SPDK by writing to the returned socket.

### JSON-RPC Calls

The first implementation uses SPDK JSON-RPC:

- `bdev_lvol_create`
- `bdev_lvol_delete`
- `bdev_lvol_snapshot`
- `bdev_lvol_clone`
- `vhost_create_blk_controller`
- `vhost_delete_controller`

The volume locator is JSON:

```json
{
  "lvs_name": "nexus",
  "lvol_name": "nq-rootfs-...",
  "lvol_uuid": "...",
  "size_bytes": 10737418240
}
```

### Firecracker Integration

Firecracker vhost-user block drives use `socket`, not `path_on_host`.

For `AttachedPath::File` and `AttachedPath::BlockDevice`, manager sends:

```json
{"drive_id":"rootfs","path_on_host":"/dev/...", "is_root_device":true,"is_read_only":false}
```

For `AttachedPath::VhostUserSock`, manager sends:

```json
{"drive_id":"rootfs","socket":"/var/tmp/nq.<volume-id>", "is_root_device":true}
```

Read-only state is controlled by the backend-advertised virtio feature, not Firecracker's `is_read_only` field.

### Image Import

Image import into SPDK is not solved by vhost-user. The first implementation uses SPDK's Linux NBD export:

1. `attach` creates the vhost controller and records `vhost socket -> lvol UUID` inside the agent backend.
2. `populate_streaming` starts `nbd_start_disk` for that lvol on a configured NBD device.
3. The agent writes image bytes to the NBD device and calls `sync_all`.
4. The agent always attempts `nbd_stop_disk` before returning.

This requires the `nbd` kernel module to be loaded and one or more configured NBD devices. The agent reads `AGENT_SPDK_NBD_DEVICES` as a comma-separated pool, falling back to `AGENT_SPDK_IMPORT_NBD_DEVICE`, then `/dev/nbd0`. Each import, resize, or snapshot-read operation takes a lease from the pool and releases it only after `nbd_stop_disk`.

Ext4 growth uses the same pattern. The manager's rootfs allocator detects the source image as ext4, then calls the agent resize route with the backend kind. LocalFile and iSCSI run `e2fsck`/`resize2fs` directly on their attached path; SPDK exports the lvol to NBD, runs `e2fsck`/`resize2fs` on the NBD device, and stops the export.

Snapshot backup reads also use NBD in this first slice. `read_snapshot` parses the snapshot lvol locator, exports that lvol to the configured NBD device, opens it for reading, and stops the export when the reader is dropped.

### Explicit B-I Gaps

- NBD pool capacity directly limits concurrent SPDK imports, resizes, and snapshot backup reads on the agent. Operators must provision enough `/dev/nbdX` devices for expected concurrency.
- SPDK process lifecycle, hugepage setup, vfio binding, and lvstore creation are operational prerequisites and need installer/runbook support before production use.
- The development bootstrap script is intentionally not production lifecycle management. It builds a local SPDK checkout under `.worktrees/spdk`, starts a memory-backed target, and applies local build pruning so smoke tests can run on developer machines without the full SPDK dependency surface.

## B-II: Raft Replication

Do not implement B-II directly against production VM disks first. Build a fake-block prototype and chaos harness before using SPDK lvols.

Required design decisions:

- write unit size and alignment,
- log entry format and checksums,
- flush, FUA, and barrier semantics,
- leader fencing and stale leader prevention,
- idempotent replay after agent restart,
- read policy: leader-only first, follower reads only after leases are proven,
- log compaction and snapshot interaction,
- repair after missed writes,
- quorum loss behavior,
- corruption detection and operator-visible health.

Minimum safety bar:

- deterministic model tests for write ordering,
- crash/restart tests at every await boundary in write replication,
- partition tests: leader isolated, follower isolated, majority loss,
- disk-full and partial-write simulations,
- checksum mismatch tests,
- restore from backup after replica loss.

## B-III: Reconfiguration

B-III depends on B-II invariants. It adds:

- host add/remove,
- replica placement,
- replica rebalancing,
- hot-spare promotion,
- decommission workflow,
- repair queue,
- status API and CLI.

Membership changes must use Raft joint consensus or an equivalent safe transition. Never change replica sets by mutating DB rows outside the replicated protocol.

## Success Criteria

B-I succeeds when:

- Manager can provision/destroy SPDK lvols.
- Manager can snapshot/clone/delete SPDK lvol snapshots.
- Agent can create/delete SPDK vhost-blk controllers and return `VhostUserSock`.
- VM start sends Firecracker `socket` for vhost-user rootfs.
- Agent can import image bytes and read snapshots through a leased NBD device, with NBD setup/release waits to avoid racing the kernel device.
- The real-SPDK smoke test passes against a live `spdk_tgt`.

B-II succeeds only when chaos tests demonstrate correct behavior under crashes, partitions, disk-full, and replay.
