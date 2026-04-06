-- SSO Identity Provider configurations
CREATE TABLE IF NOT EXISTS sso_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Human-readable name and URL-safe slug
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,

    -- Provider protocol
    protocol TEXT NOT NULL CHECK (protocol IN ('oidc', 'saml')),
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- OIDC-specific configuration (NULL for SAML providers)
    oidc_issuer_url TEXT,
    oidc_client_id TEXT,
    oidc_client_secret_encrypted TEXT,
    oidc_scopes TEXT DEFAULT 'openid profile email',
    oidc_extra_params JSONB DEFAULT '{}',

    -- SAML-specific configuration (NULL for OIDC providers)
    saml_idp_metadata_xml TEXT,
    saml_idp_sso_url TEXT,
    saml_idp_entity_id TEXT,
    saml_idp_certificate_pem TEXT,
    saml_sp_entity_id TEXT,
    saml_name_id_format TEXT DEFAULT 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
    saml_sign_requests BOOLEAN DEFAULT false,
    saml_sp_private_key_encrypted TEXT,
    saml_sp_certificate_pem TEXT,

    -- Role mapping: {"admin": ["IdP-Admins"], "user": ["IdP-Users"], "viewer": ["IdP-Readonly"]}
    role_mapping JSONB DEFAULT '{}',
    role_claim_name TEXT DEFAULT 'groups',
    default_role TEXT NOT NULL DEFAULT 'viewer' CHECK (default_role IN ('admin', 'user', 'viewer')),

    -- JIT provisioning
    allow_jit_provisioning BOOLEAN NOT NULL DEFAULT true,

    -- Claim/attribute name mapping
    username_claim TEXT DEFAULT 'preferred_username',
    email_claim TEXT DEFAULT 'email',
    display_name_claim TEXT DEFAULT 'name',

    -- Display
    display_order INT NOT NULL DEFAULT 0,
    icon_hint TEXT DEFAULT 'generic',

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sso_providers_slug ON sso_providers(slug);
CREATE INDEX IF NOT EXISTS idx_sso_providers_enabled ON sso_providers(enabled);

-- Links external identities to internal user accounts
CREATE TABLE IF NOT EXISTS user_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES sso_providers(id) ON DELETE CASCADE,

    -- External identifier from IdP (OIDC 'sub' claim or SAML NameID)
    external_id TEXT NOT NULL,

    email TEXT,
    display_name TEXT,
    raw_claims JSONB DEFAULT '{}',

    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT uq_identity_provider_external UNIQUE (provider_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_user_identities_user_id ON user_identities(user_id);
CREATE INDEX IF NOT EXISTS idx_user_identities_provider_external ON user_identities(provider_id, external_id);

-- OAuth2 state parameter storage for CSRF protection (short-lived)
CREATE TABLE IF NOT EXISTS sso_auth_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state_token TEXT NOT NULL UNIQUE,
    provider_id UUID NOT NULL REFERENCES sso_providers(id) ON DELETE CASCADE,
    pkce_verifier_encrypted TEXT,
    nonce TEXT,
    redirect_after_login TEXT DEFAULT '/',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sso_auth_states_token ON sso_auth_states(state_token);
CREATE INDEX IF NOT EXISTS idx_sso_auth_states_expires ON sso_auth_states(expires_at);
