-- Create runtime_snapshots table for managing container runtime warm boot snapshots
CREATE TABLE IF NOT EXISTS runtime_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    runtime_image_id UUID NOT NULL REFERENCES image(id) ON DELETE CASCADE,
    snapshot_path TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'creating',  -- 'creating', 'ready', 'unhealthy', 'deleted'
    fc_version TEXT NOT NULL,                -- Firecracker version compatibility
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success_count INT NOT NULL DEFAULT 0,
    failure_count INT NOT NULL DEFAULT 0,
    last_used_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb,      -- size_bytes, compressed_size_bytes, etc.
    CONSTRAINT valid_state CHECK (state IN ('creating', 'ready', 'unhealthy', 'deleted'))
);

-- Add boot_method column to containers table
ALTER TABLE containers
ADD COLUMN IF NOT EXISTS boot_method TEXT CHECK (boot_method IN ('warm', 'cold'));

-- Create index on runtime_image_id for fast lookups
CREATE INDEX IF NOT EXISTS idx_runtime_snapshots_runtime_image_id
ON runtime_snapshots(runtime_image_id);

-- Create index on state for filtering
CREATE INDEX IF NOT EXISTS idx_runtime_snapshots_state
ON runtime_snapshots(state);

-- Create unique constraint: one ready snapshot per runtime image
CREATE UNIQUE INDEX IF NOT EXISTS idx_runtime_snapshots_unique_ready
ON runtime_snapshots(runtime_image_id)
WHERE state = 'ready';
