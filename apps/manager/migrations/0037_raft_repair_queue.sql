-- 0037_raft_repair_queue.sql
-- Durable operation ledger for raft_spdk repair and membership changes.

CREATE TABLE IF NOT EXISTS raft_repair_queue (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  backend_id UUID NOT NULL REFERENCES storage_backend(id) ON DELETE CASCADE,
  group_id UUID NOT NULL,
  op_type TEXT NOT NULL CHECK (
    op_type IN (
      'repair_replica',
      'add_replica',
      'remove_replica',
      'transfer_leader',
      'decommission_host',
      'promote_hot_spare',
      'rebalance'
    )
  ),
  op_args JSONB NOT NULL DEFAULT '{}'::jsonb,
  state TEXT NOT NULL DEFAULT 'pending' CHECK (
    state IN ('pending', 'in_progress', 'succeeded', 'failed', 'cancelled')
  ),
  attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  last_error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  started_at TIMESTAMPTZ,
  finished_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_raft_repair_queue_backend_group
  ON raft_repair_queue(backend_id, group_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_raft_repair_queue_active
  ON raft_repair_queue(state, updated_at)
  WHERE state IN ('pending', 'in_progress', 'failed');

COMMENT ON TABLE raft_repair_queue IS
  'Durable raft_spdk operation ledger. Membership changes must create a row here before issuing agent or Openraft RPCs.';
