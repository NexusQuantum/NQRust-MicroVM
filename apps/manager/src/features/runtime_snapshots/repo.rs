use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct RuntimeSnapshotRow {
    pub id: Uuid,
    pub runtime_image_id: Uuid,
    pub snapshot_path: String,
    pub state: String,
    pub fc_version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub success_count: i32,
    pub failure_count: i32,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Clone)]
pub struct RuntimeSnapshotRepository {
    pool: PgPool,
}

impl RuntimeSnapshotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, new_row: &NewRuntimeSnapshotRow) -> sqlx::Result<RuntimeSnapshotRow> {
        sqlx::query_as::<_, RuntimeSnapshotRow>(
            r#"
            INSERT INTO runtime_snapshots (id, runtime_image_id, snapshot_path, state, fc_version, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, runtime_image_id, snapshot_path, state, fc_version, created_at, success_count, failure_count, last_used_at, metadata
            "#,
        )
        .bind(new_row.id)
        .bind(new_row.runtime_image_id)
        .bind(&new_row.snapshot_path)
        .bind(&new_row.state)
        .bind(&new_row.fc_version)
        .bind(&new_row.metadata)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list(&self) -> sqlx::Result<Vec<RuntimeSnapshotRow>> {
        sqlx::query_as::<_, RuntimeSnapshotRow>(
            r#"
            SELECT id, runtime_image_id, snapshot_path, state, fc_version, created_at, success_count, failure_count, last_used_at, metadata
            FROM runtime_snapshots
            WHERE state != 'deleted'
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<RuntimeSnapshotRow> {
        sqlx::query_as::<_, RuntimeSnapshotRow>(
            r#"
            SELECT id, runtime_image_id, snapshot_path, state, fc_version, created_at, success_count, failure_count, last_used_at, metadata
            FROM runtime_snapshots
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_by_runtime_image(&self, runtime_image_id: Uuid) -> sqlx::Result<Option<RuntimeSnapshotRow>> {
        sqlx::query_as::<_, RuntimeSnapshotRow>(
            r#"
            SELECT id, runtime_image_id, snapshot_path, state, fc_version, created_at, success_count, failure_count, last_used_at, metadata
            FROM runtime_snapshots
            WHERE runtime_image_id = $1 AND state = 'ready'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(runtime_image_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_state(&self, id: Uuid, state: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE runtime_snapshots
            SET state = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(state)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_success(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE runtime_snapshots
            SET success_count = success_count + 1, last_used_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_failure(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE runtime_snapshots
            SET failure_count = failure_count + 1
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_unhealthy(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE runtime_snapshots
            SET state = 'unhealthy'
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE runtime_snapshots
            SET state = 'deleted'
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn hard_delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM runtime_snapshots
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct NewRuntimeSnapshotRow {
    pub id: Uuid,
    pub runtime_image_id: Uuid,
    pub snapshot_path: String,
    pub state: String,
    pub fc_version: String,
    pub metadata: serde_json::Value,
}
