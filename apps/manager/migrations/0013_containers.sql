-- Add containers support for Docker/OCI container management

CREATE TABLE containers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    image TEXT NOT NULL,
    command TEXT,
    args JSONB DEFAULT '[]'::jsonb,
    env_vars JSONB DEFAULT '{}'::jsonb,
    volumes JSONB DEFAULT '[]'::jsonb,
    port_mappings JSONB DEFAULT '[]'::jsonb,
    cpu_limit REAL,
    memory_limit_mb INTEGER,
    restart_policy TEXT DEFAULT 'no',
    state TEXT NOT NULL DEFAULT 'creating',
    host_id UUID,
    container_runtime_id TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    stopped_at TIMESTAMPTZ
);

CREATE INDEX idx_containers_state ON containers(state);
CREATE INDEX idx_containers_host_id ON containers(host_id);
CREATE INDEX idx_containers_created_at ON containers(created_at DESC);

CREATE TABLE container_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    container_id UUID NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stream TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_container_logs_container_id ON container_logs(container_id, timestamp DESC);
CREATE INDEX idx_container_logs_timestamp ON container_logs(timestamp DESC);

CREATE TABLE container_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    container_id UUID NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    cpu_percent REAL,
    memory_used_mb BIGINT,
    memory_limit_mb BIGINT,
    network_rx_bytes BIGINT,
    network_tx_bytes BIGINT,
    block_read_bytes BIGINT,
    block_write_bytes BIGINT,
    pids INTEGER,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_container_stats_container_id ON container_stats(container_id, recorded_at DESC);
CREATE INDEX idx_container_stats_recorded_at ON container_stats(recorded_at DESC);
