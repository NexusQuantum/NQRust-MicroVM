# Raft Block Reconfiguration (B-III) Implementation Plan

**Status:** In progress — Task 1 backend/API/auth slice landed; UI/live validation pending.
**Spec:** `docs/superpowers/specs/2026-04-29-spdk-raft-hci-design.md` § "B-III: Reconfiguration".
**Predecessor:** `docs/superpowers/plans/2026-04-29-raft-block-prototype.md` (B-II).
**Scope:** Take B-II's static three-replica raft_spdk groups and make membership dynamic — host add/remove, replica repair, rebalancing, hot-spares, decommission, plus an operator-facing status surface.

## Where B-II left off

The 1-node and 3-node smokes pass. Replicated populate via openraft is wired (commit `4594375`), the spdk_lvol manifest mechanism survives agent restarts (`3981328` + `4d029c2`), URLs normalize (`7634bc0`), the typed `RaftBlockStoreKind` enum gates store-mode mismatches (`d289bd3`), and standalone volume create/delete now drives `backend.provision()` / `backend.destroy()` (`754a475` + `79d936b`).

Static membership is configured in TOML at manager startup. Adding or removing a replica is a manager restart with a config edit. There is no observability beyond per-group `/status` and the manager log. Replica re-sync after an extended outage works only because the local sidecar/spdk_lvol persistence preserves the log — there is no operator-facing knob to drive a repair.

These are exactly the gaps B-III closes.

## Task 1: Group-level status API

Status: in progress.

The first thing every other B-III feature needs is observability. Before changing membership, an operator must see the cluster's view of the cluster.

- Add `GET /v1/storage_backends/{id}/groups` returning every group the backend knows about (group_id, capacity, block_size, current leader_hint).
- Add `GET /v1/storage_backends/{id}/groups/{group_id}` aggregating per-replica status by fan-out to each replica's `/v1/raft_block/{group_id}/status`. Return the aggregated metrics: per-node `last_applied_index`, `retained_log_entries`, `store_kind`, `store_path`, plus a derived `quorum_state` (`leader_steady` / `electing` / `quorum_lost`) and `lagging_followers` (any node whose `last_applied_index` is more than N entries behind the leader's commit index — N is configurable, defaults to 1024).
- Surface the same data in `apps/ui` under a new "Storage / Replication" panel on the storage backend detail page. Read-only; no mutating actions yet.
- Auth: status is read-only; admin role only because the response leaks per-host topology.

Implementation notes:

- DONE: agent `/v1/raft_block/{group_id}/status` now includes Raft runtime fields (`raft_state`, `current_term`, `current_leader`, `last_log_index`, `millis_since_quorum_ack`) when the Openraft runtime is active.
- DONE: manager `GET /v1/storage_backends/{id}/groups` derives known groups from current `volume` rows whose locator parses as `RaftSpdkLocator`. This is the B-II source of truth until Task 3 introduces `raft_spdk_replica`.
- DONE: manager `GET /v1/storage_backends/{id}/groups/{group_id}` fans out to the locator's replica agents, returns per-node status/errors, derives `quorum_state`, and reports `lagging_followers` using configurable `?lag_threshold=`.
- TODO: wire the read-only UI panel.
- DONE: storage backend routes are protected by the manager auth middleware plus admin-role middleware.
- TODO: live KubeVirt validation.

Validation:

- Unit: aggregator collapses three matching `/status` payloads into one response, marks `quorum_state: leader_steady` when all three see the same leader_id; marks `quorum_lost` when fewer than `n/2 + 1` respond.
- Live: bring up the 3-node KubeVirt smoke, query the new endpoint, kill leader-1, query again. Expect `quorum_state` to flip from `leader_steady` → `electing` → `leader_steady` once a survivor wins.

```bash
cargo test -p manager status_api
cargo test -p agent raft_block::tests::status
# Live:
curl -s http://manager/v1/storage_backends/$BID/groups/$GID | jq .
```

## Task 2: Single-replica repair (catchup)

Status: implementation slice done — manager repair endpoint restarts an existing replica runtime, waits for catch-up, records the operation, and exposes repair status; live validation pending.

The simplest membership operation. A replica that fell behind (extended host outage) but is still in the configured replica set needs to catch up from the leader. Today this happens implicitly through openraft's append_entries — but only if the lagging follower's host is up and reachable. Operators need a way to trigger it explicitly and observe progress.

