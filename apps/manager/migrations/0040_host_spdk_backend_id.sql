-- 0040_host_spdk_backend_id.sql
-- B-III Tasks 6/7/8 follow-up: each host that can carry raft_spdk
-- replicas needs an SPDK backend id (the lvol bdev id used at
-- provisioning time). Storing it on the host row lets the planner pick
-- a target host AND know which lvol id to pass to add_replica without
-- a separate operator step.
--
-- Nullable: hosts that don't host raft_spdk replicas (compute-only,
-- hosts behind a different storage backend) leave it NULL and the
-- planner skips them as raft_spdk targets.

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS spdk_backend_id UUID;

COMMENT ON COLUMN host.spdk_backend_id IS
  'SPDK lvol bdev id this host exposes for raft_spdk replicas. NULL means the host cannot host raft_spdk replicas.';
