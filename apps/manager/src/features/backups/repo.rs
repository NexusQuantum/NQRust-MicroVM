use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct BackupRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BackupRow {
    pub id: Uuid,
    pub source_volume_id: Option<Uuid>,
    #[allow(dead_code)]
    pub source_snapshot_id: Option<Uuid>,
    pub target_id: Uuid,
    pub manifest_object_key: Option<String>,
    pub size_bytes: i64,
    pub unique_bytes: i64,
    pub chunk_count: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

impl BackupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_running(
        &self,
        source_volume_id: Uuid,
        source_snapshot_id: Uuid,
        target_id: Uuid,
    ) -> sqlx::Result<BackupRow> {
        sqlx::query_as::<_, BackupRow>(
            r#"INSERT INTO backup (source_volume_id, source_snapshot_id, target_id, status)
               VALUES ($1, $2, $3, 'running')
               RETURNING *"#,
        )
        .bind(source_volume_id)
        .bind(source_snapshot_id)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn mark_completed(
        &self,
        id: Uuid,
        manifest_object_key: &str,
        size_bytes: i64,
        unique_bytes: i64,
        chunk_count: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE backup
               SET status = 'completed',
                   manifest_object_key = $1,
                   size_bytes = $2,
                   unique_bytes = $3,
                   chunk_count = $4,
                   completed_at = now(),
                   updated_at = now()
               WHERE id = $5"#,
        )
        .bind(manifest_object_key)
        .bind(size_bytes)
        .bind(unique_bytes)
        .bind(chunk_count)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid, error: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE backup
               SET status = 'failed', error_message = $1, updated_at = now()
               WHERE id = $2"#,
        )
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(r#"SELECT * FROM backup WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn list_for_volume(&self, volume_id: Uuid) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup WHERE source_volume_id = $1 ORDER BY created_at DESC"#,
        )
        .bind(volume_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_completed_oldest_first(
        &self,
        volume_id: Uuid,
    ) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup WHERE source_volume_id = $1 AND status = 'completed' ORDER BY created_at ASC"#,
        )
        .bind(volume_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_row(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM backup WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn list_stale_running(
        &self,
        older_than_minutes: i64,
    ) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup
               WHERE status = 'running'
                 AND updated_at < now() - make_interval(mins => $1)"#,
        )
        .bind(older_than_minutes)
        .fetch_all(&self.pool)
        .await
    }
}
