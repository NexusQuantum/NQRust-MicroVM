use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
struct AgentConfig {
    vm_id: String,
    manager_url: String,
}

#[derive(Debug, Deserialize)]
struct UpdateConfigReq {
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
    usage_percent.min(100.0).max(0.0)
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
fn get_current_metrics(
    prev_cpu: Option<(u64, u64, u64, u64, u64, u64, u64)>,
) -> (GuestMetrics, Option<(u64, u64, u64, u64, u64, u64, u64)>) {
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
        process_count: process_count,
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

/// Update configuration endpoint
async fn update_config(
    State((_, config_state)): State<(Arc<CpuState>, Arc<ConfigState>)>,
    Json(req): Json<UpdateConfigReq>,
) -> Json<serde_json::Value> {
    // Update the stored configuration
    let mut config = config_state.config.lock().unwrap();
    config.vm_id = req.vm_id.clone();
    config.manager_url = req.manager_url.clone();
    drop(config);

    eprintln!("✅ Updated guest agent config: VM ID = {}, Manager URL = {}", req.vm_id, req.manager_url);

    Json(serde_json::json!({
        "status": "updated",
        "vm_id": req.vm_id,
        "manager_url": req.manager_url
    }))
}

/// Clean network state for golden snapshot creation
/// This ensures restored VMs get fresh DHCP leases instead of reusing old IPs
async fn clean_network_state() -> Json<serde_json::Value> {
    eprintln!("🧹 Cleaning network state for snapshot...");

    let mut success = true;
    let mut messages = Vec::new();

    // 1. Stop networking service (Alpine Linux OpenRC)
    eprintln!("  → Stopping networking service...");
    match std::process::Command::new("rc-service")
        .args(["networking", "stop"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                eprintln!("    ✅ Networking service stopped");
                messages.push("Stopped networking service");
            } else {
                eprintln!("    ⚠️  Failed to stop networking: {}", String::from_utf8_lossy(&output.stderr));
                messages.push("Failed to stop networking service");
                success = false;
            }
        }
        Err(e) => {
            eprintln!("    ❌ Error stopping networking: {}", e);
            messages.push("Error stopping networking service");
            success = false;
        }
    }

    // 2. Kill DHCP client processes
    eprintln!("  → Killing DHCP clients...");
    let _ = std::process::Command::new("pkill")
        .arg("udhcpc")
        .output();
    messages.push("Killed DHCP clients");

    // 3. Flush IP addresses from eth0
    eprintln!("  → Flushing IP addresses...");
    match std::process::Command::new("ip")
        .args(["addr", "flush", "dev", "eth0"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                eprintln!("    ✅ IP addresses flushed");
                messages.push("Flushed IP addresses");
            } else {
                eprintln!("    ⚠️  Failed to flush IPs: {}", String::from_utf8_lossy(&output.stderr));
                messages.push("Failed to flush IP addresses");
            }
        }
        Err(e) => {
            eprintln!("    ❌ Error flushing IPs: {}", e);
            messages.push("Error flushing IP addresses");
        }
    }

    // 4. Remove DHCP lease files (common locations)
    eprintln!("  → Removing DHCP lease files...");
    let lease_paths = vec![
        "/var/lib/dhcp/udhcpc.leases",
        "/var/lib/dhcpc/udhcpc-eth0.lease",
        "/var/run/udhcpc.eth0.pid",
    ];

    for path in lease_paths {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("    ⚠️  Failed to remove {}: {}", path, e);
            }
        } else {
            eprintln!("    ✅ Removed {}", path);
        }
    }
    messages.push("Cleaned DHCP lease files");

    if success {
        eprintln!("✅ Network state cleaned successfully - ready for snapshot");
    } else {
        eprintln!("⚠️  Network state partially cleaned - some operations failed");
    }

    Json(serde_json::json!({
        "status": if success { "success" } else { "partial" },
        "message": "Network state cleaned for snapshot",
        "operations": messages,
    }))
}

