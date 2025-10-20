use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize, Clone)]
struct GuestMetrics {
    cpu_usage_percent: f64,
    memory_usage_percent: f64,
    memory_used_kb: u64,
    memory_total_kb: u64,
    memory_available_kb: u64,
    uptime_seconds: u64,
}

/// Read CPU statistics from /proc/stat
/// Returns (user, nice, system, idle, iowait, irq, softirq)
fn read_cpu_stats() -> Result<(u64, u64, u64, u64, u64, u64, u64), String> {
    let stat = fs::read_to_string("/proc/stat").map_err(|e| e.to_string())?;

    // First line is aggregate CPU stats: "cpu user nice system idle iowait irq softirq ..."
    let line = stat.lines().next().ok_or("Empty /proc/stat")?;
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 8 || parts[0] != "cpu" {
        return Err("Invalid /proc/stat format".to_string());
    }

    let user = parts[1].parse().unwrap_or(0);
    let nice = parts[2].parse().unwrap_or(0);
    let system = parts[3].parse().unwrap_or(0);
    let idle = parts[4].parse().unwrap_or(0);
    let iowait = parts[5].parse().unwrap_or(0);
    let irq = parts[6].parse().unwrap_or(0);
    let softirq = parts[7].parse().unwrap_or(0);

    Ok((user, nice, system, idle, iowait, irq, softirq))
}

/// Calculate CPU usage percentage between two samples
fn calculate_cpu_percent(
    prev: (u64, u64, u64, u64, u64, u64, u64),
    curr: (u64, u64, u64, u64, u64, u64, u64),
) -> f64 {
    let (prev_user, prev_nice, prev_system, prev_idle, prev_iowait, prev_irq, prev_softirq) = prev;
    let (curr_user, curr_nice, curr_system, curr_idle, curr_iowait, curr_irq, curr_softirq) = curr;

    let prev_idle_total = prev_idle + prev_iowait;
    let curr_idle_total = curr_idle + curr_iowait;

    let prev_non_idle = prev_user + prev_nice + prev_system + prev_irq + prev_softirq;
    let curr_non_idle = curr_user + curr_nice + curr_system + curr_irq + curr_softirq;

    let prev_total = prev_idle_total + prev_non_idle;
    let curr_total = curr_idle_total + curr_non_idle;

    let total_diff = curr_total.saturating_sub(prev_total);
    let idle_diff = curr_idle_total.saturating_sub(prev_idle_total);

    if total_diff == 0 {
        return 0.0;
    }

    let usage = total_diff.saturating_sub(idle_diff);
    (usage as f64 / total_diff as f64) * 100.0
}

/// Read memory statistics from /proc/meminfo
fn read_memory_stats() -> Result<(u64, u64, u64), String> {
    let meminfo = fs::read_to_string("/proc/meminfo").map_err(|e| e.to_string())?;

    let mut mem_total = 0u64;
    let mut mem_available = 0u64;
    let mut mem_free = 0u64;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            mem_available = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemFree:") {
            mem_free = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    // If MemAvailable is not available (older kernels), use MemFree
    if mem_available == 0 {
        mem_available = mem_free;
    }

    let mem_used = mem_total.saturating_sub(mem_available);

    Ok((mem_total, mem_used, mem_available))
}

/// Read system uptime from /proc/uptime
fn read_uptime() -> Result<u64, String> {
    let uptime = fs::read_to_string("/proc/uptime").map_err(|e| e.to_string())?;
    let uptime_secs: f64 = uptime
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    Ok(uptime_secs as u64)
}

/// Global state for CPU metrics calculation
struct MetricsState {
    last_cpu_stats: Option<(u64, u64, u64, u64, u64, u64, u64)>,
    current_cpu_percent: f64,
}

impl MetricsState {
    fn new() -> Self {
        Self {
            last_cpu_stats: None,
            current_cpu_percent: 0.0,
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    tracing::info!("Guest agent starting...");

    // Shared state for metrics
    let state = std::sync::Arc::new(tokio::sync::Mutex::new(MetricsState::new()));

    // Spawn background task to update CPU metrics
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(cpu_stats) = read_cpu_stats() {
                let mut state = state_clone.lock().await;
                if let Some(last_stats) = state.last_cpu_stats {
                    state.current_cpu_percent = calculate_cpu_percent(last_stats, cpu_stats);
                }
                state.last_cpu_stats = Some(cpu_stats);
            }

            sleep(Duration::from_secs(1)).await;
        }
    });

    // Build HTTP router
    let app = Router::new()
        .route("/metrics", get({
            let state = state.clone();
            move || metrics_handler(state)
        }))
        .route("/health", get(health_handler));

    // Bind to all interfaces on port 8080
    let addr = "0.0.0.0:8080";
    tracing::info!("Guest agent listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn metrics_handler(
    state: std::sync::Arc<tokio::sync::Mutex<MetricsState>>,
) -> Json<GuestMetrics> {
    let cpu_percent = {
        let state = state.lock().await;
        state.current_cpu_percent
    };

    let (mem_total, mem_used, mem_available) = read_memory_stats().unwrap_or((0, 0, 0));
    let mem_percent = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };

    let uptime = read_uptime().unwrap_or(0);

    Json(GuestMetrics {
        cpu_usage_percent: cpu_percent,
        memory_usage_percent: mem_percent,
        memory_used_kb: mem_used,
        memory_total_kb: mem_total,
        memory_available_kb: mem_available,
        uptime_seconds: uptime,
    })
}

async fn health_handler() -> &'static str {
    "OK"
}
