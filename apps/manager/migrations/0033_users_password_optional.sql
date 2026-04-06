-- Make password_hash optional for SSO-only users
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Track how the user authenticates
ALTER TABLE users ADD COLUMN IF NOT EXISTS auth_source TEXT NOT NULL DEFAULT 'local'
    CHECK (auth_source IN ('local', 'sso', 'both'));

-- Email for SSO users (may also be set manually for local users)
ALTER TABLE users ADD COLUMN IF NOT EXISTS email TEXT;
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