- Add `POST /v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}/repair` on the manager. Idempotent.
- Implementation: the manager sends `runtime_start` to the agent for `node_id` with the current peer URL map (re-bootstraps the runtime if the agent restarted with empty in-memory state but on-disk store is intact). If the manifest is missing on the target host, return 412 `Precondition Failed` — that's a host-rebuild scenario covered by Task 5, not Task 2.
- Wait for the follower's `last_applied_index` to reach the leader's committed index (poll `/status`, default timeout 5 minutes).
- Surface progress: stream from a new `GET /v1/.../replicas/{node_id}/repair_status` endpoint or include in Task 1's status aggregator.

Implementation notes:

- DONE: `POST /v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}/repair` validates the raft_spdk locator, creates a `raft_repair_queue` row, sends `runtime_start` with the full peer map to the target replica, polls `/status` until the target reaches the peer high-water mark, and marks the row succeeded/failed.
- DONE: runtime-start errors that look like missing local replica state return 412 `Precondition Failed`; unreachable agents still return upstream failure.
- DONE: `GET /v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}/repair_status` returns the latest repair queue row plus current applied/required catch-up progress.
- TODO: live 3-node validation.

Validation:

- Unit: agent's `runtime_start` is idempotent on a node where it's already running.
- Live: bring up 3-node smoke, write a few entries, kill agent-3 mid-write, restart agent-3 (which loses runtime state but keeps manifest), trigger repair, verify `last_applied_index` catches up.

## Task 3: Replica add (joint consensus path)

Status: not started.

This is the first **mutating** membership change. It must go through openraft's joint consensus or be rejected. **Never write replica set changes directly to TOML and restart the manager.**

- Manager-side: `POST /v1/storage_backends/{id}/groups/{group_id}/replicas` with body `{ "node_id": u64, "agent_base_url": String, "spdk_backend_id": Uuid }`.
  - Validate the new node_id doesn't collide with existing replicas in the locator.
  - Drive `agent_a.create_group` on the new replica's agent (same as B-II provisioning, with `desired_store_kind` matching the backend's mode).
  - Drive `agent_a.runtime_start` on the new replica with the current peer URL map *plus* the new entry (so it can catch up via append_entries).
  - Issue a Raft membership change RPC against the current leader. The agent route is new: `POST /v1/raft_block/{group_id}/openraft/change_membership` accepting an openraft `ChangeMembers` payload.
  - Use openraft's `change_membership(...)` with `retain=false` (or joint+commit) so the new node enters as a Voter only after it catches up. Openraft 0.9 `change_membership` already does the joint phase; expose the option to caller to force pre-vote catchup if needed.
  - Persist the new replica into the backend config (UPSERT into a new `raft_spdk_replica` table keyed by `(backend_id, node_id)`) so manager restarts see the new membership without re-running TOML validation. The TOML config becomes a *bootstrap* config; subsequent membership changes are durable in the DB.
- Backend-side change: `RaftSpdkControlPlaneBackend` reads replicas from DB on construction (TOML still seeds an initial set on first run). Locators issued after a successful add reflect the new membership.
- Concurrency: only one membership operation per group at a time. Take an advisory pg lock keyed by `(backend_id, group_id)` for the duration of the change.

Validation:

- Unit: model test in `nexus-raft-block` exercising openraft's joint consensus with one new voter. Confirm a write committed in the joint phase is visible on all old + new voters after commit.
- Live: 3-node smoke, write data, add node-4 via the new endpoint, verify md5 of capacity region on all 4 replicas matches.

## Task 4: Replica remove (decommission of one replica)

Status: not started.

Symmetrical to add. Removing a replica from a group is one half of decommissioning a host (Task 6).

