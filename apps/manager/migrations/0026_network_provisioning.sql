-- Network provisioning: change type enum, add status/managed/dhcp columns
-- Migration: 0026_network_provisioning.sql

-- Drop old constraints that reference the old type values
ALTER TABLE network DROP CONSTRAINT IF EXISTS vlan_id_required_for_vlan;
ALTER TABLE network DROP CONSTRAINT IF EXISTS network_type_check;

-- Add new columns
ALTER TABLE network ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE network ADD COLUMN IF NOT EXISTS error_message TEXT;
ALTER TABLE network ADD COLUMN IF NOT EXISTS managed BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE network ADD COLUMN IF NOT EXISTS dhcp_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE network ADD COLUMN IF NOT EXISTS dhcp_range_start TEXT;
ALTER TABLE network ADD COLUMN IF NOT EXISTS dhcp_range_end TEXT;

-- Migrate existing type values: "bridge" → "bridged", "vlan" → "nat"
UPDATE network SET type = 'bridged' WHERE type = 'bridge';
UPDATE network SET type = 'nat' WHERE type = 'vlan';

-- Add new constraints
ALTER TABLE network ADD CONSTRAINT network_type_check
  CHECK (type IN ('nat', 'bridged', 'isolated'));
ALTER TABLE network ADD CONSTRAINT network_status_check
  CHECK (status IN ('pending', 'provisioning', 'active', 'error', 'deleting'));
ALTER TABLE network ADD CONSTRAINT network_vlan_range
  CHECK (vlan_id IS NULL OR (vlan_id BETWEEN 1 AND 4094));

-- Index for status queries
CREATE INDEX IF NOT EXISTS idx_network_status ON network(status);
