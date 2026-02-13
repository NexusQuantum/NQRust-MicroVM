-- VXLAN overlay networking support
-- Migration: 0027_vxlan_networks.sql

-- Allow VXLAN as a network type
ALTER TABLE network DROP CONSTRAINT IF EXISTS network_type_check;
ALTER TABLE network ADD CONSTRAINT network_type_check
  CHECK (type IN ('nat', 'bridged', 'isolated', 'vxlan'));

-- VXLAN Network Identifier (24-bit, 1-16777215)
ALTER TABLE network ADD COLUMN IF NOT EXISTS vni INTEGER;
ALTER TABLE network ADD CONSTRAINT network_vni_range
  CHECK (vni IS NULL OR (vni BETWEEN 1 AND 16777215));
CREATE UNIQUE INDEX IF NOT EXISTS idx_network_vni ON network(vni) WHERE vni IS NOT NULL;

-- Junction table: which hosts participate in a VXLAN network
CREATE TABLE IF NOT EXISTS network_host (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  network_id UUID NOT NULL REFERENCES network(id) ON DELETE CASCADE,
  host_id UUID NOT NULL REFERENCES host(id) ON DELETE CASCADE,
  vtep_ip TEXT NOT NULL,
  is_gateway BOOLEAN NOT NULL DEFAULT false,
  status TEXT NOT NULL DEFAULT 'provisioning',
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(network_id, host_id)
);

CREATE INDEX IF NOT EXISTS idx_network_host_network ON network_host(network_id);
CREATE INDEX IF NOT EXISTS idx_network_host_host ON network_host(host_id);
