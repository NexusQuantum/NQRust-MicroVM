use axum::{http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

#[derive(Deserialize)]
struct PrepareMetricsReq {
    metrics_path: String,
}

#[derive(Serialize)]
struct PrepareMetricsResp {
    metrics_path: String,
}

#[derive(Deserialize)]
struct ProcessStatsReq {
    sock_path: String,
}

#[derive(Serialize)]
struct ProcessStatsResp {
    pid: u32,
    cpu_percent: f64,
    memory_rss_kb: u64,
    memory_percent: f64,
}

pub fn router() -> Router {
    Router::new()
        .route("/:id/metrics/prepare", post(prepare))
        .route("/:id/metrics/process-stats", post(get_process_stats))
}

async fn prepare(
    Json(req): Json<PrepareMetricsReq>,
) -> Result<Json<PrepareMetricsResp>, (StatusCode, String)> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&req.metrics_path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_error)?;
    }

    // If file exists but is not a FIFO, remove it
    if let Ok(md) = tokio::fs::metadata(&req.metrics_path).await {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt as _;
            if !md.file_type().is_fifo() {
                tokio::fs::remove_file(&req.metrics_path)
                    .await
                    .map_err(internal_error)?;
            }
        }
        #[cfg(not(unix))]
        {
            // On non-unix, just remove and recreate
            tokio::fs::remove_file(&req.metrics_path)
                .await
                .map_err(internal_error)?;
        }
    }

    // Create FIFO if missing
    if tokio::fs::symlink_metadata(&req.metrics_path)
        .await
        .is_err()
    {
        let status = Command::new("sudo")
            .args(["-n", "mkfifo", &req.metrics_path])
            .status()
            .await
            .map_err(internal_error)?;
        if !status.success() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create metrics fifo".into(),
            ));
        }
    }

    // Set permissive permissions so Firecracker can open it
    let _ = Command::new("sudo")
        .args(["-n", "chmod", "666", &req.metrics_path])
        .status()
        .await;

    Ok(Json(PrepareMetricsResp {
        metrics_path: req.metrics_path,
    }))
}

async fn get_process_stats(
    Json(req): Json<ProcessStatsReq>,
) -> Result<Json<ProcessStatsResp>, (StatusCode, String)> {
    // Find the Firecracker PID by looking for the process with the given socket
    let pid = find_firecracker_pid(&req.sock_path).await?;

    // Read CPU and memory stats from /proc
    let (cpu_percent, memory_rss_kb, memory_percent) = read_process_stats(pid).await?;

    Ok(Json(ProcessStatsResp {
        pid,
        cpu_percent,
        memory_rss_kb,
        memory_percent,
    }))
}

async fn find_firecracker_pid(sock_path: &str) -> Result<u32, (StatusCode, String)> {
    // Use lsof to find which process has the socket open
    let output = Command::new("sudo")
        .args(["-n", "lsof", "-t", sock_path])
        .output()
        .await
        .map_err(internal_error)?;

    if !output.status.success() {
        return Err((
            StatusCode::NOT_FOUND,
            "Could not find process for socket".into(),
        ));
    }

    let pid_str = String::from_utf8_lossy(&output.stdout);
    let pid = pid_str
        .trim()
        .lines()
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse PID".to_string(),
            )
        })?;

    Ok(pid)
}

async fn read_process_stats(pid: u32) -> Result<(f64, u64, f64), (StatusCode, String)> {
    // Read /proc/{pid}/stat for CPU stats
    let stat_path = format!("/proc/{}/stat", pid);
    let stat_content = fs::read_to_string(&stat_path)
        .await
        .map_err(internal_error)?;

    // Read /proc/{pid}/status for memory stats
    let status_path = format!("/proc/{}/status", pid);
    let status_content = fs::read_to_string(&status_path)
        .await
        .map_err(internal_error)?;

    // Parse /proc/{pid}/stat
    // Format: pid (comm) state ppid ... utime stime ...
    let parts: Vec<&str> = stat_content.split_whitespace().collect();
    if parts.len() < 15 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid /proc/stat format".into(),
        ));
    }

    let utime: u64 = parts[13].parse().unwrap_or(0);
    let stime: u64 = parts[14].parse().unwrap_or(0);
    let total_time = utime + stime;

    // Read system uptime to calculate CPU percentage
    let uptime_content = fs::read_to_string("/proc/uptime")
        .await
        .map_err(internal_error)?;
    let system_uptime: f64 = uptime_content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);

    // CPU percentage = (total_time / clock_ticks) / uptime * 100
    // clock_ticks is usually 100 (sysconf(_SC_CLK_TCK))
    let clock_ticks = 100.0;
    let process_uptime = total_time as f64 / clock_ticks;
    let cpu_percent = if system_uptime > 0.0 {
        (process_uptime / system_uptime) * 100.0
    } else {
        0.0
    };

    // Parse VmRSS from /proc/{pid}/status
    let mut memory_rss_kb = 0u64;
    for line in status_content.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(value) = line.split_whitespace().nth(1) {
                memory_rss_kb = value.parse().unwrap_or(0);
                break;
            }
        }
    }

    // Calculate memory percentage based on system total memory
    let meminfo_content = fs::read_to_string("/proc/meminfo")
        .await
        .map_err(internal_error)?;
    let mut total_memory_kb = 0u64;
    for line in meminfo_content.lines() {
        if line.starts_with("MemTotal:") {
            if let Some(value) = line.split_whitespace().nth(1) {
                total_memory_kb = value.parse().unwrap_or(0);
                break;
            }
        }
    }

    let memory_percent = if total_memory_kb > 0 {
        (memory_rss_kb as f64 / total_memory_kb as f64) * 100.0
    } else {
        0.0
    };

    Ok((cpu_percent, memory_rss_kb, memory_percent))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
