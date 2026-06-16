-- QEMU CPU model selection (e.g. "host", "kvm64", "x86-64-v3", "EPYC").
-- NULL means the backend default ("host"). Forward-only nullable column.
ALTER TABLE vm ADD COLUMN IF NOT EXISTS cpu_type TEXT;