- `DELETE /v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}`.
  - Refuse if the resulting voter set would be smaller than 2 (single-node groups stay single-node by configuration; you don't drop to zero this way).
  - Refuse if `node_id` is the current leader unless `force=true` — leader removal requires a leader transfer first (Task 4a below).
  - Drive openraft `change_membership` to drop the voter.
  - On commit: `agent.stop_runtime` + `agent.destroy_group` on the removed node (releases the spdk_lvol stub and removes the manifest, same as `backend.destroy()`).
  - Update DB membership.
- Task 4a: `POST /v1/storage_backends/{id}/groups/{group_id}/leadership/transfer` — manager sends openraft `transfer_leader(target)` against the current leader. Used as a precursor to leader removal.

Validation:

- Unit: model test that removes one of three voters; confirm next write on the remaining two commits with quorum=2.
- Live: 3-node smoke, transfer leadership, remove old leader, write through new leader, confirm md5 on the two survivors.

## Task 5: Host add

Status: not started.

A host is added to the cluster (a new agent registers with the manager). B-III's host-add is the *capacity* admission. It does not automatically become a replica — Task 8's rebalancer or an operator's explicit Task 3 places replicas onto it.

- Existing `POST /v1/hosts/register` already covers the agent-side handshake. B-III adds a manager-side reconciliation: when a new healthy host appears with `supports_backend_kinds` including `raft_spdk`, mark it as a candidate target for placement and surface it in the new "Storage / Replication" UI panel.
- Hot-spare flag: per-host capability `is_hot_spare` (default false). Hot-spare hosts only receive replicas during failure recovery (Task 6 promote), not during normal placement.
- No mutating action by default. Adding a new host without explicit replica-add is harmless — it just sits in the candidate pool.

Validation:

- Unit: candidate selector skips hosts without `raft_spdk` in `supported_backend_kinds`.
- Live: register a 4th host, confirm it appears in the "Storage / Replication / Candidates" UI list with status `idle`.

## Task 6: Host decommission

Status: not started.

The full inverse of host-add: remove a host from the cluster, draining all replicas it hosts first.

- `POST /v1/hosts/{id}/decommission` puts the host in `draining` state (new column on `host` table).
- Manager-side reconciler walks every group with a replica on this host and runs Task 4 (replica remove) for that node_id. If a hot-spare exists, the reconciler runs Task 3 (replica add) onto the spare *before* the remove, so the group's voter count stays at 3 throughout.
- Refuse decommission if doing so would drop any group below 2 voters and no hot-spare is available. Operator must add capacity first.
- On success: host transitions to `decommissioned`. Subsequent VM creation refuses to schedule rootfs onto decommissioned hosts. Agent process keeps running (so destroy RPCs still work for any straggling resources) until operator stops it manually.

Validation:

- Unit: reconciler dry-run on a 3-host setup with one hot-spare confirms the planned operations are `[add hot-spare to G, remove decommission target from G]` for every group.
- Live: 4-host setup (3 voters + 1 hot-spare), decommission one voter, observe reconciler add hot-spare and remove the old voter, md5 on the new 3-replica set matches.

## Task 7: Hot-spare promotion on host failure

Status: not started.

Different from decommission: this is an *unplanned* host loss, where the manager detects a host has been unhealthy long enough that recovery should kick in.

- Health threshold: configurable `host_failure_recovery_after_seconds` (default 600 = 10 min). Default is conservative because false-positive promotion is expensive (full replica re-sync).
- When a host with raft_spdk replicas exceeds the threshold, the recovery reconciler runs Task 3 (add) for each affected group onto the best-available hot-spare, then leaves the failed replica in place (so it can be repaired via Task 2 if the host recovers).
- The failed replica remains a member of the group but is no longer counted toward placement; future writes commit on the new {survivors + spare} quorum.
- If the original host comes back: operator drives Task 4 (remove) to clean up the now-redundant replica, or runs Task 8 (rebalance) to drop it.

Validation:

- Live: 3 voters + 1 spare, kill voter-1's host abruptly, wait for recovery threshold, observe spare promoted, write through new quorum, confirm new md5.

## Task 8: Replica rebalancing

Status: not started.

Lowest priority because manual placement via Tasks 3/4 covers most operational needs.

- `POST /v1/storage_backends/{id}/rebalance` runs a planner that walks all groups and decides whether to migrate replicas to balance per-host load. The plan is shown to the operator (`?dry_run=true` returns the plan; without dry_run, executes).
- Placement policy: minimize the variance of `(group count per host)` across non-decommissioned, non-hot-spare hosts. Tie-break by host disk free space.
- Each migration is an add+remove pair (Tasks 3+4) so the group's voter count stays at 3 throughout.
- Rate-limited: at most one migration in flight per backend at a time.

Validation:

- Unit: planner test with deliberately skewed group counts (host A has 10 groups, hosts B/C have 0 each) produces a plan that adds 3-4 groups to B and C each.
- Live: skip until operator pressure makes this useful. Manual placement via Tasks 3+4 is the everyday path.

## Task 9: Repair queue

Status: in progress — schema and read API foundation landed; writers/reconciler pending.

A durable record of pending and in-flight membership operations so that a manager restart mid-operation doesn't leave a half-applied change.

- New table `raft_repair_queue (id, backend_id, group_id, op_type, op_args jsonb, state, attempts, last_error, started_at, finished_at)`.
- Every Task 3/4/6/7/8 operation appends a row before issuing any agent RPC and updates state on completion. The row is the source of truth for "is this group currently being reconfigured" (Task 3's pg lock holds while a row is `in_progress`).
- A reconciler retries failed operations with exponential backoff. After `max_attempts` (default 5), the row is moved to `failed` state and an alert is raised.
- API: `GET /v1/storage_backends/{id}/repair_queue` for operators.

Implementation notes:

- DONE: migration `0037_raft_repair_queue.sql` creates the durable operation ledger with checked `op_type` / `state` values and active-operation indexes.
- DONE: manager `GET /v1/storage_backends/{id}/repair_queue` lists recent rows for raft_spdk backends.
- TODO: helper functions that create/update queue rows for Tasks 2-8.
- TODO: retry reconciler with exponential backoff and idempotent resume hooks.

Validation:

- Unit: a Task-3 add that crashes after the openraft `change_membership` commit but before DB persistence is recovered by the reconciler — the second attempt observes the membership is already changed and just runs the persistence step.
- Live: kill the manager during a replica-add, restart, observe the queue row resume and complete.

## Task 10: Operator CLI

Status: not started.

Wraps Tasks 1-9 in a `nqvm` CLI subcommand for operators who don't want to talk JSON. Lives in the existing `crates/nqvm-cli` crate.

- `nqvm storage groups list` (Task 1).
- `nqvm storage groups show <group-id>` (Task 1 detail).
- `nqvm storage replicas add --group <id> --host <host-id>` (Task 3).
- `nqvm storage replicas remove --group <id> --node <node_id>` (Task 4).
- `nqvm storage hosts decommission <host-id>` (Task 6).
- `nqvm storage repair-queue` (Task 9).

Validation: shell-level `--help` parses; integration test using mock HTTP responses.

## Non-goals (deferred past B-III)

- **Cross-backend migration** (e.g. local_file → raft_spdk live migration). Different problem; needs a streaming copy + cutover protocol distinct from membership changes.
- **Erasure-coded replicas.** B-III is full-replica only; EC is a separate B-IV work item.
- **Tenant-aware placement.** Placement policy is just per-host load in B-III. Multi-tenant fairness is out of scope.
- **Online resize.** Capacity is fixed at provision time. Growing a group's capacity is a B-IV item.

## Order of attack

1. **Task 1 first.** No mutating change without the observation surface.
2. **Task 9 next.** Membership ops without the durable queue cannot survive manager restart; risk too high to skip.
3. **Tasks 3, 4, 4a together.** The atomic primitives. Tasks 5, 6, 7 build on them.
4. **Task 2** (repair) can land any time after Task 1 — it's read-only on membership.
5. **Tasks 5/6/7** as the operator-facing host lifecycle.
6. **Task 8** last; defer until measured load justifies it.
7. **Task 10** alongside whichever API task ships, not at the end.

## Success criteria for B-III

- 4-host failover smoke: kill any one host abruptly, hot-spare promotes within `host_failure_recovery_after_seconds`, no committed write is lost.
- Add+remove cycle on a single group commits and reverses cleanly with no orphaned manifests/stubs/lvols on either end.
- Decommission a healthy host with no hot-spare available: refuses with a clear error pointing at the placement constraint.
- Repair queue survives a `kill -9` of the manager mid-operation; after restart the operation completes with no manual intervention.
- Operator can answer "is my data healthy and where does it live" without reading agent logs.

## Operator-only items (will not be code-validated in CI)

- Real SPDK lvol creation/deletion alongside the Raft group lifecycle (B-II runbook covers this; B-III extends the same operator process to additions and removals).
- Multi-host kernel network tuning for openraft heartbeats under steady-state production load. The 3-node KubeVirt smoke has documented HTTP-over-loopback flakiness that production-grade infrastructure won't reproduce, but the operator should still validate against their actual fabric.
