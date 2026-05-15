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

    pub async fn first_healthy(&self) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
            ORDER BY last_seen_at DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_healthy(&self) -> sqlx::Result<Vec<HostRow>> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
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

    /// Replace the set of VMM kinds advertised by an agent on this host.
    pub async fn update_vmm_kinds_installed(
        &self,
        host_id: Uuid,
        kinds: Vec<String>,
    ) -> sqlx::Result<()> {
        sqlx::query(r#"UPDATE host SET vmm_kinds_installed = $1 WHERE id = $2"#)
            .bind(&kinds)
            .bind(host_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return the VMM kinds currently installed on a host. Empty if the host
    /// has never registered or the column is empty (default '{firecracker}').
    pub async fn vmm_kinds_installed(&self, host_id: Uuid) -> sqlx::Result<Vec<String>> {
        let kinds: Vec<String> =
            sqlx::query_scalar(r#"SELECT vmm_kinds_installed FROM host WHERE id = $1"#)
                .bind(host_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(kinds)
    }

    /// Reserve vcpu+mem capacity on a host atomically. Returns Ok(true) when
    /// the reservation fit; Ok(false) when it would over-commit (callers
    /// should pick a different host or refuse).
    pub async fn try_reserve(&self, host_id: Uuid, vcpu: i32, mem_mib: i64) -> sqlx::Result<bool> {
        let updated: Option<(i64,)> = sqlx::query_as(
            r#"UPDATE host
                SET reserved_vcpu = reserved_vcpu + $2,
                    reserved_mem_mib = reserved_mem_mib + $3
                WHERE id = $1
                  AND (total_vcpu IS NULL OR reserved_vcpu + $2 <= total_vcpu)
                  AND (total_mem_mib IS NULL OR reserved_mem_mib + $3 <= total_mem_mib)
                RETURNING reserved_mem_mib"#,
        )
        .bind(host_id)
        .bind(vcpu)
        .bind(mem_mib)
        .fetch_optional(&self.pool)
        .await?;
        Ok(updated.is_some())
    }

    /// Release a previously-reserved capacity (when a VM is deleted).
    pub async fn release_reservation(
        &self,
        host_id: Uuid,
        vcpu: i32,
        mem_mib: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE host
                SET reserved_vcpu = GREATEST(0, reserved_vcpu - $2),
                    reserved_mem_mib = GREATEST(0, reserved_mem_mib - $3)
                WHERE id = $1"#,
        )
        .bind(host_id)
        .bind(vcpu)
        .bind(mem_mib)
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
}
