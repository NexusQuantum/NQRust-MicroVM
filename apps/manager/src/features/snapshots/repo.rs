use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct SnapshotRow {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub snapshot_path: String,
    pub mem_path: String,
    pub size_bytes: i64,
    pub state: String,
    pub snapshot_type: String,
    pub parent_id: Option<Uuid>,
    pub track_dirty_pages: bool,
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct SnapshotRepository {
    pool: PgPool,
}

impl SnapshotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, new_row: &NewSnapshotRow) -> sqlx::Result<SnapshotRow> {
        sqlx::query_as::<_, SnapshotRow>(
            r#"
            INSERT INTO snapshot (id, vm_id, snapshot_path, mem_path, size_bytes, state, snapshot_type, parent_id, track_dirty_pages, name)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, vm_id, snapshot_path, mem_path, size_bytes, state, snapshot_type, parent_id, track_dirty_pages, name, created_at, updated_at
            "#,
        )
        .bind(new_row.id)
        .bind(new_row.vm_id)
        .bind(&new_row.snapshot_path)
        .bind(&new_row.mem_path)
        .bind(new_row.size_bytes)
        .bind(&new_row.state)
        .bind(&new_row.snapshot_type)
        .bind(new_row.parent_id)
        .bind(new_row.track_dirty_pages)
        .bind(&new_row.name)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_for_vm(&self, vm_id: Uuid) -> sqlx::Result<Vec<SnapshotRow>> {
        sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT id, vm_id, snapshot_path, mem_path, size_bytes, state, snapshot_type, parent_id, track_dirty_pages, name, created_at, updated_at
            FROM snapshot
            WHERE vm_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(vm_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<SnapshotRow> {
        sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT id, vm_id, snapshot_path, mem_path, size_bytes, state, snapshot_type, parent_id, track_dirty_pages, name, created_at, updated_at
            FROM snapshot
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM snapshot
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn update_size_and_mem(
    pool: &PgPool,
    id: Uuid,
    size_bytes: i64,
    mem_path: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE snapshot
        SET size_bytes = $2, mem_path = $3, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(size_bytes)
    .bind(mem_path)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct NewSnapshotRow {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub snapshot_path: String,
    pub mem_path: String,
    pub size_bytes: i64,
    pub state: String,
    pub snapshot_type: String,
    pub parent_id: Option<Uuid>,
    pub track_dirty_pages: bool,
    pub name: Option<String>,
}
