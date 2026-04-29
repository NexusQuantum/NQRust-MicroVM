-- 0036_backup.sql — Chunked encrypted backup pipeline.

CREATE TABLE IF NOT EXISTS backup_target (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  endpoint TEXT NOT NULL,
  region TEXT,
  bucket TEXT NOT NULL,
  prefix TEXT NOT NULL DEFAULT '',
  access_key_id TEXT NOT NULL,
  encrypted_secret_access_key BYTEA NOT NULL,
  encrypted_target_key BYTEA NOT NULL,
  gc_hour SMALLINT NOT NULL DEFAULT 3 CHECK (gc_hour BETWEEN 0 AND 23),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS backup (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_volume_id UUID REFERENCES volume(id) ON DELETE SET NULL,
  source_snapshot_id UUID,
  target_id UUID NOT NULL REFERENCES backup_target(id),
  manifest_object_key TEXT,
  size_bytes BIGINT NOT NULL DEFAULT 0,
  unique_bytes BIGINT NOT NULL DEFAULT 0,
  chunk_count BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed', 'pruning')),
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_backup_volume ON backup(source_volume_id);
CREATE INDEX IF NOT EXISTS idx_backup_target ON backup(target_id);
CREATE INDEX IF NOT EXISTS idx_backup_status_updated ON backup(status, updated_at)
  WHERE status = 'running';

ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_cron TEXT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_retain_count INT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_target_id UUID
  REFERENCES backup_target(id);

CREATE TABLE IF NOT EXISTS backup_gc_run (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  target_id UUID NOT NULL REFERENCES backup_target(id) ON DELETE CASCADE,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  bytes_freed BIGINT NOT NULL DEFAULT 0,
  chunks_deleted BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed')),
  error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_backup_gc_run_target ON backup_gc_run(target_id, started_at DESC);

COMMENT ON COLUMN backup_target.encrypted_secret_access_key IS
  'AES-GCM(envelope_key) over the S3 secret access key.';
COMMENT ON COLUMN backup_target.encrypted_target_key IS
  'AES-GCM(envelope_key) over the per-target XChaCha20-Poly1305 key used for chunk + manifest encryption.';
COMMENT ON COLUMN backup.unique_bytes IS
  'Post-dedup ciphertext bytes that this backup actually wrote (chunks not skipped by HEAD).';
