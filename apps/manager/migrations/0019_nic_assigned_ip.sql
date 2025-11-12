-- Add assigned_ip field to vm_network_interface for static IP allocation
ALTER TABLE vm_network_interface
ADD COLUMN IF NOT EXISTS assigned_ip TEXT;

-- Index for looking up IPs in a network
CREATE INDEX IF NOT EXISTS idx_vm_nic_network_ip
ON vm_network_interface(network_id, assigned_ip)
WHERE network_id IS NOT NULL AND assigned_ip IS NOT NULL;
