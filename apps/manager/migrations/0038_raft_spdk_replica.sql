-- 0038_raft_spdk_replica.sql
-- Durable raft_spdk membership table. TOML remains bootstrap input; B-III
-- membership changes persist here after the replicated Openraft change commits.

CREATE TABLE IF NOT EXISTS raft_spdk_replica (
  backend_id UUID NOT NULL REFERENCES storage_backend(id) ON DELETE CASCADE,
  group_id UUID NOT NULL,
  node_id BIGINT NOT NULL CHECK (node_id > 0),
  agent_base_url TEXT NOT NULL,
  spdk_lvol_locator TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'voter' CHECK (role IN ('voter', 'learner', 'removed')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  removed_at TIMESTAMPTZ,
  PRIMARY KEY (backend_id, group_id, node_id)
);

CREATE INDEX IF NOT EXISTS idx_raft_spdk_replica_group
  ON raft_spdk_replica(backend_id, group_id)
  WHERE removed_at IS NULL;

COMMENT ON TABLE raft_spdk_replica IS
  'Durable raft_spdk group membership after Openraft membership changes commit.';
