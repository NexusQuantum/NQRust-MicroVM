use chrono::DateTime;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct HostRepository {
    pool: PgPool,
}

impl HostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn register(
        &self,
        name: &str,
        addr: &str,
        capabilities: Value,
    ) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            INSERT INTO host (id, name, addr, capabilities_json, last_seen_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (addr) DO UPDATE
            SET name = EXCLUDED.name,
                capabilities_json = EXCLUDED.capabilities_json,
                last_seen_at = now()
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(addr)
        .bind(capabilities)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn heartbeat(&self, id: Uuid, capabilities: Option<Value>) -> sqlx::Result<HostRow> {
        match capabilities {
            Some(value) => {
                sqlx::query_as::<_, HostRow>(
                    r#"UPDATE host SET capabilities_json=$2, last_seen_at=now() WHERE id=$1 RETURNING *"#,
                )
                .bind(id)
                .bind(value)
                .fetch_one(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, HostRow>(
                    r#"UPDATE host SET last_seen_at=now() WHERE id=$1 RETURNING *"#,
                )
                .bind(id)
                .fetch_one(&self.pool)
                .await
            }
        }
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(r#"SELECT * FROM host WHERE id=$1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    /// First placeable host: healthy heartbeat, not a hot-spare, not
    /// draining or decommissioned. B-III Tasks 5 + 6: hot-spares and
    /// non-active hosts must not show up as placement targets.
    pub async fn first_healthy(&self) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
              AND is_hot_spare = false
              AND lifecycle_state = 'active'
            ORDER BY last_seen_at DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// All placeable hosts (same filters as `first_healthy`).
    pub async fn list_healthy(&self) -> sqlx::Result<Vec<HostRow>> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
              AND is_hot_spare = false
              AND lifecycle_state = 'active'
            ORDER BY last_seen_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Hot-spare hosts that have a healthy heartbeat. Used by Task 7
    /// (failure recovery) and the host-add candidate listing. Decommissioned
    /// hosts are excluded; draining hosts are excluded.
    pub async fn list_hot_spares(&self) -> sqlx::Result<Vec<HostRow>> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
              AND is_hot_spare = true
              AND lifecycle_state = 'active'
            ORDER BY last_seen_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_all(&self) -> sqlx::Result<Vec<HostRow>> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            ORDER BY last_seen_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_metrics(
        &self,
        id: Uuid,
        total_cpus: i32,
        total_memory_mb: i64,
        total_disk_gb: i64,
        used_disk_gb: i64,
    ) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            UPDATE host
            SET total_cpus = $2,
                total_memory_mb = $3,
                total_disk_gb = $4,
                used_disk_gb = $5,
                last_metrics_at = now(),
                last_seen_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(total_cpus)
        .bind(total_memory_mb)
        .bind(total_disk_gb)
        .bind(used_disk_gb)
        .fetch_one(&self.pool)
        .await
    }

    /// B-III follow-up: set the host's SPDK backend id (the lvol bdev id
    /// used when placing a raft_spdk replica on this host). Pass `None`
    /// to clear the configuration and remove the host from raft_spdk
    /// placement.
    pub async fn set_spdk_backend_id(
        &self,
        id: Uuid,
        spdk_backend_id: Option<Uuid>,
    ) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            UPDATE host
               SET spdk_backend_id = $2
             WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(spdk_backend_id)
        .fetch_one(&self.pool)
        .await
    }

    /// B-III Task 5: toggle hot-spare flag.
    pub async fn set_hot_spare(&self, id: Uuid, value: bool) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            UPDATE host
               SET is_hot_spare = $2
             WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(value)
        .fetch_one(&self.pool)
        .await
    }

    /// B-III Task 6: transition host lifecycle. Refuses invalid moves
    /// (`decommissioned` is terminal — once set, can only be re-activated
    /// by deleting and re-registering the host).
    pub async fn set_lifecycle(&self, id: Uuid, target: &str) -> sqlx::Result<HostRow> {
        if !matches!(target, "active" | "draining" | "decommissioned") {
            return Err(sqlx::Error::Protocol(format!(
                "invalid host lifecycle target: {target}"
            )));
        }
        sqlx::query_as::<_, HostRow>(
            r#"
            UPDATE host
               SET lifecycle_state = $2,
                   lifecycle_changed_at = now()
             WHERE id = $1
               AND (
                   lifecycle_state <> 'decommissioned'
                   OR $2 = 'decommissioned'
               )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(target)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_vm_count(&self, host_id: Uuid) -> sqlx::Result<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM vm
            WHERE host_id = $1
            "#,
        )
        .bind(host_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    pub async fn delete(&self, id: Uuid) -> sqlx::Result<()> {
        // Only allow deletion of dead hosts (last_seen_at > 30 seconds ago)
        sqlx::query(
            r#"
            DELETE FROM host 
            WHERE id = $1 
            AND last_seen_at <= now() - INTERVAL '30 seconds'
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn is_alive(&self, id: Uuid) -> sqlx::Result<bool> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT last_seen_at > now() - INTERVAL '30 seconds' as is_alive
            FROM host
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    pub async fn supported_backend_kinds(&self, host_id: Uuid) -> sqlx::Result<Vec<String>> {
        let v: serde_json::Value =
            sqlx::query_scalar(r#"SELECT supported_backend_kinds FROM host WHERE id = $1"#)
                .bind(host_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn update_supported_backend_kinds(
        &self,
        host_id: Uuid,
        kinds: Vec<String>,
    ) -> sqlx::Result<()> {
        let v =
            serde_json::Value::Array(kinds.into_iter().map(serde_json::Value::String).collect());
        sqlx::query(r#"UPDATE host SET supported_backend_kinds = $1 WHERE id = $2"#)
            .bind(v)
            .bind(host_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct HostRow {
    pub id: Uuid,
    pub name: String,
    pub addr: String,
    pub capabilities_json: Value,
    pub last_seen_at: DateTime<chrono::Utc>,
    pub total_cpus: Option<i32>,
    pub total_memory_mb: Option<i64>,
    pub total_disk_gb: Option<i64>,
    pub used_disk_gb: Option<i64>,
    pub last_metrics_at: Option<DateTime<chrono::Utc>>,
    /// B-III Task 5: when true, the host is held in reserve and is
    /// skipped by `first_healthy`/`list_healthy` placement. Promoted to
    /// active during failure recovery (Task 7).
    pub is_hot_spare: bool,
    /// B-III Task 6: `active`, `draining` (mid-decommission, refuses new
    /// placement), or `decommissioned` (terminal).
    pub lifecycle_state: String,
    pub lifecycle_changed_at: Option<DateTime<chrono::Utc>>,
    /// B-III follow-up: SPDK lvol bdev id this host uses for raft_spdk
    /// replicas. `None` means the host cannot host raft_spdk replicas
    /// and the planner skips it as a raft_spdk placement target.
    pub spdk_backend_id: Option<Uuid>,
}
