use nexus_types::{ContainerMetric, HostMetric, VmMetric};
use sqlx::PgPool;
use uuid::Uuid;

// ── Insert helpers (used by collector) ──────────────────────────────

pub async fn insert_host_metric(
    pool: &PgPool,
    host_id: Uuid,
    cpu_usage_percent: Option<f64>,
    memory_used_mb: Option<f64>,
    memory_total_mb: Option<f64>,
    disk_used_gb: Option<f64>,
    disk_total_gb: Option<f64>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO metrics.host_metrics
            (host_id, cpu_usage_percent, memory_used_mb, memory_total_mb, disk_used_gb, disk_total_gb)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(host_id)
    .bind(cpu_usage_percent)
    .bind(memory_used_mb)
    .bind(memory_total_mb)
    .bind(disk_used_gb)
    .bind(disk_total_gb)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_vm_metric(
    pool: &PgPool,
    vm_id: Uuid,
    cpu_usage_percent: Option<f64>,
    memory_usage_percent: Option<f64>,
    memory_used_kb: Option<i64>,
    memory_total_kb: Option<i64>,
    load_average: Option<f64>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO metrics.vm_metrics
            (vm_id, cpu_usage_percent, memory_usage_percent, memory_used_kb, memory_total_kb, load_average)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(vm_id)
    .bind(cpu_usage_percent)
    .bind(memory_usage_percent)
    .bind(memory_used_kb)
    .bind(memory_total_kb)
    .bind(load_average)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_container_metric(
    pool: &PgPool,
    container_id: Uuid,
    cpu_percent: Option<f64>,
    memory_used_mb: Option<f64>,
    memory_limit_mb: Option<f64>,
    network_rx_bytes: Option<i64>,
    network_tx_bytes: Option<i64>,
    block_read_bytes: Option<i64>,
    block_write_bytes: Option<i64>,
    pids: Option<i32>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO metrics.container_metrics
            (container_id, cpu_percent, memory_used_mb, memory_limit_mb,
             network_rx_bytes, network_tx_bytes, block_read_bytes, block_write_bytes, pids)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(container_id)
    .bind(cpu_percent)
    .bind(memory_used_mb)
    .bind(memory_limit_mb)
    .bind(network_rx_bytes)
    .bind(network_tx_bytes)
    .bind(block_read_bytes)
    .bind(block_write_bytes)
    .bind(pids)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Query helpers (used by routes) ──────────────────────────────────

pub async fn query_host_metrics(
    pool: &PgPool,
    host_id: Uuid,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<HostMetric>> {
    sqlx::query_as::<_, HostMetricRow>(
        r#"
        SELECT host_id, recorded_at, cpu_usage_percent, memory_used_mb,
               memory_total_mb, disk_used_gb, disk_total_gb
        FROM metrics.host_metrics
        WHERE host_id = $1
          AND ($2::timestamptz IS NULL OR recorded_at >= $2)
          AND ($3::timestamptz IS NULL OR recorded_at <= $3)
        ORDER BY recorded_at DESC
        LIMIT $4
        "#,
    )
    .bind(host_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(Into::into).collect())
}

pub async fn query_vm_metrics(
    pool: &PgPool,
    vm_id: Uuid,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<VmMetric>> {
    sqlx::query_as::<_, VmMetricRow>(
        r#"
        SELECT vm_id, recorded_at, cpu_usage_percent, memory_usage_percent,
               memory_used_kb, memory_total_kb, load_average
        FROM metrics.vm_metrics
        WHERE vm_id = $1
          AND ($2::timestamptz IS NULL OR recorded_at >= $2)
          AND ($3::timestamptz IS NULL OR recorded_at <= $3)
        ORDER BY recorded_at DESC
        LIMIT $4
        "#,
    )
    .bind(vm_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(Into::into).collect())
}

pub async fn query_container_metrics(
    pool: &PgPool,
    container_id: Uuid,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<ContainerMetric>> {
    sqlx::query_as::<_, ContainerMetricRow>(
        r#"
        SELECT container_id, recorded_at, cpu_percent, memory_used_mb, memory_limit_mb,
               network_rx_bytes, network_tx_bytes, block_read_bytes, block_write_bytes, pids
        FROM metrics.container_metrics
        WHERE container_id = $1
          AND ($2::timestamptz IS NULL OR recorded_at >= $2)
          AND ($3::timestamptz IS NULL OR recorded_at <= $3)
        ORDER BY recorded_at DESC
        LIMIT $4
        "#,
    )
    .bind(container_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(Into::into).collect())
}

pub async fn purge_old_metrics(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query("SELECT metrics.purge_old_metrics()")
        .execute(pool)
        .await?;
    Ok(())
}

// ── Row types ───────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct HostMetricRow {
    host_id: Uuid,
    recorded_at: chrono::DateTime<chrono::Utc>,
    cpu_usage_percent: Option<f64>,
    memory_used_mb: Option<f64>,
    memory_total_mb: Option<f64>,
    disk_used_gb: Option<f64>,
    disk_total_gb: Option<f64>,
}

impl From<HostMetricRow> for HostMetric {
    fn from(r: HostMetricRow) -> Self {
        Self {
            host_id: r.host_id,
            recorded_at: r.recorded_at,
            cpu_usage_percent: r.cpu_usage_percent,
            memory_used_mb: r.memory_used_mb,
            memory_total_mb: r.memory_total_mb,
            disk_used_gb: r.disk_used_gb,
            disk_total_gb: r.disk_total_gb,
        }
    }
}

#[derive(sqlx::FromRow)]
struct VmMetricRow {
    vm_id: Uuid,
    recorded_at: chrono::DateTime<chrono::Utc>,
    cpu_usage_percent: Option<f64>,
    memory_usage_percent: Option<f64>,
    memory_used_kb: Option<i64>,
    memory_total_kb: Option<i64>,
    load_average: Option<f64>,
}

impl From<VmMetricRow> for VmMetric {
    fn from(r: VmMetricRow) -> Self {
        Self {
            vm_id: r.vm_id,
            recorded_at: r.recorded_at,
            cpu_usage_percent: r.cpu_usage_percent,
            memory_usage_percent: r.memory_usage_percent,
            memory_used_kb: r.memory_used_kb,
            memory_total_kb: r.memory_total_kb,
            load_average: r.load_average,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ContainerMetricRow {
    container_id: Uuid,
    recorded_at: chrono::DateTime<chrono::Utc>,
    cpu_percent: Option<f64>,
    memory_used_mb: Option<f64>,
    memory_limit_mb: Option<f64>,
    network_rx_bytes: Option<i64>,
    network_tx_bytes: Option<i64>,
    block_read_bytes: Option<i64>,
    block_write_bytes: Option<i64>,
    pids: Option<i32>,
}

impl From<ContainerMetricRow> for ContainerMetric {
    fn from(r: ContainerMetricRow) -> Self {
        Self {
            container_id: r.container_id,
            recorded_at: r.recorded_at,
            cpu_percent: r.cpu_percent,
            memory_used_mb: r.memory_used_mb,
            memory_limit_mb: r.memory_limit_mb,
            network_rx_bytes: r.network_rx_bytes,
            network_tx_bytes: r.network_tx_bytes,
            block_read_bytes: r.block_read_bytes,
            block_write_bytes: r.block_write_bytes,
            pids: r.pids,
        }
    }
}
