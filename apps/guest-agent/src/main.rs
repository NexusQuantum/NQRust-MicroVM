use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// CPU statistics tuple: (user, nice, system, idle, iowait, irq, softirq)
type CpuStats = (u64, u64, u64, u64, u64, u64, u64);

#[derive(Debug, Clone)]
struct AgentConfig {
    vm_id: String,
    manager_url: String,
}

#[derive(Debug, Serialize, Clone)]
struct GuestMetrics {
    cpu_usage_percent: f64,
    memory_usage_percent: f64,
    memory_used_kb: u64,
    memory_total_kb: u64,
    memory_available_kb: u64,
    uptime_seconds: u64,
    load_average: Option<f64>,
    process_count: Option<u32>,
}

/// Read CPU statistics from /proc/stat
/// Returns (user, nice, system, idle, iowait, irq, softirq)
fn read_cpu_stats() -> Result<CpuStats, String> {
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
fn calculate_cpu_percent(prev: CpuStats, curr: CpuStats) -> f64 {
    let (prev_user, prev_nice, prev_system, prev_idle, prev_iowait, prev_irq, prev_softirq) = prev;
    let (curr_user, curr_nice, curr_system, curr_idle, curr_iowait, curr_irq, curr_softirq) = curr;

    let prev_idle_total = prev_idle + prev_iowait;
    let curr_idle_total = curr_idle + curr_iowait;

    let prev_total =
        prev_user + prev_nice + prev_system + prev_idle + prev_iowait + prev_irq + prev_softirq;
    let curr_total =
        curr_user + curr_nice + curr_system + curr_idle + curr_iowait + curr_irq + curr_softirq;

    let total_diff = curr_total.saturating_sub(prev_total);
    let idle_diff = curr_idle_total.saturating_sub(prev_idle_total);

    if total_diff == 0 {
        return 0.0;
    }

    let usage_percent = ((total_diff - idle_diff) as f64 / total_diff as f64) * 100.0;
    usage_percent.clamp(0.0, 100.0)
}

/// Read memory statistics from /proc/meminfo
/// Works across all Linux distributions
fn read_memory_stats() -> Result<(u64, u64, u64), String> {
    let meminfo = fs::read_to_string("/proc/meminfo").map_err(|e| e.to_string())?;

    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    let mut free_kb = 0u64;
    let mut buffers_kb = 0u64;
    let mut cached_kb = 0u64;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                total_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("MemAvailable:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                available_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("MemFree:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                free_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("Buffers:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                buffers_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("Cached:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                cached_kb = kb_str.parse().unwrap_or(0);
            }
        }
    }

    if total_kb == 0 {
        return Err("Could not parse memory total".to_string());
    }

    // If MemAvailable is not available (older kernels), calculate it
    if available_kb == 0 {
        available_kb = free_kb + buffers_kb + cached_kb;
    }

    let used_kb = total_kb.saturating_sub(available_kb);
    let usage_percent = if total_kb > 0 {
        (used_kb as f64 / total_kb as f64) * 100.0
    } else {
        0.0
    };

    Ok((total_kb, used_kb, usage_percent as u64))
}

/// Read uptime from /proc/uptime
fn read_uptime() -> Result<u64, String> {
    let uptime = fs::read_to_string("/proc/uptime").map_err(|e| e.to_string())?;
    let seconds_str = uptime.split_whitespace().next().unwrap_or("0");
    Ok(seconds_str.parse().unwrap_or(0.0) as u64)
}

/// Read load average from /proc/loadavg
fn read_load_average() -> Option<f64> {
    let loadavg = fs::read_to_string("/proc/loadavg").ok()?;
    let load_str = loadavg.split_whitespace().next()?;
    load_str.parse().ok()
}

/// Count processes in /proc
fn count_processes() -> Option<u32> {
    let proc_entries = fs::read_dir("/proc").ok()?;
    let count = proc_entries
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.chars().all(|c| c.is_ascii_digit()))
            })
        })
        .count();
    Some(count as u32)
}

/// Read guest agent configuration from /etc/guest-agent.conf
fn read_config() -> Option<AgentConfig> {
    let config_content = fs::read_to_string("/etc/guest-agent.conf").ok()?;

    let mut vm_id = None;
    let mut manager_url = None;

    for line in config_content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "VM_ID" => vm_id = Some(value.trim().to_string()),
                "MANAGER_URL" => manager_url = Some(value.trim().to_string()),
                _ => {}
            }
        }
    }

    Some(AgentConfig {
        vm_id: vm_id?,
        manager_url: manager_url?,
    })
}

