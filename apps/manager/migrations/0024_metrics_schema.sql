-- Migration: Create metrics schema for time-series metrics collection
-- Stores host, VM, and container metrics sampled every 10 seconds.
-- Isolated in its own schema (same pattern as audit schema).

CREATE SCHEMA IF NOT EXISTS metrics;

-- Host metrics (CPU, memory, disk from agent heartbeat data)
CREATE TABLE metrics.host_metrics (
    id BIGSERIAL PRIMARY KEY,
    host_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    cpu_usage_percent DOUBLE PRECISION,
    memory_used_mb DOUBLE PRECISION,
    memory_total_mb DOUBLE PRECISION,
    disk_used_gb DOUBLE PRECISION,
    disk_total_gb DOUBLE PRECISION
);
CREATE INDEX idx_host_metrics_lookup ON metrics.host_metrics (host_id, recorded_at DESC);

-- VM metrics (from guest agent HTTP endpoint)
CREATE TABLE metrics.vm_metrics (
    id BIGSERIAL PRIMARY KEY,
    vm_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    cpu_usage_percent DOUBLE PRECISION,
    memory_usage_percent DOUBLE PRECISION,
    memory_used_kb BIGINT,
    memory_total_kb BIGINT,
    load_average DOUBLE PRECISION
);
CREATE INDEX idx_vm_metrics_lookup ON metrics.vm_metrics (vm_id, recorded_at DESC);

-- Container metrics (from Docker stats API)
CREATE TABLE metrics.container_metrics (
    id BIGSERIAL PRIMARY KEY,
    container_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    cpu_percent DOUBLE PRECISION,
    memory_used_mb DOUBLE PRECISION,
    memory_limit_mb DOUBLE PRECISION,
    network_rx_bytes BIGINT,
    network_tx_bytes BIGINT,
    block_read_bytes BIGINT,
    block_write_bytes BIGINT,
    pids INTEGER
);
CREATE INDEX idx_container_metrics_lookup ON metrics.container_metrics (container_id, recorded_at DESC);

-- Purge function — called by collector after each cycle to enforce 7-day retention
CREATE OR REPLACE FUNCTION metrics.purge_old_metrics(retention INTERVAL DEFAULT '7 days')
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM metrics.host_metrics      WHERE recorded_at < now() - retention;
    DELETE FROM metrics.vm_metrics        WHERE recorded_at < now() - retention;
    DELETE FROM metrics.container_metrics WHERE recorded_at < now() - retention;
END;
$$;

-- Read-only role for external dashboards (same pattern as audit_reader)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'metrics_reader') THEN
        CREATE ROLE metrics_reader NOLOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA metrics TO metrics_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA metrics TO metrics_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA metrics GRANT SELECT ON TABLES TO metrics_reader;
