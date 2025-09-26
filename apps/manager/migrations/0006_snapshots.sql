CREATE TABLE IF NOT EXISTS snapshot (
    id UUID PRIMARY KEY,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    snapshot_path TEXT NOT NULL,
    mem_path TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_snapshot_vm_id ON snapshot(vm_id);
CREATE INDEX IF NOT EXISTS idx_snapshot_state ON snapshot(state);

ALTER TABLE vm
    ADD COLUMN IF NOT EXISTS source_snapshot_id UUID REFERENCES snapshot(id);
