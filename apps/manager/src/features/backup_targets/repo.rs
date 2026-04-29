use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct BackupTargetRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct BackupTargetRow {
    pub id: Uuid,
    pub name: String,
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub encrypted_secret_access_key: Vec<u8>,
    pub encrypted_target_key: Vec<u8>,
    pub gc_hour: i16,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub struct CreateParams<'a> {
    pub name: &'a str,
    pub endpoint: &'a str,
    pub region: Option<&'a str>,
    pub bucket: &'a str,
    pub prefix: &'a str,
    pub access_key_id: &'a str,
    pub encrypted_secret_access_key: &'a [u8],
    pub encrypted_target_key: &'a [u8],
    pub gc_hour: i16,
}

impl BackupTargetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_active(&self) -> sqlx::Result<Vec<BackupTargetRow>> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"SELECT * FROM backup_target WHERE deleted_at IS NULL ORDER BY name"#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<BackupTargetRow>> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"SELECT * FROM backup_target WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, p: CreateParams<'_>) -> sqlx::Result<BackupTargetRow> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"
            INSERT INTO backup_target
              (name, endpoint, region, bucket, prefix, access_key_id,
               encrypted_secret_access_key, encrypted_target_key, gc_hour)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(p.name)
        .bind(p.endpoint)
        .bind(p.region)
        .bind(p.bucket)
        .bind(p.prefix)
        .bind(p.access_key_id)
        .bind(p.encrypted_secret_access_key)
        .bind(p.encrypted_target_key)
        .bind(p.gc_hour)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn soft_delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE backup_target SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn count_backups_for_target(&self, id: Uuid) -> sqlx::Result<i64> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM backup WHERE target_id = $1 AND status IN ('running','completed')"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
