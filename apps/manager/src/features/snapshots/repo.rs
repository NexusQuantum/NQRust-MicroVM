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
            INSERT INTO snapshot (id, vm_id, snapshot_path, mem_path, size_bytes, state)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, vm_id, snapshot_path, mem_path, size_bytes, state, created_at, updated_at
            "#,
        )
        .bind(new_row.id)
        .bind(new_row.vm_id)
        .bind(&new_row.snapshot_path)
        .bind(&new_row.mem_path)
        .bind(new_row.size_bytes)
        .bind(&new_row.state)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_for_vm(&self, vm_id: Uuid) -> sqlx::Result<Vec<SnapshotRow>> {
        sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT id, vm_id, snapshot_path, mem_path, size_bytes, state, created_at, updated_at
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
            SELECT id, vm_id, snapshot_path, mem_path, size_bytes, state, created_at, updated_at
            FROM snapshot
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}

#[derive(Clone)]
pub struct NewSnapshotRow {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub snapshot_path: String,
    pub mem_path: String,
    pub size_bytes: i64,
    pub state: String,
}
