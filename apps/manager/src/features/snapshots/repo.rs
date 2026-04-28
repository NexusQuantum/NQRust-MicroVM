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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_new_row() -> NewSnapshotRow {
        NewSnapshotRow {
            id: Uuid::new_v4(),
            vm_id: Uuid::new_v4(),
            snapshot_path: "/srv/fc/snap.bin".into(),
            mem_path: "/srv/fc/mem.bin".into(),
            size_bytes: 4096,
            state: "available".into(),
            snapshot_type: "Full".into(),
            parent_id: None,
            track_dirty_pages: false,
            name: Some("nightly".into()),
        }
    }

    #[test]
    fn new_snapshot_row_is_clonable_and_field_accurate() {
        let row = sample_new_row();
        let cloned = row.clone();

        assert_eq!(cloned.id, row.id);
        assert_eq!(cloned.vm_id, row.vm_id);
        assert_eq!(cloned.snapshot_path, "/srv/fc/snap.bin");
        assert_eq!(cloned.mem_path, "/srv/fc/mem.bin");
        assert_eq!(cloned.size_bytes, 4096);
        assert_eq!(cloned.state, "available");
        assert_eq!(cloned.snapshot_type, "Full");
        assert_eq!(cloned.parent_id, None);
        assert!(!cloned.track_dirty_pages);
        assert_eq!(cloned.name.as_deref(), Some("nightly"));
    }

    #[test]
    fn new_snapshot_row_supports_diff_with_parent() {
        // Diff snapshots reference a parent and may have an empty mem_path
        // (the manager zeroes it out before persisting). Confirm the struct
        // can carry that combination.
        let parent = Uuid::new_v4();
        let row = NewSnapshotRow {
            id: Uuid::new_v4(),
            vm_id: Uuid::new_v4(),
            snapshot_path: "/srv/fc/diff.bin".into(),
            mem_path: String::new(),
            size_bytes: 0,
            state: "available".into(),
            snapshot_type: "Diff".into(),
            parent_id: Some(parent),
            track_dirty_pages: true,
            name: None,
        };

        assert_eq!(row.snapshot_type, "Diff");
        assert_eq!(row.parent_id, Some(parent));
        assert!(row.track_dirty_pages);
        assert!(row.mem_path.is_empty());
        assert!(row.name.is_none());
    }

    #[test]
    fn snapshot_row_clone_round_trip_preserves_fields() {
        let now = chrono::Utc::now();
        let row = SnapshotRow {
            id: Uuid::new_v4(),
            vm_id: Uuid::new_v4(),
            snapshot_path: "/srv/fc/snap.bin".into(),
            mem_path: "/srv/fc/mem.bin".into(),
            size_bytes: 9001,
            state: "available".into(),
            snapshot_type: "Full".into(),
            parent_id: None,
            track_dirty_pages: false,
            name: Some("snap-a".into()),
            created_at: now,
            updated_at: now,
        };

        let copy = row.clone();
        assert_eq!(copy.id, row.id);
        assert_eq!(copy.size_bytes, 9001);
        assert_eq!(copy.snapshot_type, "Full");
        assert_eq!(copy.created_at, now);
        assert_eq!(copy.updated_at, now);
    }
}
