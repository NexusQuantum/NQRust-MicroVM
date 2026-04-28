-- 0035_host_backend_kinds.sql
-- Per-host advertised list of HostBackend kinds. Manager refuses to schedule
-- a VM on a host whose kind set doesn't include the volume's backend kind.

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS supported_backend_kinds JSONB NOT NULL
    DEFAULT '["local_file"]'::jsonb;

COMMENT ON COLUMN host.supported_backend_kinds IS
  'JSON array of BackendKind db strings (e.g. ["local_file","iscsi"]) the agent advertises support for. Updated on agent registration / heartbeat.';
