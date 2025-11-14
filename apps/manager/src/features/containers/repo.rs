use anyhow::{Context, Result};
use chrono::Utc;
use nexus_types::{
    Container, ContainerLog, ContainerStats, CreateContainerReq, UpdateContainerReq,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct ContainerRepository {
    db: PgPool,
}

impl ContainerRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create(&self, req: CreateContainerReq, host_id: Option<Uuid>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let args_json = serde_json::to_value(&req.args)?;
        let env_vars_json = serde_json::to_value(&req.env_vars)?;
        let volumes_json = serde_json::to_value(&req.volumes)?;
        let port_mappings_json = serde_json::to_value(&req.port_mappings)?;

        sqlx::query(
            r#"
            INSERT INTO containers (
                id, name, image, command, args, env_vars, volumes, port_mappings,
                cpu_limit, memory_limit_mb, restart_policy, state, host_id,
                created_by_user_id, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.image)
        .bind(&req.command)
        .bind(args_json)
        .bind(env_vars_json)
        .bind(volumes_json)
        .bind(port_mappings_json)
        .bind(req.cpu_limit)
        .bind(req.memory_limit_mb)
        .bind(&req.restart_policy)
        .bind("creating")
        .bind(host_id)
        .bind(None::<Option<Uuid>>) // created_by_user_id - TODO: Set from authenticated user context
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await
        .context("failed to insert container")?;

        Ok(id)
    }

    pub async fn get(&self, id: Uuid) -> Result<Container> {
        let row = sqlx::query_as::<_, ContainerRow>(
            r#"
            SELECT
                c.id, c.name, c.image, c.command, c.args, c.env_vars, c.volumes, c.port_mappings,
                c.cpu_limit, c.memory_limit_mb, c.restart_policy, c.state, c.host_id,
                c.container_runtime_id, c.error_message, c.created_by_user_id, c.created_at, c.updated_at,
                c.started_at, c.stopped_at,
                v.guest_ip
            FROM containers c
            LEFT JOIN vm v ON c.container_runtime_id = 'vm-' || v.id::text
            WHERE c.id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.db)
        .await
        .context("container not found")?;

        let args: Vec<String> =
            serde_json::from_value(row.args.unwrap_or_else(|| serde_json::json!([])))?;
        let env_vars: std::collections::HashMap<String, String> =
            serde_json::from_value(row.env_vars.unwrap_or_else(|| serde_json::json!({})))?;
        let volumes: Vec<nexus_types::VolumeMount> =
            serde_json::from_value(row.volumes.unwrap_or_else(|| serde_json::json!([])))?;
        let port_mappings: Vec<nexus_types::PortMapping> =
            serde_json::from_value(row.port_mappings.unwrap_or_else(|| serde_json::json!([])))?;

        let uptime_seconds = if row.state == "running" {
            row.started_at
                .map(|started| (Utc::now() - started).num_seconds())
        } else {
            None
        };

        Ok(Container {
            id: row.id,
            name: row.name,
            image: row.image,
            command: row.command,
            args,
            env_vars,
            volumes,
            port_mappings,
            cpu_limit: row.cpu_limit,
            memory_limit_mb: row.memory_limit_mb,
            restart_policy: row.restart_policy.unwrap_or_else(|| "no".to_string()),
            state: row.state,
            host_id: row.host_id,
            container_runtime_id: row.container_runtime_id,
            error_message: row.error_message,
            created_by_user_id: row.created_by_user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            started_at: row.started_at,
            stopped_at: row.stopped_at,
            uptime_seconds,
            cpu_percent: None,
            memory_used_mb: None,
            guest_ip: row.guest_ip,
        })
    }

    pub async fn list(
        &self,
        state_filter: Option<String>,
        host_filter: Option<Uuid>,
    ) -> Result<Vec<Container>> {
        let mut query_str = String::from(
            r#"
            SELECT
                c.id, c.name, c.image, c.command, c.args, c.env_vars, c.volumes, c.port_mappings,
                c.cpu_limit, c.memory_limit_mb, c.restart_policy, c.state, c.host_id,
                c.container_runtime_id, c.error_message, c.created_by_user_id, c.created_at, c.updated_at,
                c.started_at, c.stopped_at,
                v.guest_ip
            FROM containers c
            LEFT JOIN vm v ON c.container_runtime_id = 'vm-' || v.id::text
            WHERE 1=1
            "#,
        );

        if state_filter.is_some() {
            query_str.push_str(" AND state = $1");
        }
        if host_filter.is_some() {
            if state_filter.is_some() {
                query_str.push_str(" AND host_id = $2");
            } else {
                query_str.push_str(" AND host_id = $1");
            }
        }
        query_str.push_str(" ORDER BY created_at DESC");

        let mut query = sqlx::query_as::<_, ContainerRow>(&query_str);

        if let Some(state) = &state_filter {
            query = query.bind(state);
        }
        if let Some(host_id) = host_filter {
            query = query.bind(host_id);
        }

        let rows = query.fetch_all(&self.db).await?;

        let containers = rows
            .into_iter()
            .map(|row| {
                let uptime_seconds = if row.state == "running" {
                    row.started_at
                        .map(|started| (Utc::now() - started).num_seconds())
                } else {
                    None
                };

                Ok(Container {
                    id: row.id,
                    name: row.name,
                    image: row.image,
                    command: row.command,
                    args: serde_json::from_value(
                        row.args.unwrap_or_else(|| serde_json::json!([])),
                    )?,
                    env_vars: serde_json::from_value(
                        row.env_vars.unwrap_or_else(|| serde_json::json!({})),
                    )?,
                    volumes: serde_json::from_value(
                        row.volumes.unwrap_or_else(|| serde_json::json!([])),
                    )?,
                    port_mappings: serde_json::from_value(
                        row.port_mappings.unwrap_or_else(|| serde_json::json!([])),
                    )?,
                    cpu_limit: row.cpu_limit,
                    memory_limit_mb: row.memory_limit_mb,
                    restart_policy: row.restart_policy.unwrap_or_else(|| "no".to_string()),
                    state: row.state,
                    host_id: row.host_id,
                    container_runtime_id: row.container_runtime_id,
                    error_message: row.error_message,
                    created_by_user_id: row.created_by_user_id,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    started_at: row.started_at,
                    stopped_at: row.stopped_at,
                    uptime_seconds,
                    cpu_percent: None,
                    memory_used_mb: None,
                    guest_ip: row.guest_ip,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(containers)
    }

    pub async fn update(&self, id: Uuid, req: UpdateContainerReq) -> Result<()> {
        let now = Utc::now();

        let mut updates = vec!["updated_at = $1".to_string()];
        let mut bind_index = 2;

        if req.name.is_some() {
            updates.push(format!("name = ${}", bind_index));
            bind_index += 1;
        }
        if req.env_vars.is_some() {
            updates.push(format!("env_vars = ${}", bind_index));
            bind_index += 1;
        }
        if req.cpu_limit.is_some() {
            updates.push(format!("cpu_limit = ${}", bind_index));
            bind_index += 1;
        }
        if req.memory_limit_mb.is_some() {
            updates.push(format!("memory_limit_mb = ${}", bind_index));
            bind_index += 1;
        }
        if req.restart_policy.is_some() {
            updates.push(format!("restart_policy = ${}", bind_index));
            bind_index += 1;
        }

        let query_str = format!(
            "UPDATE containers SET {} WHERE id = ${}",
            updates.join(", "),
            bind_index
        );

        let mut query = sqlx::query(&query_str).bind(now);

        if let Some(name) = req.name {
            query = query.bind(name);
        }
        if let Some(env_vars) = req.env_vars {
            query = query.bind(serde_json::to_value(env_vars)?);
        }
        if let Some(cpu_limit) = req.cpu_limit {
            query = query.bind(cpu_limit);
        }
        if let Some(memory_limit_mb) = req.memory_limit_mb {
            query = query.bind(memory_limit_mb);
        }
        if let Some(restart_policy) = req.restart_policy {
            query = query.bind(restart_policy);
        }

        query = query.bind(id);

        query.execute(&self.db).await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM containers WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await
            .context("failed to delete container")?;
        Ok(())
    }

    pub async fn update_state(
        &self,
        id: Uuid,
        state: &str,
        error_message: Option<String>,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE containers
            SET state = $1, error_message = $2, updated_at = $3
            WHERE id = $4
            "#,
        )
        .bind(state)
        .bind(error_message.as_ref())
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn update_runtime_id(&self, id: Uuid, runtime_id: String) -> Result<()> {
        sqlx::query("UPDATE containers SET container_runtime_id = $1 WHERE id = $2")
            .bind(runtime_id)
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn set_started(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE containers
            SET state = 'running', started_at = $1, stopped_at = NULL, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn set_stopped(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE containers
            SET state = 'stopped', stopped_at = $1, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn record_stats(&self, container_id: Uuid, stats: &ContainerStatsData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO container_stats (
                container_id, cpu_percent, memory_used_mb, memory_limit_mb,
                network_rx_bytes, network_tx_bytes, block_read_bytes, block_write_bytes,
                pids, recorded_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(container_id)
        .bind(stats.cpu_percent)
        .bind(stats.memory_used_mb)
        .bind(stats.memory_limit_mb)
        .bind(stats.network_rx_bytes)
        .bind(stats.network_tx_bytes)
        .bind(stats.block_read_bytes)
        .bind(stats.block_write_bytes)
        .bind(stats.pids)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn get_latest_stats(
        &self,
        container_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ContainerStats>> {
        let rows = sqlx::query_as::<_, ContainerStatsRow>(
            r#"
            SELECT id, container_id, cpu_percent, memory_used_mb, memory_limit_mb,
                   network_rx_bytes, network_tx_bytes, block_read_bytes, block_write_bytes,
                   pids, recorded_at
            FROM container_stats
            WHERE container_id = $1
            ORDER BY recorded_at DESC
            LIMIT $2
            "#,
        )
        .bind(container_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ContainerStats {
                id: row.id,
                container_id: row.container_id,
                cpu_percent: row.cpu_percent,
                memory_used_mb: row.memory_used_mb,
                memory_limit_mb: row.memory_limit_mb,
                network_rx_bytes: row.network_rx_bytes,
                network_tx_bytes: row.network_tx_bytes,
                block_read_bytes: row.block_read_bytes,
                block_write_bytes: row.block_write_bytes,
                pids: row.pids,
                recorded_at: row.recorded_at,
            })
            .collect())
    }

    #[allow(dead_code)]
    pub async fn append_log(
        &self,
        container_id: Uuid,
        stream: &str,
        message: String,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO container_logs (container_id, stream, message, timestamp)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(container_id)
        .bind(stream)
        .bind(message)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn get_logs(
        &self,
        container_id: Uuid,
        tail: Option<i64>,
    ) -> Result<Vec<ContainerLog>> {
        let limit = tail.unwrap_or(100);

        let rows = sqlx::query_as::<_, ContainerLogRow>(
            r#"
            SELECT id, container_id, timestamp, stream, message, created_at
            FROM container_logs
            WHERE container_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(container_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ContainerLog {
                id: row.id,
                container_id: row.container_id,
                timestamp: row.timestamp,
                stream: row.stream,
                message: row.message,
                created_at: row.created_at,
            })
            .collect())
    }
}

// Helper struct for query results
#[derive(sqlx::FromRow)]
struct ContainerRow {
    id: Uuid,
    name: String,
    image: String,
    command: Option<String>,
    args: Option<serde_json::Value>,
    env_vars: Option<serde_json::Value>,
    volumes: Option<serde_json::Value>,
    port_mappings: Option<serde_json::Value>,
    cpu_limit: Option<f32>,
    memory_limit_mb: Option<i32>,
    restart_policy: Option<String>,
    state: String,
    host_id: Option<Uuid>,
    container_runtime_id: Option<String>,
    error_message: Option<String>,
    created_by_user_id: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    started_at: Option<chrono::DateTime<Utc>>,
    stopped_at: Option<chrono::DateTime<Utc>>,
    guest_ip: Option<String>,
}

// Helper structs for query results
#[derive(sqlx::FromRow)]
struct ContainerStatsRow {
    id: Uuid,
    container_id: Uuid,
    cpu_percent: Option<f32>,
    memory_used_mb: Option<i64>,
    memory_limit_mb: Option<i64>,
    network_rx_bytes: Option<i64>,
    network_tx_bytes: Option<i64>,
    block_read_bytes: Option<i64>,
    block_write_bytes: Option<i64>,
    pids: Option<i32>,
    recorded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct ContainerLogRow {
    id: Uuid,
    container_id: Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
    stream: String,
    message: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

// Stats data structure for recording
pub struct ContainerStatsData {
    pub cpu_percent: Option<f32>,
    pub memory_used_mb: Option<i64>,
    pub memory_limit_mb: Option<i64>,
    pub network_rx_bytes: Option<i64>,
    pub network_tx_bytes: Option<i64>,
    pub block_read_bytes: Option<i64>,
    pub block_write_bytes: Option<i64>,
    pub pids: Option<i32>,
}
