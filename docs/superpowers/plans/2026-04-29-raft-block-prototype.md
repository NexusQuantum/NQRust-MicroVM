# Raft Block Prototype Implementation Plan

**Status:** Correctness model, durable local replica lifecycle, Openraft storage harness, and
raft_spdk guardrail scaffold implemented
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
normal `BlockCommand` entries apply to the persistent local replica. The production Openraft
log/state-machine persistence split and network adapter are still pending.

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
higher term can advance the vote.

Define an agent-internal transport for block log replication:

- append entries;
- vote/pre-vote;
- install snapshot;
- heartbeat/lease metadata;
- repair stream.

The first transport can be in-process test doubles. Production HTTP/gRPC is a later slice.

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

Validation:

```bash
cargo test -p agent raft_block
cargo test -p agent raft_spdk
```

## B-II Exit Criteria Still Open

Do not start B-III until these are complete:

- Run the upstream Openraft storage test suite against the promoted storage harness.
- Implement Openraft HTTP network adapter for append, vote, heartbeat, and install-snapshot.
- Implement `raftblk` vhost-user-blk service and make VM guest writes propose through Raft.
- Move committed block bytes from the JSON prototype store to SPDK lvol/NBD-backed replicas.
- Implement manager-side replica provisioning and bootstrap for static three-node groups.
- Run a three-agent integration test that writes through raftblk, kills the leader, elects a new
  leader, and proves committed bytes survive.

## Non-Goals

- No SPDK writes through the replicated path yet.
- `BackendKind::RaftSpdk` exists only as a guarded scaffold. It does not provision production volumes yet.
- No dynamic membership.
- No follower reads.
- No live migration claim.