/// Restart networking to get fresh DHCP lease (for snapshot restore)
async fn restart_network() -> Json<serde_json::Value> {
    eprintln!("🔄 Restarting networking for fresh DHCP lease...");

    let mut success = true;
    let mut messages = Vec::new();

    // 1. Kill existing DHCP clients
    eprintln!("  → Killing DHCP clients...");
    let _ = std::process::Command::new("pkill")
        .arg("udhcpc")
        .output();
    messages.push("Killed DHCP clients");

    // 2. Restart networking service (Alpine Linux OpenRC)
    eprintln!("  → Restarting networking service...");
    match std::process::Command::new("rc-service")
        .args(["networking", "restart"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                eprintln!("    ✅ Networking service restarted");
                messages.push("Restarted networking service");
            } else {
                eprintln!("    ⚠️  Failed to restart networking: {}", String::from_utf8_lossy(&output.stderr));
                messages.push("Failed to restart networking service");
                success = false;
            }
        }
        Err(e) => {
            eprintln!("    ❌ Error restarting networking: {}", e);
            messages.push("Error restarting networking service");
            success = false;
        }
    }

    if success {
        eprintln!("✅ Networking restarted successfully");
    } else {
        eprintln!("⚠️  Networking restart had errors");
    }

    Json(serde_json::json!({
        "status": if success { "success" } else { "partial" },
        "message": "Networking restarted",
        "operations": messages,
    }))
}

