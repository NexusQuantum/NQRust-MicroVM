use chrono::DateTime;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct VolumeRepository {
    pool: PgPool,
}

impl VolumeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        path: &str,
        size_bytes: i64,
        volume_type: &str,
        host_id: Option<Uuid>,
        backend_id: Uuid,
    ) -> sqlx::Result<VolumeRow> {
        self.create_with_id(None, name, description, path, size_bytes, volume_type, host_id, backend_id)
            .await
    }

    /// Insert a volume row with an explicit `id`. Used when the storage
    /// backend's `provision()` already minted a `volume_id` (e.g. raft_spdk
    /// embeds the volume id in its locator and the same id is used as the
    /// raft group identifier — the DB row and the backend resource must
    /// agree on which uuid is "the volume").
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_id(
        &self,
        id: Option<Uuid>,
        name: &str,
        description: Option<&str>,
        path: &str,
        size_bytes: i64,
        volume_type: &str,
        host_id: Option<Uuid>,
        backend_id: Uuid,
    ) -> sqlx::Result<VolumeRow> {
        let id = id.unwrap_or_else(Uuid::new_v4);
        sqlx::query_as::<_, VolumeRow>(
            r#"
            INSERT INTO volume (id, name, description, path, size_bytes, type, status, host_id, backend_id, created_by_user_id)
            VALUES ($1, $2, $3, $4, $5, $6, 'available', $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(path)
        .bind(size_bytes)
        .bind(volume_type)
        .bind(host_id)
        .bind(backend_id)
        .bind(None as Option<Uuid>) // created_by_user_id - TODO: Set from authenticated user context
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<VolumeRow> {
        sqlx::query_as::<_, VolumeRow>(r#"SELECT * FROM volume WHERE id = $1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn list(&self) -> sqlx::Result<Vec<VolumeRow>> {
        sqlx::query_as::<_, VolumeRow>(
            r#"
            SELECT * FROM volume
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_by_host(&self, host_id: Uuid) -> sqlx::Result<Vec<VolumeRow>> {
        sqlx::query_as::<_, VolumeRow>(
            r#"
            SELECT * FROM volume
            WHERE host_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn list_by_status(&self, status: &str) -> sqlx::Result<Vec<VolumeRow>> {
        sqlx::query_as::<_, VolumeRow>(
            r#"
            SELECT * FROM volume
            WHERE status = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> sqlx::Result<VolumeRow> {
        sqlx::query_as::<_, VolumeRow>(
            r#"
            UPDATE volume
            SET status = $2, updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(&self, id: Uuid) -> sqlx::Result<VolumeRow> {
        sqlx::query_as::<_, VolumeRow>(r#"DELETE FROM volume WHERE id = $1 RETURNING *"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn attach(
        &self,
        volume_id: Uuid,
        vm_id: Uuid,
        drive_id: &str,
    ) -> sqlx::Result<AttachmentRow> {
        // First update volume status to 'attached'
        sqlx::query(r#"UPDATE volume SET status = 'attached' WHERE id = $1"#)
            .bind(volume_id)
            .execute(&self.pool)
            .await?;

        // Then create attachment record
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            INSERT INTO volume_attachment (volume_id, vm_id, drive_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(volume_id)
        .bind(vm_id)
        .bind(drive_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn detach(&self, volume_id: Uuid, vm_id: Uuid) -> sqlx::Result<()> {
        // Soft-detach: set detached_at to preserve audit trail
        sqlx::query(
            r#"UPDATE volume_attachment SET detached_at = now()
               WHERE volume_id = $1 AND vm_id = $2 AND detached_at IS NULL"#,
        )
        .bind(volume_id)
        .bind(vm_id)
        .execute(&self.pool)
        .await?;

        // Update volume status to 'available'
        sqlx::query(r#"UPDATE volume SET status = 'available' WHERE id = $1"#)
            .bind(volume_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_attachments(&self, volume_id: Uuid) -> sqlx::Result<Vec<AttachmentRow>> {
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            SELECT * FROM volume_attachment
            WHERE volume_id = $1
            ORDER BY attached_at DESC
            "#,
        )
        .bind(volume_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_attached_vm(&self, volume_id: Uuid) -> sqlx::Result<Option<Uuid>> {
        let result: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT vm_id FROM volume_attachment
            WHERE volume_id = $1 AND detached_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(volume_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(vm_id,)| vm_id))
    }

    #[allow(dead_code)]
    pub async fn insert_active_attachment(
        &self,
        volume_id: Uuid,
        vm_id: Uuid,
        drive_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id) VALUES ($1, $2, $3)"#,
        )
        .bind(volume_id)
        .bind(vm_id)
        .bind(drive_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_detached(&self, vm_id: Uuid, drive_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE volume_attachment SET detached_at = now()
               WHERE vm_id = $1 AND drive_id = $2 AND detached_at IS NULL"#,
        )
        .bind(vm_id)
        .bind(drive_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct VolumeRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub size_bytes: i64,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub type_: String,
    pub status: String,
    pub host_id: Option<Uuid>,
    pub backend_id: Uuid,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<chrono::Utc>,
    pub updated_at: DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AttachmentRow {
    pub id: Uuid,
    pub volume_id: Uuid,
    pub vm_id: Uuid,
    pub drive_id: String,
    pub attached_at: DateTime<chrono::Utc>,
    pub detached_at: Option<DateTime<chrono::Utc>>,
}
