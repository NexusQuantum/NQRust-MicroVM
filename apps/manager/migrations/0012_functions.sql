-- Functions table
-- Each function runs in its own dedicated MicroVM for isolation and monitoring
CREATE TABLE IF NOT EXISTS function (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    runtime TEXT NOT NULL,  -- node, python, go, rust
    code TEXT NOT NULL,
    handler TEXT NOT NULL,
    timeout_seconds INT NOT NULL DEFAULT 30,
    memory_mb INT NOT NULL DEFAULT 128,
    vcpu INT NOT NULL DEFAULT 1,
    env_vars JSONB,
    vm_id UUID REFERENCES vm(id) ON DELETE SET NULL,  -- Dedicated MicroVM
    guest_ip TEXT,  -- IP of the function's VM
    port INT NOT NULL DEFAULT 3000,  -- Port where runtime server listens
    state TEXT NOT NULL DEFAULT 'creating',  -- creating, ready, error, stopped
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_invoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_function_name ON function(name);
CREATE INDEX IF NOT EXISTS idx_function_runtime ON function(runtime);
CREATE INDEX IF NOT EXISTS idx_function_vm_id ON function(vm_id);
CREATE INDEX IF NOT EXISTS idx_function_state ON function(state);

-- Function invocations/logs table
CREATE TABLE IF NOT EXISTS function_invocation (
    id UUID PRIMARY KEY,
    function_id UUID NOT NULL REFERENCES function(id) ON DELETE CASCADE,
    status TEXT NOT NULL,  -- success, error, timeout
    duration_ms BIGINT NOT NULL,
    memory_used_mb INT,
    request_id TEXT NOT NULL,
    event JSONB NOT NULL,
    response JSONB,
    logs TEXT[],
    error TEXT,
    invoked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_function_invocation_function_id ON function_invocation(function_id);
CREATE INDEX IF NOT EXISTS idx_function_invocation_status ON function_invocation(status);
CREATE INDEX IF NOT EXISTS idx_function_invocation_invoked_at ON function_invocation(invoked_at DESC);
CREATE INDEX IF NOT EXISTS idx_function_invocation_request_id ON function_invocation(request_id);
