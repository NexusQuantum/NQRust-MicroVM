-- 0034_storage_backends.sql
-- Pluggable storage backend abstraction. See
-- docs/superpowers/specs/2026-04-28-storage-hci-design.md.

-- 1. Backend instance registry. TOML is source of truth on startup; this
-- table caches what the manager loaded so the rest of the system has a
-- stable id to reference.
CREATE TABLE IF NOT EXISTS storage_backend (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  capabilities_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_default BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

-- At most one default backend.
CREATE UNIQUE INDEX IF NOT EXISTS one_default_backend
  ON storage_backend (is_default) WHERE is_default = true;

-- 2. Seed the localfile-default backend so existing volumes have something
-- to point at. Capabilities reflect the LocalFile impl: clone-from-image yes
-- (it's just fs::copy), snapshot/concurrent/migration no.
INSERT INTO storage_backend (name, kind, capabilities_json, is_default)
VALUES (
  'localfile-default',
  'local_file',
  '{"supports_native_snapshots": false, "supports_concurrent_attach": false, "supports_live_migration": false, "supports_clone_from_image": true}'::jsonb,
  true
)
ON CONFLICT (name) DO NOTHING;

-- 3. Volume schema changes.
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backend_id UUID REFERENCES storage_backend(id);
UPDATE volume
   SET backend_id = (SELECT id FROM storage_backend WHERE name = 'localfile-default')
 WHERE backend_id IS NULL;
ALTER TABLE volume ALTER COLUMN backend_id SET NOT NULL;
ALTER TABLE volume ALTER COLUMN host_id DROP NOT NULL;

COMMENT ON COLUMN volume.host_id IS
  'Home host for host-pinned volumes (LocalFile). NULL for network-attached backends.';
COMMENT ON COLUMN volume.path IS
  'Backend-defined locator. LocalFile: filesystem path. Iscsi: IQN+LUN. Unique within a backend instance.';

CREATE INDEX IF NOT EXISTS idx_volume_backend ON volume(backend_id);

-- 4. Single-attach enforcement + audit trail. detached_at NULL = active.
ALTER TABLE volume_attachment ADD COLUMN IF NOT EXISTS detached_at TIMESTAMPTZ;

-- The original unique constraint UNIQUE (volume_id, vm_id) does not prevent a
-- volume being attached to a SECOND vm. The new partial unique index does:
-- at most one row with detached_at IS NULL per volume.
DROP INDEX IF EXISTS volume_one_active_attachment;
CREATE UNIQUE INDEX volume_one_active_attachment
  ON volume_attachment(volume_id) WHERE detached_at IS NULL;
