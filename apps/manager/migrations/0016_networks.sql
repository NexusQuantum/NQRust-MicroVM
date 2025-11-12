-- Add network management tables
-- Migration: 0016_networks.sql

-- Create network table for managing bridges and VLANs
CREATE TABLE IF NOT EXISTS network (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  type TEXT NOT NULL CHECK (type IN ('bridge', 'vlan')),
  vlan_id INTEGER,
  bridge_name TEXT NOT NULL,
  host_id UUID REFERENCES host(id) ON DELETE CASCADE,
  cidr TEXT,  -- Optional CIDR for network documentation
  gateway TEXT,  -- Optional gateway IP
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- Constraints
  CONSTRAINT vlan_id_required_for_vlan CHECK (
    (type = 'vlan' AND vlan_id IS NOT NULL AND vlan_id BETWEEN 1 AND 4094) OR
    (type = 'bridge' AND vlan_id IS NULL)
  ),
  CONSTRAINT unique_network_name_per_host UNIQUE (host_id, name),
  CONSTRAINT unique_vlan_per_host UNIQUE (host_id, bridge_name, vlan_id)
);

-- Add network_id to vm_network_interface table
ALTER TABLE vm_network_interface ADD COLUMN IF NOT EXISTS network_id UUID REFERENCES network(id) ON DELETE SET NULL;

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_network_host ON network(host_id);
CREATE INDEX IF NOT EXISTS idx_network_type ON network(type);
CREATE INDEX IF NOT EXISTS idx_network_bridge ON network(bridge_name);
CREATE INDEX IF NOT EXISTS idx_vm_nic_network ON vm_network_interface(network_id);

-- Create default bridge network for each existing host
INSERT INTO network (name, description, type, bridge_name, host_id)
SELECT
  'default-bridge',
  'Default bridge network',
  'bridge',
  COALESCE((capabilities_json->>'bridge')::TEXT, 'fcbr0'),
  id
FROM host
ON CONFLICT DO NOTHING;

-- Update existing vm_network_interface rows to point to default network
UPDATE vm_network_interface AS vni
SET network_id = n.id
FROM vm v
JOIN host h ON v.host_id = h.id
JOIN network n ON n.host_id = h.id AND n.name = 'default-bridge'
WHERE vni.vm_id = v.id AND vni.network_id IS NULL;
