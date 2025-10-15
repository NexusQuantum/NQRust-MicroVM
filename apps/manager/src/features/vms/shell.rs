use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Clone, FromRow)]
pub struct VmShellCredential {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub username: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
pub struct VmShellSession {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct ShellRepository {
    pool: PgPool,
}

impl ShellRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_credentials(
        &self,
        vm_id: Uuid,
        username: &str,
        password: &str,
    ) -> Result<VmShellCredential> {
        sqlx::query_as::<_, VmShellCredential>(
            r#"
            INSERT INTO vm_shell_credential (id, vm_id, username, password)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (vm_id)
            DO UPDATE SET username = EXCLUDED.username,
                           password = EXCLUDED.password,
                           updated_at = now()
            RETURNING id, vm_id, username, password, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(vm_id)
        .bind(username)
        .bind(password)
        .fetch_one(&self.pool)
        .await
        .context("failed to upsert shell credentials")
    }

    pub async fn get_credentials(&self, vm_id: Uuid) -> Result<Option<VmShellCredential>> {
        sqlx::query_as::<_, VmShellCredential>(
            r#"
            SELECT id, vm_id, username, password, created_at, updated_at
            FROM vm_shell_credential
            WHERE vm_id = $1
            "#,
        )
        .bind(vm_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch shell credentials")
    }

    pub async fn create_session(
        &self,
        vm_id: Uuid,
        token: &str,
        ttl: Duration,
    ) -> Result<VmShellSession> {
        let expires_at = Utc::now() + ttl;
        sqlx::query_as::<_, VmShellSession>(
            r#"
            INSERT INTO vm_shell_session (id, vm_id, token, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, vm_id, token, expires_at, created_at, last_seen_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(vm_id)
        .bind(token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .context("failed to create shell session")
    }

    pub async fn touch_session(&self, token: &str) -> Result<Option<VmShellSession>> {
        sqlx::query_as::<_, VmShellSession>(
            r#"
            UPDATE vm_shell_session
            SET last_seen_at = now()
            WHERE token = $1 AND expires_at > now()
            RETURNING id, vm_id, token, expires_at, created_at, last_seen_at
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .context("failed to update shell session")
    }

    pub async fn delete_session(&self, token: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM vm_shell_session
            WHERE token = $1
            "#,
        )
        .bind(token)
        .execute(&self.pool)
        .await
        .context("failed to delete shell session")?;
        Ok(())
    }

    pub async fn purge_expired(&self) -> Result<()> {
        sqlx::query("DELETE FROM vm_shell_session WHERE expires_at <= now()")
            .execute(&self.pool)
            .await
            .context("failed to purge expired shell sessions")?;
        Ok(())
    }
}
