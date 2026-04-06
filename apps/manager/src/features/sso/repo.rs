use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

// ─── SSO Provider Repository ───────────────────────────────────────

#[derive(Clone)]
pub struct SsoProviderRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SsoProviderRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub protocol: String,
    pub enabled: bool,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret_encrypted: Option<String>,
    pub oidc_scopes: Option<String>,
    pub oidc_extra_params: Option<sqlx::types::JsonValue>,
    pub saml_idp_metadata_xml: Option<String>,
    pub saml_idp_sso_url: Option<String>,
    pub saml_idp_entity_id: Option<String>,
    pub saml_idp_certificate_pem: Option<String>,
    pub saml_sp_entity_id: Option<String>,
    pub saml_name_id_format: Option<String>,
    pub saml_sign_requests: Option<bool>,
    pub saml_sp_private_key_encrypted: Option<String>,
    pub saml_sp_certificate_pem: Option<String>,
    pub role_mapping: Option<sqlx::types::JsonValue>,
    pub role_claim_name: Option<String>,
    pub default_role: String,
    pub allow_jit_provisioning: bool,
    pub username_claim: Option<String>,
    pub email_claim: Option<String>,
    pub display_name_claim: Option<String>,
    pub display_order: i32,
    pub icon_hint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SsoProviderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List enabled providers (for login page — no secrets).
    pub async fn list_enabled(&self) -> Result<Vec<SsoProviderRow>, SsoRepoError> {
        let rows = sqlx::query_as::<_, SsoProviderRow>(
            "SELECT * FROM sso_providers WHERE enabled = true ORDER BY display_order, name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// List all providers (admin).
    pub async fn list_all(&self) -> Result<Vec<SsoProviderRow>, SsoRepoError> {
        let rows = sqlx::query_as::<_, SsoProviderRow>(
            "SELECT * FROM sso_providers ORDER BY display_order, name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<SsoProviderRow, SsoRepoError> {
        sqlx::query_as::<_, SsoProviderRow>("SELECT * FROM sso_providers WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(SsoRepoError::ProviderNotFound)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<SsoProviderRow, SsoRepoError> {
        sqlx::query_as::<_, SsoProviderRow>("SELECT * FROM sso_providers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(SsoRepoError::ProviderNotFound)
    }

    pub async fn create(
        &self,
        req: &nexus_types::CreateSsoProviderRequest,
        encrypted_secret: Option<&str>,
    ) -> Result<SsoProviderRow, SsoRepoError> {
        let row = sqlx::query_as::<_, SsoProviderRow>(
            r#"
            INSERT INTO sso_providers (
                id, name, slug, protocol, oidc_issuer_url, oidc_client_id,
                oidc_client_secret_encrypted, oidc_scopes,
                saml_idp_metadata_xml, saml_idp_sso_url, saml_idp_entity_id,
                saml_idp_certificate_pem, saml_sp_entity_id,
                role_mapping, role_claim_name, default_role,
                allow_jit_provisioning, username_claim, email_claim,
                display_name_claim, icon_hint, display_order
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22
            )
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&req.name)
        .bind(&req.slug)
        .bind(req.protocol.to_string())
        .bind(&req.oidc_issuer_url)
        .bind(&req.oidc_client_id)
        .bind(encrypted_secret)
        .bind(&req.oidc_scopes)
        .bind(&req.saml_idp_metadata_xml)
        .bind(&req.saml_idp_sso_url)
        .bind(&req.saml_idp_entity_id)
        .bind(&req.saml_idp_certificate_pem)
        .bind(&req.saml_sp_entity_id)
        .bind(&req.role_mapping)
        .bind(&req.role_claim_name)
        .bind(req.default_role.map(|r| r.as_str()).unwrap_or("viewer"))
        .bind(req.allow_jit_provisioning.unwrap_or(true))
        .bind(&req.username_claim)
        .bind(&req.email_claim)
        .bind(&req.display_name_claim)
        .bind(&req.icon_hint)
        .bind(req.display_order.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: &nexus_types::UpdateSsoProviderRequest,
        encrypted_secret: Option<&str>,
    ) -> Result<SsoProviderRow, SsoRepoError> {
        // Fetch current, apply updates
        let current = self.get_by_id(id).await?;

        let row = sqlx::query_as::<_, SsoProviderRow>(
            r#"
            UPDATE sso_providers SET
                name = $2, enabled = $3,
                oidc_issuer_url = $4, oidc_client_id = $5,
                oidc_client_secret_encrypted = COALESCE($6, oidc_client_secret_encrypted),
                oidc_scopes = $7,
                saml_idp_metadata_xml = $8, saml_idp_sso_url = $9,
                saml_idp_entity_id = $10, saml_idp_certificate_pem = $11,
                saml_sp_entity_id = $12,
                role_mapping = $13, role_claim_name = $14, default_role = $15,
                allow_jit_provisioning = $16, username_claim = $17,
                email_claim = $18, display_name_claim = $19,
                icon_hint = $20, display_order = $21,
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(req.name.as_deref().unwrap_or(&current.name))
        .bind(req.enabled.unwrap_or(current.enabled))
        .bind(
            req.oidc_issuer_url
                .as_deref()
                .or(current.oidc_issuer_url.as_deref()),
        )
        .bind(
            req.oidc_client_id
                .as_deref()
                .or(current.oidc_client_id.as_deref()),
        )
        .bind(encrypted_secret)
        .bind(
            req.oidc_scopes
                .as_deref()
                .or(current.oidc_scopes.as_deref()),
        )
        .bind(
            req.saml_idp_metadata_xml
                .as_deref()
                .or(current.saml_idp_metadata_xml.as_deref()),
        )
        .bind(
            req.saml_idp_sso_url
                .as_deref()
                .or(current.saml_idp_sso_url.as_deref()),
        )
        .bind(
            req.saml_idp_entity_id
                .as_deref()
                .or(current.saml_idp_entity_id.as_deref()),
        )
        .bind(
            req.saml_idp_certificate_pem
                .as_deref()
                .or(current.saml_idp_certificate_pem.as_deref()),
        )
        .bind(
            req.saml_sp_entity_id
                .as_deref()
                .or(current.saml_sp_entity_id.as_deref()),
        )
        .bind(req.role_mapping.as_ref().or(current.role_mapping.as_ref()))
        .bind(
            req.role_claim_name
                .as_deref()
                .or(current.role_claim_name.as_deref()),
        )
        .bind(
            req.default_role
                .map(|r| r.as_str())
                .unwrap_or(&current.default_role),
        )
        .bind(
            req.allow_jit_provisioning
                .unwrap_or(current.allow_jit_provisioning),
        )
        .bind(
            req.username_claim
                .as_deref()
                .or(current.username_claim.as_deref()),
        )
        .bind(
            req.email_claim
                .as_deref()
                .or(current.email_claim.as_deref()),
        )
        .bind(
            req.display_name_claim
                .as_deref()
                .or(current.display_name_claim.as_deref()),
        )
        .bind(req.icon_hint.as_deref().or(current.icon_hint.as_deref()))
        .bind(req.display_order.unwrap_or(current.display_order))
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), SsoRepoError> {
        let result = sqlx::query("DELETE FROM sso_providers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(SsoRepoError::ProviderNotFound);
        }
        Ok(())
    }
}

// ─── User Identity Repository ──────────────────────────────────────

#[derive(Clone)]
pub struct UserIdentityRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserIdentityRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_id: Uuid,
    pub external_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub raw_claims: Option<sqlx::types::JsonValue>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_external_id(
        &self,
        provider_id: Uuid,
        external_id: &str,
    ) -> Result<UserIdentityRow, SsoRepoError> {
        sqlx::query_as::<_, UserIdentityRow>(
            "SELECT * FROM user_identities WHERE provider_id = $1 AND external_id = $2",
        )
        .bind(provider_id)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(SsoRepoError::IdentityNotFound)
    }

    pub async fn create_link(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
        external_id: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        raw_claims: Option<serde_json::Value>,
    ) -> Result<UserIdentityRow, SsoRepoError> {
        let row = sqlx::query_as::<_, UserIdentityRow>(
            r#"
            INSERT INTO user_identities (id, user_id, provider_id, external_id, email, display_name, raw_claims, last_login_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now())
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(provider_id)
        .bind(external_id)
        .bind(email)
        .bind(display_name)
        .bind(raw_claims)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_last_login(&self, id: Uuid) -> Result<(), SsoRepoError> {
        sqlx::query(
            "UPDATE user_identities SET last_login_at = now(), updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<UserIdentityRow>, SsoRepoError> {
        let rows = sqlx::query_as::<_, UserIdentityRow>(
            "SELECT * FROM user_identities WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ─── Auth State Repository ─────────────────────────────────────────

#[derive(Clone)]
pub struct AuthStateRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthStateRow {
    pub id: Uuid,
    pub state_token: String,
    pub provider_id: Uuid,
    pub pkce_verifier_encrypted: Option<String>,
    pub nonce: Option<String>,
    pub redirect_after_login: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthStateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new auth state and return the state token.
    pub async fn create_state(
        &self,
        provider_id: Uuid,
        state_token: &str,
        pkce_verifier_encrypted: Option<&str>,
        nonce: Option<&str>,
        redirect_after: Option<&str>,
    ) -> Result<(), SsoRepoError> {
        let expires_at = Utc::now() + Duration::minutes(10);
        sqlx::query(
            r#"
            INSERT INTO sso_auth_states (id, state_token, provider_id, pkce_verifier_encrypted, nonce, redirect_after_login, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(state_token)
        .bind(provider_id)
        .bind(pkce_verifier_encrypted)
        .bind(nonce)
        .bind(redirect_after.unwrap_or("/"))
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Consume (fetch and delete) a state token. Returns None if expired or not found.
    pub async fn consume_state(&self, state_token: &str) -> Result<AuthStateRow, SsoRepoError> {
        let row = sqlx::query_as::<_, AuthStateRow>(
            "DELETE FROM sso_auth_states WHERE state_token = $1 AND expires_at > now() RETURNING *",
        )
        .bind(state_token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(SsoRepoError::InvalidState)?;
        Ok(row)
    }

    /// Clean up expired states.
    pub async fn cleanup_expired(&self) -> Result<u64, SsoRepoError> {
        let result = sqlx::query("DELETE FROM sso_auth_states WHERE expires_at < now()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

// ─── Errors ────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SsoRepoError {
    #[error("SSO provider not found")]
    ProviderNotFound,
    #[error("identity not found")]
    IdentityNotFound,
    #[error("invalid or expired auth state")]
    InvalidState,
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
}
