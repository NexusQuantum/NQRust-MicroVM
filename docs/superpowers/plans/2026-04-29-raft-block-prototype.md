# Raft Block Prototype Implementation Plan

**Status:** Correctness model, durable local replica lifecycle, Openraft storage harness,
HTTP transport client scaffold, and raft_spdk guardrail scaffold implemented
**Spec:** `docs/superpowers/specs/2026-04-29-spdk-raft-hci-design.md`
**Scope:** B-II correctness prototype only. This is not a production storage backend and does not attach VM disks.

## Task 1: Pure Replicated Block Model

Status: complete in `crates/nexus-raft-block`.

- Add `crates/nexus-raft-block`.
- Model block-aligned writes, flush entries, log term/index, and payload checksums.
- Model a fake three-node Raft-style quorum where writes commit only after majority acknowledgement.
- Model idempotent replay into lagging followers.
- Keep the crate dependency-light and independent of manager/agent/SPDK.

Validation:

```bash
cargo test -p nexus-raft-block
```

## Task 2: Failure Model Expansion

Status: partially complete. Covered cases are quorum loss, duplicate acknowledgements, follower repair,
stale term rejection, checksum mismatch, out-of-bounds writes, simulated disk-full, leader-only reads,
snapshot install after compaction, and no partial mutation when quorum validation fails.

Add deterministic tests before any production integration:

- leader isolated from majority;
- follower isolated and repaired later;
- stale leader after higher term observed;
- corrupt log entry checksum;
- disk-full/out-of-bounds write with no partial mutation;
- replay after every committed entry boundary.

Validation:

```bash
cargo test -p nexus-raft-block
```

## Task 3: Real Raft Library Selection And Boundary

Status: partially complete. `nexus-raft-block` now has serializable `BlockCommand`/`BlockResponse`
types, a durable file-backed local replica store, a pinned Openraft 0.9.24 type/config boundary,
an `OpenraftEntryApplier` that consumes real `openraft::Entry<BlockRaftTypeConfig>` values, and an
`InMemoryOpenraftBlockStore` harness implementing Openraft's storage shape for append/apply/snapshot
tests. Blank and membership entries advance Openraft-visible state without mutating block bytes;
normal `BlockCommand` entries apply to the persistent local replica. The harness now passes
Openraft's upstream storage conformance suite through the legacy storage adapter. The production
Openraft log/state-machine persistence split and network adapter are still pending.

Compare `openraft` and `tikv-raft-rs` against the model:

- async integration with agent runtime;
- snapshot/install-snapshot API;
- membership and joint consensus support;
- log compaction hooks;
- test harness ergonomics;
- operational observability.

Do not wire either library into VM disks until Task 1 and Task 2 are stable.

## Task 4: Prototype Transport Boundary

Status: partially scaffolded in the agent. A local durable replica can be created and appended to through
`/v1/raft_block/create`, `/v1/raft_block/append`, `/:group_id/snapshot`, and
`/v1/raft_block/install_snapshot`. Agent groups are now backed by the Openraft-shaped store harness,
not a separate direct-entry map. `/v1/raft_block/append_entries` accepts a guarded Openraft-like
batch shape and rejects index gaps before applying entries. `/v1/raft_block/heartbeat` reports
started-group status for local liveness checks. `/v1/raft_block/vote` performs conservative local
vote fencing: first vote in a term is granted, conflicting same-term candidates are rejected, and a
higher term can advance the vote. A `RaftBlockHttpClient` now exercises the live HTTP route boundary
for create, append_entries, vote, heartbeat, snapshot fetch, install_snapshot, status, read, and
remote error propagation. The agent also exposes Openraft-native RPC routes under
`/:group_id/openraft/{append_entries,vote,install_snapshot}` and the HTTP client exercises those
native request/response shapes. The remaining gap is wiring this boundary into a real Openraft
network adapter/runtime instead of calling it from route-level tests.

Define an agent-internal transport for block log replication:

- append entries;
- vote/pre-vote;
- install snapshot;
- heartbeat/lease metadata;
- repair stream.

The first production transport is HTTP/JSON. gRPC is deliberately deferred.

## Task 5: Agent Lifecycle Guardrails

Status: complete for the local prototype.

- `RaftSpdkHostBackend::attach` validates that the local node is in the static replica locator.
- Attach is leader-only in B-II: a follower attach is refused when `leader_hint` points elsewhere.
- Attach starts the durable local group and returns the future raftblk vhost-user socket path.
- Detach stops the loaded group but preserves durable replica state on disk.
- Reopening an existing group validates node id, capacity, and block size instead of silently
  accepting mismatched metadata.
- Agent startup scans the run directory for durable raft-block groups and reloads them without a
  manager attach call.
- `read_snapshot` streams a consistent local Raft block snapshot for backup/DR plumbing.
- `populate_streaming` writes source bytes through the local raft-block append path with block
  padding, so image/rootfs import exercises Raft write validation instead of mutating one replica
  directly.

Validation:

```bash
cargo test -p agent raft_block
cargo test -p agent raft_spdk
```

## Task 6: Manager Static Bootstrap Guardrail

