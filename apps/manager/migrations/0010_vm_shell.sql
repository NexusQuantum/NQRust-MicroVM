CREATE TABLE IF NOT EXISTS vm_shell_credential (
    id UUID PRIMARY KEY,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_vm_shell_credential_vm_id
    ON vm_shell_credential(vm_id);

CREATE TABLE IF NOT EXISTS vm_shell_session (
    id UUID PRIMARY KEY,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_vm_shell_session_vm_id
    ON vm_shell_session(vm_id);

CREATE INDEX IF NOT EXISTS idx_vm_shell_session_expires_at
    ON vm_shell_session(expires_at);





