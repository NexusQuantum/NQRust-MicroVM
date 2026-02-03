use crate::features::metrics::repo;
use crate::AppState;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, warn};
use uuid::Uuid;

const COLLECT_INTERVAL_SECS: u64 = 10;
const HTTP_TIMEOUT_SECS: u64 = 2;
const MAX_CONCURRENT: usize = 10;

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(COLLECT_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(err) = collect_once(&state).await {
                warn!(error = ?err, "metrics collector iteration failed");
            }
        }
    })
}

async fn collect_once(state: &AppState) -> anyhow::Result<()> {
    let sem = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT));

    // Collect host, VM, and container metrics concurrently
    let (host_res, vm_res, container_res) = tokio::join!(
        collect_host_metrics(state),
        collect_vm_metrics(state, sem.clone()),
        collect_container_metrics(state, sem),
    );

    if let Err(e) = host_res {
        warn!(error = ?e, "host metrics collection failed");
    }
    if let Err(e) = vm_res {
        warn!(error = ?e, "vm metrics collection failed");
    }
    if let Err(e) = container_res {
        warn!(error = ?e, "container metrics collection failed");
    }

    // Purge old data (cheap indexed delete)
    if let Err(e) = repo::purge_old_metrics(&state.db).await {
        warn!(error = ?e, "metrics purge failed");
    }

    Ok(())
}

// ── Host metrics ────────────────────────────────────────────────────

async fn collect_host_metrics(state: &AppState) -> anyhow::Result<()> {
    let hosts = state.hosts.list_all().await?;

    for host in &hosts {
        // Host metrics are already in the host table (updated by agent heartbeat).
        // We snapshot them into the time-series table.
        let cpu = host.total_cpus.map(|_| {
            // The host table stores total_cpus but not current CPU usage %.
            // We don't have CPU usage from heartbeat — leave as None for now.
            // When agent starts reporting cpu_usage_percent, this can be populated.
            None::<f64>
        });
        let _ = cpu; // suppress unused

        // We don't have memory_used from heartbeat — only totals.
        // Store totals so dashboards can at least show capacity.
        let memory_used: Option<f64> = None;

        repo::insert_host_metric(
            &state.db,
            host.id,
            None,                                   // cpu_usage_percent (not in heartbeat)
            memory_used,                            // memory_used_mb
            host.total_memory_mb.map(|v| v as f64), // memory_total_mb
            host.used_disk_gb.map(|v| v as f64),    // disk_used_gb
            host.total_disk_gb.map(|v| v as f64),   // disk_total_gb
        )
        .await?;
    }

    debug!(count = hosts.len(), "collected host metrics");
    Ok(())
}

// ── VM metrics ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GuestMetrics {
    cpu_usage_percent: f64,
    memory_usage_percent: f64,
    memory_used_kb: u64,
    memory_total_kb: u64,
    #[allow(dead_code)]
    memory_available_kb: u64,
    #[allow(dead_code)]
    uptime_seconds: u64,
    load_average: Option<f64>,
    #[allow(dead_code)]
    process_count: Option<u32>,
}

async fn collect_vm_metrics(
    state: &AppState,
    sem: std::sync::Arc<Semaphore>,
) -> anyhow::Result<()> {
    let vms = crate::features::vms::repo::list(&state.db).await?;
    let running: Vec<_> = vms
        .into_iter()
        .filter(|vm| vm.state == "running" && vm.guest_ip.is_some())
        .collect();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;

    let mut handles = Vec::with_capacity(running.len());

    for vm in running {
        let pool = state.db.clone();
        let client = client.clone();
        let sem = sem.clone();
        let guest_ip = vm.guest_ip.clone().unwrap();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let url = format!("http://{}:9000/metrics", guest_ip);

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.json::<GuestMetrics>().await {
                    Ok(m) => {
                        if let Err(e) = repo::insert_vm_metric(
                            &pool,
                            vm.id,
                            Some(m.cpu_usage_percent),
                            Some(m.memory_usage_percent),
                            Some(m.memory_used_kb as i64),
                            Some(m.memory_total_kb as i64),
                            m.load_average,
                        )
                        .await
                        {
                            warn!(vm_id = %vm.id, error = ?e, "failed to insert vm metric");
                        }
                    }
                    Err(e) => {
                        debug!(vm_id = %vm.id, error = ?e, "failed to parse guest metrics");
                    }
                },
                Ok(resp) => {
                    debug!(vm_id = %vm.id, status = %resp.status(), "guest agent returned error");
                }
                Err(e) => {
                    debug!(vm_id = %vm.id, error = ?e, "failed to reach guest agent");
                }
            }
        }));
    }

    let count = handles.len();
    for h in handles {
        let _ = h.await;
    }
    debug!(count, "collected vm metrics");
    Ok(())
}

// ── Container metrics ───────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct RunningContainer {
    id: Uuid,
    guest_ip: Option<String>,
    container_runtime_id: Option<String>,
}

