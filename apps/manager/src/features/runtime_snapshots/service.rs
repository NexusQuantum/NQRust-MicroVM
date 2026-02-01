use super::repo::{NewRuntimeSnapshotRow, RuntimeSnapshotRepository, RuntimeSnapshotRow};
use nexus_types::RuntimeSnapshot;
use uuid::Uuid;

pub struct RuntimeSnapshotService {
    pub repo: RuntimeSnapshotRepository,
}

impl RuntimeSnapshotService {
    pub fn new(repo: RuntimeSnapshotRepository) -> Self {
        Self { repo }
    }

    pub async fn create(
        &self,
        runtime_image_id: Uuid,
        snapshot_path: String,
        fc_version: String,
    ) -> Result<RuntimeSnapshotRow, sqlx::Error> {
        let new_row = NewRuntimeSnapshotRow {
            id: Uuid::new_v4(),
            runtime_image_id,
            snapshot_path,
            state: "creating".to_string(),
            fc_version,
            metadata: serde_json::json!({}),
        };

        self.repo.insert(&new_row).await
    }

    pub async fn list(&self) -> Result<Vec<RuntimeSnapshot>, sqlx::Error> {
        let rows = self.repo.list().await?;
        Ok(rows.into_iter().map(row_to_snapshot).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<RuntimeSnapshot, sqlx::Error> {
        let row = self.repo.get(id).await?;
        Ok(row_to_snapshot(row))
    }

    pub async fn find_by_runtime_image(
        &self,
        runtime_image_id: Uuid,
    ) -> Result<Option<RuntimeSnapshot>, sqlx::Error> {
        let row = self.repo.find_by_runtime_image(runtime_image_id).await?;
        Ok(row.map(row_to_snapshot))
    }

    pub async fn mark_ready(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.update_state(id, "ready").await
    }

    pub async fn mark_unhealthy(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.mark_unhealthy(id).await
    }

    pub async fn record_success(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.increment_success(id).await
    }

    pub async fn record_failure(&self, id: Uuid) -> Result<(), sqlx::Error> {
        // Increment failure count
        self.repo.increment_failure(id).await?;

        // Check if we should mark as unhealthy (3+ consecutive failures)
        let snapshot = self.repo.get(id).await?;
        if snapshot.failure_count >= 3 && snapshot.state == "ready" {
            self.repo.mark_unhealthy(id).await?;
        }

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.delete(id).await
    }

    pub async fn hard_delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.hard_delete(id).await
    }
}

fn row_to_snapshot(row: RuntimeSnapshotRow) -> RuntimeSnapshot {
    RuntimeSnapshot {
        id: row.id,
        runtime_image_id: row.runtime_image_id,
        snapshot_path: row.snapshot_path,
        state: row.state,
        fc_version: row.fc_version,
        created_at: row.created_at,
        success_count: row.success_count,
        failure_count: row.failure_count,
        last_used_at: row.last_used_at,
        metadata: row.metadata,
    }
}
