use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub avatar_path: Option<String>,
    pub timezone: Option<String>,
    pub theme: Option<String>,
    pub preferences: Option<sqlx::types::JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    /// Convert role string to Role enum
    pub fn get_role(&self) -> nexus_types::Role {
        self.role.parse().unwrap_or(nexus_types::Role::User)
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub username: String,
    pub role: nexus_types::Role,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn hash_password(password: &str) -> Result<String, UserRepoError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| UserRepoError::HashingError(e.to_string()))?
            .to_string();
        Ok(password_hash)
    }

    fn verify_password_hash(password: &str, hash: &str) -> Result<bool, UserRepoError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| UserRepoError::HashingError(e.to_string()))?;
        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let hash = hasher.finalize();
        general_purpose::STANDARD.encode(hash.as_slice())
    }

    fn generate_token() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        general_purpose::STANDARD.encode(&bytes)
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        role: nexus_types::Role,
    ) -> Result<UserRow, UserRepoError> {
        let password_hash = Self::hash_password(password)?;
        let role_str = role.as_str();

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (id, username, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, password_hash, role, last_login_at, avatar_path, timezone, theme, preferences, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(username)
        .bind(password_hash)
        .bind(role_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_by_username(&self, username: &str) -> Result<UserRow, UserRepoError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, role, last_login_at, avatar_path, timezone, theme, preferences, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(UserRepoError::UserNotFound)?;

        Ok(row)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<UserRow, UserRepoError> {
        debug!(user_id = ?id, "fetching user by id from database");

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, role, last_login_at, avatar_path, timezone, theme, preferences, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!(?e, user_id = ?id, "database error fetching user by id");
            e
        })?
        .ok_or_else(|| {
            debug!(user_id = ?id, "user not found in database");
            UserRepoError::UserNotFound
        })?;

        debug!(user_id = ?id, username = ?row.username, "user fetched successfully");
        Ok(row)
    }

    pub async fn list(&self) -> Result<Vec<UserRow>, UserRepoError> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, role, last_login_at, avatar_path, timezone, theme, preferences, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn update(
        &self,
        id: Uuid,
        username: Option<&str>,
        password: Option<&str>,
        role: Option<nexus_types::Role>,
    ) -> Result<UserRow, UserRepoError> {
        // Get current user first
        let mut user = self.get_by_id(id).await?;

        // Update fields if provided
        if let Some(u) = username {
            user.username = u.to_string();
        }

        if let Some(p) = password {
            user.password_hash = Self::hash_password(p)?;
        }

        if let Some(r) = role {
            user.role = r.as_str().to_string();
        }

        // Update in database
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET username = $2, password_hash = $3, role = $4, updated_at = now()
            WHERE id = $1
            RETURNING id, username, password_hash, role, last_login_at, avatar_path, timezone, theme, preferences, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), UserRepoError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(UserRepoError::UserNotFound);
        }

        Ok(())
    }

    pub async fn verify_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserRow, UserRepoError> {
        let user = self.get_by_username(username).await?;
        let is_valid = Self::verify_password_hash(password, &user.password_hash)?;
        if !is_valid {
            return Err(UserRepoError::InvalidCredentials);
        }
        Ok(user)
    }

    pub async fn create_token(
        &self,
        user_id: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<String, UserRepoError> {
        let token = Self::generate_token();
        let token_hash = Self::hash_token(&token);

        sqlx::query(
            r#"
            INSERT INTO api_tokens (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    pub async fn validate_token(&self, token: &str) -> Result<AuthenticatedUser, UserRepoError> {
        let token_hash = Self::hash_token(token);

        let token_row = sqlx::query_as::<_, ApiTokenRow>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, last_used_at
            FROM api_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let token_row = token_row.ok_or(UserRepoError::InvalidToken)?;

        // Check expiration
        if let Some(expires_at) = token_row.expires_at {
            if expires_at < Utc::now() {
                return Err(UserRepoError::TokenExpired);
            }
        }

        // Update last_used_at
        sqlx::query(
            r#"
            UPDATE api_tokens
            SET last_used_at = now()
            WHERE id = $1
            "#,
        )
        .bind(token_row.id)
        .execute(&self.pool)
        .await?;

        // Get user
        let user = self.get_by_id(token_row.user_id).await?;

        Ok(AuthenticatedUser {
            id: user.id,
            username: user.username.clone(),
            role: user.get_role(),
        })
    }

    pub async fn revoke_token(&self, token: &str) -> Result<(), UserRepoError> {
        let token_hash = Self::hash_token(token);

        sqlx::query("DELETE FROM api_tokens WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Preferences Management
    pub async fn get_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<nexus_types::UserPreferences, UserRepoError> {
        debug!(user_id = ?user_id, "fetching preferences from database");

        let row: Option<(
            Option<sqlx::types::JsonValue>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as("SELECT preferences, timezone, theme FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(?e, user_id = ?user_id, "database error fetching preferences");
                e
            })?;

        let (prefs_json, timezone, theme) = row.ok_or_else(|| {
            debug!(user_id = ?user_id, "user not found when fetching preferences");
            UserRepoError::UserNotFound
        })?;

        let mut prefs: nexus_types::UserPreferences = if let Some(json) = prefs_json {
            serde_json::from_value(json).unwrap_or_default()
        } else {
            nexus_types::UserPreferences::default()
        };

        // Override with column values if present
        if let Some(tz) = timezone {
            prefs.timezone = Some(tz);
        }
        if let Some(th) = theme {
            prefs.theme = Some(th);
        }

        debug!(user_id = ?user_id, "preferences fetched successfully");
        Ok(prefs)
    }

    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        req: &nexus_types::UpdatePreferencesRequest,
    ) -> Result<nexus_types::UserPreferences, UserRepoError> {
        // Get current preferences
        let mut prefs = self.get_preferences(user_id).await.unwrap_or_default();

        // Update fields
        if let Some(tz) = &req.timezone {
            prefs.timezone = Some(tz.clone());
        }
        if let Some(theme) = &req.theme {
            prefs.theme = Some(theme.clone());
        }
        if let Some(date_format) = &req.date_format {
            prefs.date_format = Some(date_format.clone());
        }
        if let Some(notifications) = &req.notifications {
            prefs.notifications = notifications.clone();
        }
        if let Some(vm_defaults) = &req.vm_defaults {
            prefs.vm_defaults = vm_defaults.clone();
        }
        if let Some(auto_refresh) = req.auto_refresh {
            prefs.auto_refresh = Some(auto_refresh);
        }
        if let Some(metrics_retention) = req.metrics_retention {
            prefs.metrics_retention = Some(metrics_retention);
        }

        // Save to database
        let prefs_json =
            serde_json::to_value(&prefs).map_err(|e| UserRepoError::HashingError(e.to_string()))?;

        sqlx::query(
            "UPDATE users SET preferences = $1, timezone = $2, theme = $3, updated_at = now() WHERE id = $4"
        )
        .bind(&prefs_json)
        .bind(&prefs.timezone)
        .bind(&prefs.theme)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(prefs)
    }

    // Profile Management
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        username: Option<&str>,
    ) -> Result<UserRow, UserRepoError> {
        if let Some(new_username) = username {
            sqlx::query("UPDATE users SET username = $1, updated_at = now() WHERE id = $2")
                .bind(new_username)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        self.get_by_id(user_id).await
    }

    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), UserRepoError> {
        // Get user
        let user = self.get_by_id(user_id).await?;

        // Verify current password
        if !Self::verify_password_hash(current_password, &user.password_hash)? {
            return Err(UserRepoError::InvalidCredentials);
        }

        // Hash new password
        let new_hash = Self::hash_password(new_password)?;

        // Update password
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
            .bind(new_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Avatar Management
    pub async fn get_avatar_path(&self, user_id: Uuid) -> Result<Option<String>, UserRepoError> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT avatar_path FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.and_then(|(path,)| path))
    }

    pub async fn set_avatar_path(&self, user_id: Uuid, path: &str) -> Result<(), UserRepoError> {
        sqlx::query("UPDATE users SET avatar_path = $1, updated_at = now() WHERE id = $2")
            .bind(path)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_avatar(&self, user_id: Uuid) -> Result<(), UserRepoError> {
        sqlx::query("UPDATE users SET avatar_path = NULL, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum UserRepoError {
    #[error("user not found")]
    UserNotFound,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid token")]
    InvalidToken,
    #[error("token expired")]
    TokenExpired,
    #[error("invalid role: {0}")]
    InvalidRole(String),
    #[error("password hashing error: {0}")]
    HashingError(String),
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
}
