-- Add port forwarding table
-- Migration: 0025_port_forwards.sql

-- Create port_forward table for managing VM port forwards
CREATE TABLE IF NOT EXISTS port_forward (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
  host_port INTEGER NOT NULL CHECK (host_port BETWEEN 1 AND 65535),
  guest_port INTEGER NOT NULL CHECK (guest_port BETWEEN 1 AND 65535),
  protocol TEXT NOT NULL DEFAULT 'tcp' CHECK (protocol IN ('tcp', 'udp')),
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- No two VMs can claim the same host port + protocol combination
  CONSTRAINT unique_host_port_protocol UNIQUE (host_port, protocol)
);

-- Create index for fast lookups by vm_id
CREATE INDEX IF NOT EXISTS idx_port_forward_vm ON port_forward(vm_id);
