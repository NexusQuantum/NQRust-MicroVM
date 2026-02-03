-- Create dedicated schema for audit/logging data.
-- Isolates audit logs from operational tables (vm, containers, function, host, etc.)
-- so the audit schema can be safely exposed to external applications.

CREATE SCHEMA IF NOT EXISTS audit;

-- Move audit_logs table from public to audit schema.
-- This is atomic — indexes and constraints move with the table.
-- The FK to public.users(id) continues to work cross-schema.
ALTER TABLE public.audit_logs SET SCHEMA audit;

-- Create a read-only role for external apps that need audit log access.
-- Usage: CREATE USER some_app WITH PASSWORD 'xxx'; GRANT audit_reader TO some_app;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_reader') THEN
        CREATE ROLE audit_reader NOLOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA audit TO audit_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA audit TO audit_reader;

-- Ensure future tables in the audit schema also get SELECT for audit_reader
ALTER DEFAULT PRIVILEGES IN SCHEMA audit GRANT SELECT ON TABLES TO audit_reader;
