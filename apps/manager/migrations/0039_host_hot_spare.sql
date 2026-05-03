-- 0039_host_hot_spare.sql
-- B-III Task 5: per-host hot-spare and decommission state.
-- Decommission state is foundational for Task 6 (host decommission); the
-- two columns ship together so the host row carries the full lifecycle.

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS is_hot_spare BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS lifecycle_state TEXT NOT NULL DEFAULT 'active'
    CHECK (lifecycle_state IN ('active', 'draining', 'decommissioned'));

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS lifecycle_changed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_host_lifecycle_state
  ON host(lifecycle_state)
  WHERE lifecycle_state <> 'active';

COMMENT ON COLUMN host.is_hot_spare IS
  'When true, the host is held in reserve for failure recovery (Task 7) and is skipped by normal placement.';

COMMENT ON COLUMN host.lifecycle_state IS
  'B-III host lifecycle: active accepts placement; draining is mid-decommission and refuses new placement; decommissioned is terminal.';