/// Detect the VM's IP address from eth0
fn detect_ip() -> Option<String> {
    // Try reading from /sys/class/net/eth0/address first
    let output = std::process::Command::new("ip")
        .args(["addr", "show", "eth0"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse: "inet 192.168.18.2/24 ..."
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("inet ") {
            if let Some(ip_part) = line.split_whitespace().nth(1) {
                if let Some(ip) = ip_part.split('/').next() {
                    // Skip localhost
                    if ip != "127.0.0.1" && !ip.is_empty() {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Report IP address to the manager
async fn report_ip_to_manager(
    config: &AgentConfig,
    ip: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/v1/vms/{}/guest-ip", config.manager_url, config.vm_id);

    // Create JSON payload as a string to ensure proper formatting
    let payload = format!(r#"{{"guest_ip":"{}"}}"#, ip);

    eprintln!("Reporting to: {}", url);
    eprintln!("Payload: {}", payload);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;

    if response.status().is_success() {
        eprintln!("✅ Successfully reported IP {} to manager", ip);
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to report IP: {} - {}", status, body).into())
    }
}

/// Get current metrics
fn get_current_metrics(prev_cpu: Option<CpuStats>) -> (GuestMetrics, Option<CpuStats>) {
    let cpu_stats = read_cpu_stats().unwrap_or((0, 0, 0, 0, 0, 0, 0));
    let cpu_percent = if let Some(prev) = prev_cpu {
        calculate_cpu_percent(prev, cpu_stats)
    } else {
        0.0 // Need two samples to calculate percentage
    };

    let (total_kb, used_kb, usage_percent) = read_memory_stats().unwrap_or((0, 0, 0));
    let uptime = read_uptime().unwrap_or(0);
    let load_avg = read_load_average();
    let process_count = count_processes();

    let metrics = GuestMetrics {
        cpu_usage_percent: cpu_percent,
        memory_usage_percent: usage_percent as f64,
        memory_used_kb: used_kb,
        memory_total_kb: total_kb,
        memory_available_kb: total_kb.saturating_sub(used_kb),
        uptime_seconds: uptime,
        load_average: load_avg,
        process_count,
    };

    (metrics, Some(cpu_stats))
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Metrics endpoint
async fn get_metrics(State(cpu_state): State<Arc<CpuState>>) -> Json<GuestMetrics> {
    let prev_cpu = cpu_state.last_cpu.load(Ordering::Relaxed);
    let prev_cpu_tuple = if prev_cpu == 0 {
        None
    } else {
        // Convert stored u128 back to tuple
        let user = (prev_cpu >> 48) & 0xFFFF;
        let nice = (prev_cpu >> 32) & 0xFFFF;
        let system = (prev_cpu >> 16) & 0xFFFF;
        let idle = prev_cpu & 0xFFFF;
        Some((user, nice, system, idle, 0, 0, 0))
    };

    let (metrics, new_cpu) = get_current_metrics(prev_cpu_tuple);

    // Store new CPU stats (compressed as u64 to save space)
    if let Some(cpu) = new_cpu {
        let compressed = ((cpu.0 & 0xFFFF) << 48)
            | ((cpu.1 & 0xFFFF) << 32)
            | ((cpu.2 & 0xFFFF) << 16)
            | (cpu.3 & 0xFFFF);
        cpu_state.last_cpu.store(compressed, Ordering::Relaxed);
    }

    Json(metrics)
}

/// Request to configure a network interface
#[derive(Deserialize)]
struct ConfigureInterfaceRequest {
    interface: String,
    /// Optional static IP with CIDR (e.g., "10.9.0.5/24")
    /// If not provided, will try DHCP
    static_ip: Option<String>,
    /// Optional gateway IP (e.g., "10.9.0.1")
    gateway: Option<String>,
}

/// Configure network interface endpoint
/// Brings up the interface and starts DHCP client
async fn configure_interface(
    Json(req): Json<ConfigureInterfaceRequest>,
) -> Json<serde_json::Value> {
    eprintln!("Configuring network interface: {}", req.interface);

    // Step 1: Bring up the interface
    let link_up = std::process::Command::new("ip")
        .args(["link", "set", &req.interface, "up"])
        .output();

    match link_up {
        Ok(output) if output.status.success() => {
            eprintln!("✅ Interface {} is now UP", req.interface);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("❌ Failed to bring up {}: {}", req.interface, stderr);
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to bring up interface: {}", stderr)
            }));
        }
        Err(e) => {
            eprintln!("❌ Failed to execute ip command: {}", e);
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to execute ip command: {}", e)
            }));
        }
    }

    // Step 2: Configure IP address (static or DHCP)
    if let Some(static_ip) = &req.static_ip {
        // Static IP configuration
        eprintln!("Configuring static IP {} on {}", static_ip, req.interface);

        let ip_result = std::process::Command::new("ip")
            .args(["addr", "add", static_ip, "dev", &req.interface])
            .output();

        match ip_result {
            Ok(output) if output.status.success() => {
                eprintln!("✅ Static IP {} configured on {}", static_ip, req.interface);

                // Configure gateway if provided
                if let Some(gateway) = &req.gateway {
                    eprintln!("Adding gateway {} via {}", gateway, req.interface);
                    let route_result = std::process::Command::new("ip")
                        .args([
                            "route",
                            "add",
                            "default",
                            "via",
                            gateway,
                            "dev",
                            &req.interface,
                        ])
                        .output();

                    if let Ok(output) = route_result {
                        if output.status.success() {
                            eprintln!("✅ Gateway configured: {}", gateway);
                        } else {
                            eprintln!("⚠️  Gateway configuration may have failed (route might already exist)");
                        }
                    }
                }

                Json(serde_json::json!({
                    "success": true,
                    "interface": req.interface,
                    "mode": "static",
                    "ip": static_ip,
                    "gateway": req.gateway
                }))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("❌ Failed to configure static IP: {}", stderr);
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to configure static IP: {}", stderr)
                }))
            }
            Err(e) => {
                eprintln!("❌ Failed to execute ip addr command: {}", e);
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to execute ip addr command: {}", e)
                }))
            }
        }
    } else {
        // DHCP configuration (background mode, don't wait for response)
        let dhcp_result = std::process::Command::new("udhcpc")
            .args(["-i", &req.interface, "-b", "-q", "-n", "-t", "3"])
            .spawn();

        match dhcp_result {
            Ok(_) => {
                eprintln!(
                    "✅ DHCP client started on {} using udhcpc (background)",
                    req.interface
                );
                Json(serde_json::json!({
                    "success": true,
                    "interface": req.interface,
                    "mode": "dhcp",
                    "dhcp_client": "udhcpc"
                }))
            }
            Err(_) => {
                // Try dhclient as fallback
                eprintln!("udhcpc not available, trying dhclient...");
                let dhclient_result = std::process::Command::new("dhclient")
                    .arg(&req.interface)
                    .spawn();

                match dhclient_result {
                    Ok(_) => {
                        eprintln!(
                            "✅ DHCP client started on {} using dhclient (background)",
                            req.interface
                        );
                        Json(serde_json::json!({
                            "success": true,
                            "interface": req.interface,
                            "mode": "dhcp",
                            "dhcp_client": "dhclient"
                        }))
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to start DHCP client: {}", e);
                        Json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to start DHCP client: {}", e)
                        }))
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct CpuState {
    last_cpu: Arc<AtomicU64>,
}

#[tokio::main]
async fn main() {
    // Initialize logging to stderr (works everywhere)
    eprintln!("Guest agent v{} starting...", env!("CARGO_PKG_VERSION"));

    // Read configuration
    let config = read_config();
    if let Some(ref cfg) = config {
        eprintln!(
            "Loaded config: VM ID = {}, Manager URL = {}",
            cfg.vm_id, cfg.manager_url
        );
    } else {
        eprintln!("Warning: No config found at /etc/guest-agent.conf - IP reporting disabled");
    }

    let cpu_state = Arc::new(CpuState {
        last_cpu: Arc::new(AtomicU64::new(0)),
    });

    // Sample CPU every second in background
    let cpu_state_clone = cpu_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if let Ok(cpu_stats) = read_cpu_stats() {
                let compressed = ((cpu_stats.0 & 0xFFFF) << 48)
                    | ((cpu_stats.1 & 0xFFFF) << 32)
                    | ((cpu_stats.2 & 0xFFFF) << 16)
                    | (cpu_stats.3 & 0xFFFF);
                cpu_state_clone
                    .last_cpu
                    .store(compressed, Ordering::Relaxed);
            }
        }
    });

    // Start IP reporting task if config is available
    if let Some(config) = config {
        tokio::spawn(async move {
            // Wait a bit for network to be ready
            tokio::time::sleep(Duration::from_secs(3)).await;

            let mut reported = false;

            loop {
                if let Some(ip) = detect_ip() {
                    match report_ip_to_manager(&config, &ip).await {
                        Ok(_) => {
                            if !reported {
                                eprintln!("Initial IP report successful");
                                reported = true;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to report IP: {}", e);
                        }
                    }
                } else {
                    eprintln!("Could not detect IP address from eth0");
                }

                // Use shorter interval until first successful report, then every 30s
                if reported {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                } else {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
    }

    // Create router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .route("/configure-interface", post(configure_interface))
        .with_state(cpu_state);

    // Try to bind to port 9000 (avoid conflict with manager on 8080)
    let addr = "0.0.0.0:9000";
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            eprintln!("Guest agent listening on {}", addr);
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("Guest agent server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    }
}
