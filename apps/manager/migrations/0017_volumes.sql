-- Add central volume registry
-- Migration: 0017_volumes.sql

-- Create volume table for centralized volume management
CREATE TABLE IF NOT EXISTS volume (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  path TEXT NOT NULL,
  size_bytes BIGINT NOT NULL,
  type TEXT NOT NULL DEFAULT 'raw' CHECK (type IN ('raw', 'qcow2', 'ext4')),
  status TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'attached', 'creating', 'error')),
  host_id UUID NOT NULL REFERENCES host(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- Constraints
  CONSTRAINT unique_volume_name_per_host UNIQUE (host_id, name),
  CONSTRAINT unique_volume_path UNIQUE (path),
  CONSTRAINT positive_size CHECK (size_bytes > 0)
);

-- Create volume_attachment table to track which VMs use which volumes
CREATE TABLE IF NOT EXISTS volume_attachment (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  volume_id UUID NOT NULL REFERENCES volume(id) ON DELETE CASCADE,
  vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
  drive_id TEXT NOT NULL,  -- Firecracker drive ID (e.g., "rootfs", "data1")
  attached_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- Constraints
  CONSTRAINT unique_volume_attachment UNIQUE (volume_id, vm_id),
  CONSTRAINT unique_drive_per_vm UNIQUE (vm_id, drive_id)
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_volume_host ON volume(host_id);
CREATE INDEX IF NOT EXISTS idx_volume_status ON volume(status);
CREATE INDEX IF NOT EXISTS idx_volume_attachment_volume ON volume_attachment(volume_id);
CREATE INDEX IF NOT EXISTS idx_volume_attachment_vm ON volume_attachment(vm_id);

-- Migrate existing vm_drive data to volumes
-- Only migrate drives with size_bytes (auto-provisioned drives)
INSERT INTO volume (name, path, size_bytes, type, status, host_id)
SELECT
  CONCAT('vol-', vd.drive_id, '-', SUBSTRING(vd.vm_id::TEXT FROM 1 FOR 8)),
  vd.path_on_host,
  COALESCE(vd.size_bytes, 10737418240), -- Default to 10GB if NULL
  'raw',
  'attached',
  v.host_id
FROM vm_drive vd
JOIN vm v ON vd.vm_id = v.id
WHERE vd.size_bytes IS NOT NULL  -- Only auto-provisioned drives
  AND v.host_id IS NOT NULL
ON CONFLICT (path) DO NOTHING;

-- Create volume attachments for migrated volumes
INSERT INTO volume_attachment (volume_id, vm_id, drive_id)
SELECT
  vol.id,
  vd.vm_id,
  vd.drive_id
FROM vm_drive vd
JOIN vm v ON vd.vm_id = v.id
JOIN volume vol ON vol.path = vd.path_on_host
WHERE vd.size_bytes IS NOT NULL
  AND v.host_id IS NOT NULL
ON CONFLICT (vm_id, drive_id) DO NOTHING;

-- Add column to vm_drive to track if it's managed as a volume
ALTER TABLE vm_drive ADD COLUMN IF NOT EXISTS volume_id UUID REFERENCES volume(id) ON DELETE SET NULL;

-- Link existing drives to volumes
UPDATE vm_drive vd
SET volume_id = vol.id
FROM volume vol
WHERE vd.path_on_host = vol.path;

-- Create index for volume_id in vm_drive
CREATE INDEX IF NOT EXISTS idx_vm_drive_volume ON vm_drive(volume_id);
