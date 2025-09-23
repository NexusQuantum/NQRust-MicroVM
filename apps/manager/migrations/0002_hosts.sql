CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS host (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    addr TEXT NOT NULL UNIQUE,
    capabilities_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE vm ADD COLUMN host_id UUID;
ALTER TABLE vm
    ADD CONSTRAINT fk_vm_host
    FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE RESTRICT;

INSERT INTO host (id, name, addr, capabilities_json, last_seen_at)
SELECT DISTINCT
    gen_random_uuid(),
    vm.host_addr,
    vm.host_addr,
    '{}'::jsonb,
    now()
FROM vm
WHERE vm.host_addr IS NOT NULL
ON CONFLICT (addr) DO NOTHING;

UPDATE vm
SET host_id = host.id
FROM host
WHERE host.addr = vm.host_addr;

ALTER TABLE vm ALTER COLUMN host_id SET NOT NULL;
ALTER TABLE vm DROP COLUMN host_addr;

CREATE INDEX IF NOT EXISTS idx_vm_host ON vm(host_id);