async fn collect_container_metrics(
    state: &AppState,
    sem: std::sync::Arc<Semaphore>,
) -> anyhow::Result<()> {
    // Get running containers with their VM guest IPs
    let containers = sqlx::query_as::<_, RunningContainer>(
        r#"
        SELECT c.id, v.guest_ip, c.container_runtime_id
        FROM containers c
        LEFT JOIN vm v ON c.container_runtime_id = 'vm-' || v.id::text
        WHERE c.state = 'running' AND v.guest_ip IS NOT NULL
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;

    let mut handles = Vec::with_capacity(containers.len());

    for container in containers {
        let pool = state.db.clone();
        let client = client.clone();
        let sem = sem.clone();
        let guest_ip = match container.guest_ip {
            Some(ip) => ip,
            None => continue,
        };
        // Docker container name is the same as our container name
        let runtime_id = match container.container_runtime_id {
            Some(id) => id,
            None => continue,
        };
        // The container_runtime_id stores "vm-{uuid}" for the VM,
        // but the Docker container is identified by the container name.
        // We need to query Docker for stats using the container name.
        // Use the Docker stats API with the container name.
        let container_id = container.id;

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            // Query Docker API using container name — containers are named after their DB name
            // But we need the Docker container ID. Since we store container_runtime_id as "vm-{uuid}",
            // the Docker container was created with the container's `name` field.
            // Use /containers/json to find the running container, or just use the name directly.
            // Docker API accepts container name for stats endpoint.
            let _ = runtime_id; // We use the container name approach below

            // List running Docker containers and match by name
            let list_url = format!("http://{}:2375/containers/json", guest_ip);
            let docker_containers = match client.get(&list_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<Vec<DockerContainerInfo>>().await {
                        Ok(list) => list,
                        Err(_) => return,
                    }
                }
                _ => return,
            };

            // Get stats for first running Docker container (container-per-VM architecture)
            if let Some(dc) = docker_containers.first() {
                let stats_url = format!(
                    "http://{}:2375/containers/{}/stats?stream=false",
                    guest_ip, dc.id
                );

                match client.get(&stats_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<DockerStatsRaw>().await {
                            Ok(stats) => {
                                let cpu = calculate_cpu_percent(&stats);
                                let mem_used =
                                    stats.memory_stats.usage.map(|u| u as f64 / 1024.0 / 1024.0);
                                let mem_limit =
                                    stats.memory_stats.limit.map(|l| l as f64 / 1024.0 / 1024.0);
                                let (rx, tx) = extract_network(&stats);
                                let (br, bw) = extract_block_io(&stats);

                                if let Err(e) = repo::insert_container_metric(
                                    &pool,
                                    container_id,
                                    Some(cpu),
                                    mem_used,
                                    mem_limit,
                                    Some(rx),
                                    Some(tx),
                                    Some(br),
                                    Some(bw),
                                    stats.pids_stats.current.map(|p| p as i32),
                                )
                                .await
                                {
                                    warn!(container_id = %container_id, error = ?e,
                                          "failed to insert container metric");
                                }
                            }
                            Err(e) => {
                                debug!(container_id = %container_id, error = ?e,
                                       "failed to parse docker stats");
                            }
                        }
                    }
                    _ => {}
                }
            }
        }));
    }

    let count = handles.len();
    for h in handles {
        let _ = h.await;
    }
    debug!(count, "collected container metrics");
    Ok(())
}

// ── Docker stats response types (minimal, for collector only) ───────

#[derive(Debug, Deserialize)]
struct DockerContainerInfo {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct DockerStatsRaw {
    cpu_stats: CpuStats,
    precpu_stats: CpuStats,
    memory_stats: MemStats,
    #[serde(default)]
    networks: std::collections::HashMap<String, NetStats>,
    #[serde(default)]
    blkio_stats: BlkioStats,
    pids_stats: PidsStats,
}

#[derive(Debug, Deserialize, Default)]
struct CpuStats {
    cpu_usage: CpuUsage,
    system_cpu_usage: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct CpuUsage {
    total_usage: u64,
}

#[derive(Debug, Deserialize, Default)]
struct MemStats {
    usage: Option<u64>,
    limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct NetStats {
    rx_bytes: u64,
    tx_bytes: u64,
}

#[derive(Debug, Deserialize, Default)]
struct BlkioStats {
    #[serde(default)]
    io_service_bytes_recursive: Vec<BlkioEntry>,
}

#[derive(Debug, Deserialize)]
struct BlkioEntry {
    op: String,
    value: u64,
}

#[derive(Debug, Deserialize, Default)]
struct PidsStats {
    current: Option<u64>,
}

fn calculate_cpu_percent(stats: &DockerStatsRaw) -> f64 {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
        - stats.precpu_stats.cpu_usage.total_usage as f64;
    let sys_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    if sys_delta > 0.0 && cpu_delta > 0.0 {
        cpu_delta / sys_delta * 100.0
    } else {
        0.0
    }
}

fn extract_network(stats: &DockerStatsRaw) -> (i64, i64) {
    let mut rx = 0i64;
    let mut tx = 0i64;
    for n in stats.networks.values() {
        rx += n.rx_bytes as i64;
        tx += n.tx_bytes as i64;
    }
    (rx, tx)
}

fn extract_block_io(stats: &DockerStatsRaw) -> (i64, i64) {
    let mut read = 0i64;
    let mut write = 0i64;
    for e in &stats.blkio_stats.io_service_bytes_recursive {
        match e.op.as_str() {
            "Read" => read += e.value as i64,
            "Write" => write += e.value as i64,
            _ => {}
        }
    }
    (read, write)
}
