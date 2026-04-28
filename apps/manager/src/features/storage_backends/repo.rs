use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct StorageBackendRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StorageBackendRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub config_json: JsonValue,
    pub capabilities_json: JsonValue,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl StorageBackendRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_active(&self) -> sqlx::Result<Vec<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE deleted_at IS NULL ORDER BY name"#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_default(&self) -> sqlx::Result<Option<StorageBackendRow>> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"SELECT * FROM storage_backend WHERE is_default = true AND deleted_at IS NULL LIMIT 1"#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Upsert by name. Used by the registry on startup to reconcile TOML with DB.
    pub async fn upsert(
        &self,
        name: &str,
        kind: &str,
        config_json: &JsonValue,
        capabilities_json: &JsonValue,
        is_default: bool,
    ) -> sqlx::Result<StorageBackendRow> {
        sqlx::query_as::<_, StorageBackendRow>(
            r#"
            INSERT INTO storage_backend (name, kind, config_json, capabilities_json, is_default, deleted_at)
            VALUES ($1, $2, $3, $4, $5, NULL)
            ON CONFLICT (name) DO UPDATE
              SET kind = EXCLUDED.kind,
                  config_json = EXCLUDED.config_json,
                  capabilities_json = EXCLUDED.capabilities_json,
                  is_default = EXCLUDED.is_default,
                  deleted_at = NULL
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(kind)
        .bind(config_json)
        .bind(capabilities_json)
        .bind(is_default)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn soft_delete_by_name(&self, name: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE storage_backend SET deleted_at = now() WHERE name = $1 AND deleted_at IS NULL"#,
        )
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
