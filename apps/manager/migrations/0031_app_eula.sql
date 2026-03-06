CREATE TABLE IF NOT EXISTS app_eula_acceptance (
    id           SERIAL PRIMARY KEY,
    eula_version TEXT NOT NULL,
    language     TEXT NOT NULL DEFAULT 'en',
    accepted_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
