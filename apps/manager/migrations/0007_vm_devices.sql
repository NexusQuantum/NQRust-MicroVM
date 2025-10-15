CREATE TABLE IF NOT EXISTS vm_drive (
    id UUID PRIMARY KEY,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    drive_id TEXT NOT NULL,
    path_on_host TEXT NOT NULL,
    is_root_device BOOLEAN NOT NULL DEFAULT false,
    is_read_only BOOLEAN NOT NULL DEFAULT false,
    cache_type TEXT,
    io_engine TEXT,
    rate_limiter JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(vm_id, drive_id)
);

CREATE INDEX IF NOT EXISTS idx_vm_drive_vm_id ON vm_drive(vm_id);

CREATE TABLE IF NOT EXISTS vm_network_interface (
    id UUID PRIMARY KEY,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    iface_id TEXT NOT NULL,
    host_dev_name TEXT NOT NULL,
    guest_mac TEXT,
    rx_rate_limiter JSONB,
    tx_rate_limiter JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(vm_id, iface_id)
);

CREATE INDEX IF NOT EXISTS idx_vm_nic_vm_id ON vm_network_interface(vm_id);