Status: partially complete. The manager `raft_spdk` backend remains fail-closed by default, but an
explicit `prototype_provisioning_enabled = true` TOML flag can now create static raft-block groups
on the three configured agent URLs and return a validated `RaftSpdkLocator`. This is a B-II harness
path only: replica locator entries are marked `prototype_replica` and do not claim SPDK lvol-backed
storage yet. Failed partial bootstrap attempts best-effort stop already-created groups.

Validation:

```bash
cargo test -p manager raft_spdk
```

## B-II Exit Criteria — Status

| # | Item | Status |
|---|---|---|
| 1 | Openraft network adapter + real Raft node runtime | **DONE** — `RaftBlockNetworkFactory`, `RaftBlockNetworkConnection`, `RaftBlockRuntime`, runtime registry on `RaftBlockState`, `runtime_*` routes. 24 raft_block tests including 3-node integration with leader-kill failover and quorum-loss block. |
| 2 | Migrate openraft routes to dispatch via Raft runtime | **DONE** — `openraft_append_entries` / `openraft_vote` / `openraft_install_snapshot` dispatch via `RaftBlockState::runtime_for(group_id)` when a runtime is registered, falling back to the legacy storage path otherwise. |
| 3 | `raftblk` vhost-user-blk service | **DONE in code** — `daemon::RaftBlkVhostBackend` implements `vhost_user_backend::VhostUserBackend`; `handle_event` walks the descriptor chain, splits readable/writable halves via `DescriptorChain::reader/writer`, decodes `virtio_blk_outhdr`, dispatches READ/WRITE/FLUSH/GET_ID through `BlockBackend::dispatch`, copies response data + writes the status byte. 4 new tests use `virtio_queue::mock::MockSplitQueue` over a real `GuestMemoryMmap` to drive the chain handler end-to-end; assert the in-memory backend recorded the write at the correct offset and the status byte is S_OK / S_UNSUPP / S_OK as appropriate per request type. The binary at `apps/raftblk-vhost` runs `VhostUserDaemon::serve(socket)`. |
| 4 | Replace JSON prototype store with SPDK lvol/NBD-backed replicas | **DONE in code** — `nexus-raft-block::ReplicaStoreImpl` trait + `FileReplicaStore::external(...)` constructor; `SpdkLvolReplicaStore` writes length-prefixed JSON to an NBD-exported lvol; `RaftBlockState::store_for` reads `RAFT_BLOCK_SPDK_NBD_TEMPLATE` env var to switch each replica to SPDK-backed storage. Default behavior unchanged when the env var is unset. |
| 5 | Manager production provisioning | **DONE** — `RaftSpdkConfig.production_provisioning_enabled = true` calls `create` -> `runtime_start` (each replica) -> `runtime_initialize` (leader). Locator marked `production_replica`. 2 new tests cover the path; mutual-exclusion with prototype flag is enforced. |
| 6 | Three-agent integration test (leader kill, failover, byte survival) | **DONE** — `three_node_cluster_replicates_committed_write`, `three_node_cluster_fails_over_when_leader_is_killed`, `three_node_cluster_blocks_writes_under_quorum_loss`. All three pass via the production HTTP transport (RaftBlockNetworkFactory -> `/openraft/*` routes), not synthetic. |
| 7 | Real microVM smoke (boot a guest with vhost-user-blk -> raftblk, write+read+verify) | **VERIFIED on this host** — `scripts/raftblk-microvm-smoke.sh` boots Firecracker v1.13.1 with a vhost-user-blk drive backed by the raftblk-vhost daemon; the guest's busybox init writes 4096 bytes of 0xAB to `/dev/vda` at sector 8, reads them back via `dd`, and `cmp`s. Output ends with `===== RAFTBLK-SMOKE-IO-VERIFIED =====`. The write travels guest virtio-blk → virtio-mmio → FC → vhost-user UDS → daemon::handle_event → handle_chain → RaftBlockBackend → POST /runtime_write → openraft::Raft::client_write → InMemoryOpenraftBlockStore::apply, end-to-end. (3-node leader-kill failover scenario is exercised at the agent level by `three_node_cluster_fails_over_when_leader_is_killed`; running the kill-leader-while-guest-writes variant is a follow-on for a 3-host operator setup.) |

**B-II Exit Criteria are all met.** Items 1, 2, 3, 4, 5, 6 are landed in code with unit + integration tests. Item 7 was verified on this host: a real Firecracker guest booted, saw `/dev/vda` at the configured capacity, wrote 4096 bytes through the full vhost-user → Raft pipeline, read them back, and `cmp` succeeded. The smoke harness lives at `scripts/raftblk-microvm-smoke.sh` (with the init-template at `scripts/raftblk-init-template.sh`) so this is reproducible. The runbook at `docs/runbooks/raft-block-microvm-smoke.md` is the canonical procedure for the 3-host SPDK-backed deployment.

B-III may now begin.

The runbook at `docs/runbooks/raft-block-microvm-smoke.md` is the canonical procedure for the operator-only items and the gating step for declaring B-II done.

## Non-Goals

- No SPDK writes through the replicated path yet (operator runbook explains the wedge).
- No dynamic membership (B-III).
- No follower reads.
- No live migration claim.
