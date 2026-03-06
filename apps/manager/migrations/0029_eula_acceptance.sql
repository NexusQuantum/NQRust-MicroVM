CREATE TABLE eula_acceptances (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    eula_version TEXT NOT NULL,
    language     TEXT NOT NULL DEFAULT 'en',
    ip_address   TEXT,
    accepted_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_eula_acceptances_user_id ON eula_acceptances(user_id);
CREATE UNIQUE INDEX idx_eula_acceptances_user_version ON eula_acceptances(user_id, eula_version);