/// Metrics endpoint
async fn get_metrics(State((cpu_state, _)): State<(Arc<CpuState>, Arc<ConfigState>)>) -> Json<GuestMetrics> {
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

#[derive(Clone)]
struct CpuState {
    last_cpu: Arc<AtomicU64>,
}

#[derive(Clone)]
struct ConfigState {
    config: Arc<std::sync::Mutex<AgentConfig>>,
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

    // Create shared config state
    let config_state = if let Some(initial_config) = config {
        Arc::new(ConfigState {
            config: Arc::new(std::sync::Mutex::new(initial_config)),
        })
    } else {
        // Create a dummy config state if no initial config
        Arc::new(ConfigState {
            config: Arc::new(std::sync::Mutex::new(AgentConfig {
                vm_id: "unknown".to_string(),
                manager_url: "http://127.0.0.1:8080".to_string(),
            })),
        })
    };

    // Start a blocking watchdog thread that works even if tokio runtime is broken after snapshot restore
    // This is critical because snapshot restore can leave async timers in inconsistent state
    let config_state_watchdog = config_state.clone();
    std::thread::spawn(move || {
        use std::io::Write;
        eprintln!("[Watchdog] Starting blocking network watchdog thread");
        let _ = std::io::stderr().flush();
        eprintln!("[Watchdog] Waiting 10 seconds before starting checks...");
        let _ = std::io::stderr().flush();

        // Short delay to let the VM boot minimally
        std::thread::sleep(std::time::Duration::from_secs(10));
        eprintln!("[Watchdog] Watchdog active - will restart networking only if truly broken");
        let _ = std::io::stderr().flush();

        let mut consecutive_failures = 0;
        let mut network_restarted_by_watchdog = false;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(3)); // Check every 3 seconds

            if network_restarted_by_watchdog {
                // Already fixed, stop checking
                break;
            }

            let config = config_state_watchdog.config.lock().unwrap().clone();

            if config.vm_id == "unknown" {
                continue;
            }

            // Try to reach manager (blocking HTTP request)
            let test_url = format!("{}/v1/vms/{}/guest-ip", config.manager_url, config.vm_id);

            match std::process::Command::new("curl")
                .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--connect-timeout", "2", "--max-time", "3", "-X", "POST", "-H", "Content-Type: application/json", "-d", "{\"guest_ip\":\"0.0.0.0\"}", &test_url])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let status_code = String::from_utf8_lossy(&output.stdout);
                    // Only treat connection failures (000) as network issues
                    // 404 means manager is reachable but VM not registered (expected during golden creation)
                    // 2xx means everything works
                    if status_code == "000" {
                        consecutive_failures += 1;
                        eprintln!("[Watchdog] Network unreachable (status: 000, attempt {}/5)", consecutive_failures);

                        if consecutive_failures >= 5 {
                            eprintln!("[Watchdog] 🔍 Network appears broken - likely snapshot restore");
                            eprintln!("[Watchdog] 🔄 Restarting networking...");

                            // Kill DHCP client
                            let _ = std::process::Command::new("pkill").arg("udhcpc").output();

                            // Restart networking
                            match std::process::Command::new("rc-service")
                                .args(["networking", "restart"])
                                .output()
                            {
                                Ok(output) if output.status.success() => {
                                    eprintln!("[Watchdog] ✅ Networking restarted successfully!");
                                    network_restarted_by_watchdog = true;
                                    consecutive_failures = 0;
                                }
                                _ => {
                                    eprintln!("[Watchdog] ⚠️  Network restart failed, will retry");
                                }
                            }
                        }
                    } else {
                        // Success - manager is reachable (200, 404, 400, whatever)
                        // 404 is fine - means VM not registered yet (golden creation)
                        // 400 is fine - means request malformed but network works
                        // 200 is perfect - everything works
                        if consecutive_failures > 0 {
                            eprintln!("[Watchdog] Manager reachable (status: {}), network OK", status_code);
                        }
                        consecutive_failures = 0;
                    }
                }
                _ => {
                    consecutive_failures += 1;
                    eprintln!("[Watchdog] Failed to reach manager (curl error, attempt {}/5)", consecutive_failures);

                    if consecutive_failures >= 5 && !network_restarted_by_watchdog {
                        eprintln!("[Watchdog] 🔍 Network appears broken - likely snapshot restore");
                        eprintln!("[Watchdog] 🔄 Restarting networking...");

                        let _ = std::process::Command::new("pkill").arg("udhcpc").output();

                        match std::process::Command::new("rc-service")
                            .args(["networking", "restart"])
                            .output()
                        {
                            Ok(output) if output.status.success() => {
                                eprintln!("[Watchdog] ✅ Networking restarted successfully!");
                                network_restarted_by_watchdog = true;
                                consecutive_failures = 0;
                                std::thread::sleep(std::time::Duration::from_secs(5)); // Wait for DHCP
                            }
                            _ => {
                                eprintln!("[Watchdog] ⚠️  Network restart failed, will retry");
                            }
                        }
                    }
                }
            }
        }
    });

    // Start IP reporting task with automatic network restart detection for snapshot restores
    let config_state_clone = config_state.clone();
    tokio::spawn(async move {
        // Wait a bit for network to be ready
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Detect snapshot restore by checking if our configured VM ID is valid with the manager
        // If the manager returns 404 for our VM ID, we know we're a restored snapshot with wrong ID
        let config = config_state_clone.config.lock().unwrap().clone();

        if config.vm_id != "unknown" {
            eprintln!("Checking if VM ID {} is valid with manager...", config.vm_id);

            // Try to report IP to manager - if we get 404, we're a restored snapshot
            let test_url = format!("{}/v1/vms/{}/guest-ip", config.manager_url, config.vm_id);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build();

            if let Ok(client) = client {
                let payload = r#"{"guest_ip":"0.0.0.0"}"#;  // Dummy IP for test
                match client.post(&test_url)
                    .header("Content-Type", "application/json")
                    .body(payload)
                    .send()
                    .await
                {
                    Ok(response) if response.status() == 404 => {
                        eprintln!("🔍 Detected snapshot restore: VM ID {} not found in manager (404)", config.vm_id);
                        eprintln!("This means we're a restored VM with the golden snapshot's VM ID");

                        if let Some(ip) = detect_ip() {
                            if !ip.is_empty() && ip != "127.0.0.1" {
                                eprintln!("Current IP: {} (from snapshot)", ip);
                                eprintln!("🔄 Restarting networking to get fresh DHCP lease...");

                                // Kill existing DHCP clients
                                let _ = std::process::Command::new("pkill").arg("udhcpc").output();

                                // Restart networking
                                match std::process::Command::new("rc-service")
                                    .args(["networking", "restart"])
                                    .output()
                                {
                                    Ok(output) => {
                                        if output.status.success() {
                                            eprintln!("✅ Networking restarted - waiting for fresh DHCP lease...");
                                            tokio::time::sleep(Duration::from_secs(5)).await;

                                            // Check new IP
                                            if let Some(new_ip) = detect_ip() {
                                                eprintln!("✅ New IP obtained: {}", new_ip);
                                            }
                                        } else {
                                            eprintln!("⚠️  Failed to restart: {}", String::from_utf8_lossy(&output.stderr));
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Error restarting networking: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Ok(response) => {
                        eprintln!("VM ID is valid (HTTP {}), this is the original golden VM or first boot", response.status());
                    }
                    Err(e) => {
                        eprintln!("Could not reach manager to check VM ID: {} (will skip network restart)", e);
                    }
                }
            }
        }

        // Use a short interval (5s) so snapshot-restored VMs detect issues quickly
        // When a VM is restored from snapshot, it resumes mid-loop, potentially mid-tick
        // A shorter interval ensures faster detection and recovery
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut reported = false;
        let mut network_restarted = false;
        let mut failed_reports = 0;

        // First tick happens immediately (important for initial startup)
        interval.tick().await;

        loop {
            interval.tick().await;

            // Get current config
            let config = config_state_clone.config.lock().unwrap().clone();

            if config.vm_id == "unknown" {
                eprintln!("Skipping IP report - VM ID not set");
                continue;
            }

            if let Some(ip) = detect_ip() {
                // Try to report IP to manager
                let report_success = match report_ip_to_manager(&config, &ip).await {
                    Ok(_) => {
                        if !reported {
                            eprintln!("Initial IP report successful for VM {}", config.vm_id);
                            reported = true;
                        }
                        true
                    }
                    Err(e) => {
                        eprintln!("Failed to report IP for VM {} (attempt {}/3): {}",
                                 config.vm_id, failed_reports + 1, e);
                        false
                    }
                };

                if report_success {
                    failed_reports = 0;
                } else {
                    failed_reports += 1;
                }

                // If we fail to reach the manager 3 times in a row, restart networking
                // This handles snapshot restore where we have the wrong IP from the snapshot
                if failed_reports >= 3 && !network_restarted {
                    eprintln!("🔍 Failed to reach manager 3 times - likely snapshot restore");
                    eprintln!("   Current IP: {}", ip);
                    eprintln!("   VM ID: {}", config.vm_id);
                    eprintln!("🔄 Restarting networking to get fresh DHCP lease...");

                    // Kill any existing DHCP client
                    let _ = std::process::Command::new("pkill").arg("udhcpc").output();

                    // Restart networking service
                    match std::process::Command::new("rc-service")
                        .args(["networking", "restart"])
                        .output()
                    {
                        Ok(output) if output.status.success() => {
                            eprintln!("✅ Networking restarted successfully!");
                            network_restarted = true;
                            failed_reports = 0;

                            // Wait for new IP
                            tokio::time::sleep(Duration::from_secs(5)).await;

                            if let Some(new_ip) = detect_ip() {
                                eprintln!("✅ New IP obtained: {}", new_ip);

                                // Try reporting with new IP immediately
                                match report_ip_to_manager(&config, &new_ip).await {
                                    Err(e2) => {
                                        eprintln!("⚠️  Still can't reach manager after network restart: {}", e2);
                                    }
                                    Ok(_) => {
                                        eprintln!("✅ Successfully reported new IP to manager!");
                                        reported = true;
                                    }
                                }
                            }
                        }
                        Ok(output) => {
                            eprintln!("⚠️  Network restart failed: {}",
                                     String::from_utf8_lossy(&output.stderr));
                            // Don't mark as restarted so we can try again
                        }
                        Err(e2) => {
                            eprintln!("❌ Error restarting networking: {}", e2);
                        }
                    }
                }
            } else {
                eprintln!("Could not detect IP address from eth0");
            }
        }
    });

    // Create router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .route("/update-config", post(update_config))
        .route("/clean-network", post(clean_network_state))
        .route("/restart-network", post(restart_network))
        .with_state((cpu_state, config_state));

    // Try to bind to port 9000 (avoid conflict with manager on 8080)
    let addr = "0.0.0.0:9000";
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            eprintln!("Guest agent listening on {}", addr);
            axum::serve(listener, app).await.unwrap();
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    }
}
