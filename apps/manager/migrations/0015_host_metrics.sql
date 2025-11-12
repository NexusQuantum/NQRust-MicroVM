-- Add host metrics tracking columns
-- Migration: 0015_host_metrics.sql

-- Add metrics columns to host table
ALTER TABLE host ADD COLUMN IF NOT EXISTS total_cpus INTEGER;
ALTER TABLE host ADD COLUMN IF NOT EXISTS total_memory_mb BIGINT;
ALTER TABLE host ADD COLUMN IF NOT EXISTS total_disk_gb BIGINT;
ALTER TABLE host ADD COLUMN IF NOT EXISTS used_disk_gb BIGINT;
ALTER TABLE host ADD COLUMN IF NOT EXISTS last_metrics_at TIMESTAMPTZ;

-- Create index for efficient queries on last_metrics_at
CREATE INDEX IF NOT EXISTS idx_host_last_metrics ON host(last_metrics_at);

-- Update existing hosts with default values (will be populated by next heartbeat)
UPDATE host SET
  total_cpus = 0,
  total_memory_mb = 0,
  total_disk_gb = 0,
  used_disk_gb = 0,
  last_metrics_at = now()
WHERE total_cpus IS NULL;
