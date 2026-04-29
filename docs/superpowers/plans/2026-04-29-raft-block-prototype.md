# Raft Block Prototype Implementation Plan

**Status:** First correctness slice implemented
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
stale term rejection, checksum mismatch, out-of-bounds writes, and no partial mutation when quorum
validation fails.

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

## Task 3: Real Raft Library Selection

Compare `openraft` and `tikv-raft-rs` against the model:

- async integration with agent runtime;
- snapshot/install-snapshot API;
- membership and joint consensus support;
- log compaction hooks;
- test harness ergonomics;
- operational observability.

Do not wire either library into VM disks until Task 1 and Task 2 are stable.

## Task 4: Prototype Transport Boundary

Define an agent-internal transport for block log replication:

- append entries;
- vote/pre-vote;
- install snapshot;
- heartbeat/lease metadata;
- repair stream.

The first transport can be in-process test doubles. Production HTTP/gRPC is a later slice.

## Non-Goals

- No SPDK writes through the replicated path yet.
- No `BackendKind::RaftSpdk` yet.
- No dynamic membership.
- No follower reads.
- No live migration claim.
