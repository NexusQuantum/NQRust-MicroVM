Run cargo fmt -- --check
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/core/net.rs:156:
                     .await?;
             }
 
-            eprintln!("Created VLAN interface {} with bridge {} for VLAN {}", vlan_if, vlan_br, vlan);
+            eprintln!(
+                "Created VLAN interface {} with bridge {} for VLAN {}",
+                vlan_if, vlan_br, vlan
+            );
         }
 
         // Attach TAP to VLAN bridge instead of main bridge
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/core/systemd.rs:8:
 
 /// Spawn firecracker inside a screen session for console access
 /// The screen session name will be the same as the unit name (e.g., "fc-{vm-id}")
-pub async fn spawn_fc_scope_with_screen(unit: &str, sock: &str, screen_name: Option<&str>) -> Result<()> {
+pub async fn spawn_fc_scope_with_screen(
+    unit: &str,
+    sock: &str,
+    screen_name: Option<&str>,
+) -> Result<()> {
     // Ensure parent dir exists is done by caller.
     let session_name = screen_name.unwrap_or(unit);
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/core/systemd.rs:26:
             "TimeoutStopSec=5s",
             "--",
             "screen",
-            "-dmS",  // Create detached session with name
+            "-dmS", // Create detached session with name
             session_name,
             "firecracker",
             "--api-sock",
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/core/systemd.rs:34:
         ])
         .status()
         .await?;
-    ensure!(status.success(), "systemd-run failed for firecracker with screen");
+    ensure!(
+        status.success(),
+        "systemd-run failed for firecracker with screen"
+    );
     Ok(())
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/features/tap/mod.rs:10:
     bridge: Option<String>,
     owner_user: Option<String>,
     vlan_id: Option<u16>,
-    tap_name: Option<String>,  // Allow custom TAP device name
+    tap_name: Option<String>, // Allow custom TAP device name
 }
 
 pub fn router() -> Router {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/features/vm/shell.rs:52:
     }))
 }
 
-pub async fn proxy_console_screen(screen_name: String, ws: WebSocket) -> Result<(), (StatusCode, String)> {
+pub async fn proxy_console_screen(
+    screen_name: String,
+    ws: WebSocket,
+) -> Result<(), (StatusCode, String)> {
     // Spawn screen -x to attach to the session
     // We use 'script' to allocate a PTY because screen requires a terminal
     // script -qfc "command" /dev/null runs command with a PTY and outputs to stdout
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/features/vm/shell.rs:61:
             "script",
             "-qfc",
             &format!("screen -x {}", screen_name),
-            "/dev/null"
+            "/dev/null",
         ])
         .stdin(std::process::Stdio::piped())
         .stdout(std::process::Stdio::piped())
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/agent/src/features/vm/shell.rs:68:
         .stderr(std::process::Stdio::null())
         .spawn()
-        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to spawn screen: {}", err)))?;
+        .map_err(|err| {
+            (
+                StatusCode::INTERNAL_SERVER_ERROR,
+                format!("Failed to spawn screen: {}", err),
+            )
+        })?;
 
     let mut stdin = child.stdin.take().ok_or((
         StatusCode::INTERNAL_SERVER_ERROR,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:1:
-use axum::{routing::{get, post}, Json, Router, extract::State};
+use axum::{
+    extract::State,
+    routing::{get, post},
+    Json, Router,
+};
 use serde::{Deserialize, Serialize};
 use std::fs;
 use std::sync::atomic::{AtomicU64, Ordering};
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:51:
 }
 
 /// Calculate CPU usage percentage between two samples
-fn calculate_cpu_percent(
-    prev: CpuStats,
-    curr: CpuStats,
-) -> f64 {
+fn calculate_cpu_percent(prev: CpuStats, curr: CpuStats) -> f64 {
     let (prev_user, prev_nice, prev_system, prev_idle, prev_iowait, prev_irq, prev_softirq) = prev;
     let (curr_user, curr_nice, curr_system, curr_idle, curr_iowait, curr_irq, curr_softirq) = curr;
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:61:
     let prev_idle_total = prev_idle + prev_iowait;
     let curr_idle_total = curr_idle + curr_iowait;
 
-    let prev_total = prev_user + prev_nice + prev_system + prev_idle + prev_iowait + prev_irq + prev_softirq;
-    let curr_total = curr_user + curr_nice + curr_system + curr_idle + curr_iowait + curr_irq + curr_softirq;
+    let prev_total =
+        prev_user + prev_nice + prev_system + prev_idle + prev_iowait + prev_irq + prev_softirq;
+    let curr_total =
+        curr_user + curr_nice + curr_system + curr_idle + curr_iowait + curr_irq + curr_softirq;
 
     let total_diff = curr_total.saturating_sub(prev_total);
     let idle_diff = curr_idle_total.saturating_sub(prev_idle_total);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:79:
 /// Works across all Linux distributions
 fn read_memory_stats() -> Result<(u64, u64, u64), String> {
     let meminfo = fs::read_to_string("/proc/meminfo").map_err(|e| e.to_string())?;
-    
+
     let mut total_kb = 0u64;
     let mut available_kb = 0u64;
     let mut free_kb = 0u64;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:86:
     let mut buffers_kb = 0u64;
     let mut cached_kb = 0u64;
-    
+
     for line in meminfo.lines() {
         if line.starts_with("MemTotal:") {
             if let Some(kb_str) = line.split_whitespace().nth(1) {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:109:
             }
         }
     }
-    
+
     if total_kb == 0 {
         return Err("Could not parse memory total".to_string());
     }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:116:
-    
+
     // If MemAvailable is not available (older kernels), calculate it
     if available_kb == 0 {
         available_kb = free_kb + buffers_kb + cached_kb;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:120:
     }
-    
+
     let used_kb = total_kb.saturating_sub(available_kb);
     let usage_percent = if total_kb > 0 {
         (used_kb as f64 / total_kb as f64) * 100.0
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:125:
     } else {
         0.0
     };
-    
+
     Ok((total_kb, used_kb, usage_percent as u64))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:215:
 }
 
 /// Report IP address to the manager
-async fn report_ip_to_manager(config: &AgentConfig, ip: &str) -> Result<(), Box<dyn std::error::Error>> {
+async fn report_ip_to_manager(
+    config: &AgentConfig,
+    ip: &str,
+) -> Result<(), Box<dyn std::error::Error>> {
     let url = format!("{}/v1/vms/{}/guest-ip", config.manager_url, config.vm_id);
 
     // Create JSON payload as a string to ensure proper formatting
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:253:
     } else {
         0.0 // Need two samples to calculate percentage
     };
-    
+
     let (total_kb, used_kb, usage_percent) = read_memory_stats().unwrap_or((0, 0, 0));
     let uptime = read_uptime().unwrap_or(0);
     let load_avg = read_load_average();
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:260:
     let process_count = count_processes();
-    
+
     let metrics = GuestMetrics {
         cpu_usage_percent: cpu_percent,
         memory_usage_percent: usage_percent as f64,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:269:
         load_average: load_avg,
         process_count,
     };
-    
+
     (metrics, Some(cpu_stats))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:286:
 }
 
 /// Metrics endpoint
-async fn get_metrics(
-    State(cpu_state): State<Arc<CpuState>>,
-) -> Json<GuestMetrics> {
+async fn get_metrics(State(cpu_state): State<Arc<CpuState>>) -> Json<GuestMetrics> {
     let prev_cpu = cpu_state.last_cpu.load(Ordering::Relaxed);
     let prev_cpu_tuple = if prev_cpu == 0 {
         None
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:300:
         let idle = prev_cpu & 0xFFFF;
         Some((user, nice, system, idle, 0, 0, 0))
     };
-    
+
     let (metrics, new_cpu) = get_current_metrics(prev_cpu_tuple);
-    
+
     // Store new CPU stats (compressed as u64 to save space)
     if let Some(cpu) = new_cpu {
-        let compressed = ((cpu.0 & 0xFFFF) << 48) |
-                        ((cpu.1 & 0xFFFF) << 32) |
-                        ((cpu.2 & 0xFFFF) << 16) |
-                        (cpu.3 & 0xFFFF);
+        let compressed = ((cpu.0 & 0xFFFF) << 48)
+            | ((cpu.1 & 0xFFFF) << 32)
+            | ((cpu.2 & 0xFFFF) << 16)
+            | (cpu.3 & 0xFFFF);
         cpu_state.last_cpu.store(compressed, Ordering::Relaxed);
     }
-    
+
     Json(metrics)
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:376:
                 if let Some(gateway) = &req.gateway {
                     eprintln!("Adding gateway {} via {}", gateway, req.interface);
                     let route_result = std::process::Command::new("ip")
-                        .args(["route", "add", "default", "via", gateway, "dev", &req.interface])
+                        .args([
+                            "route",
+                            "add",
+                            "default",
+                            "via",
+                            gateway,
+                            "dev",
+                            &req.interface,
+                        ])
                         .output();
 
                     if let Ok(output) = route_result {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:420:
 
         match dhcp_result {
             Ok(_) => {
-                eprintln!("✅ DHCP client started on {} using udhcpc (background)", req.interface);
+                eprintln!(
+                    "✅ DHCP client started on {} using udhcpc (background)",
+                    req.interface
+                );
                 Json(serde_json::json!({
                     "success": true,
                     "interface": req.interface,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:437:
 
                 match dhclient_result {
                     Ok(_) => {
-                        eprintln!("✅ DHCP client started on {} using dhclient (background)", req.interface);
+                        eprintln!(
+                            "✅ DHCP client started on {} using dhclient (background)",
+                            req.interface
+                        );
                         Json(serde_json::json!({
                             "success": true,
                             "interface": req.interface,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:471:
     // Read configuration
     let config = read_config();
     if let Some(ref cfg) = config {
-        eprintln!("Loaded config: VM ID = {}, Manager URL = {}", cfg.vm_id, cfg.manager_url);
+        eprintln!(
+            "Loaded config: VM ID = {}, Manager URL = {}",
+            cfg.vm_id, cfg.manager_url
+        );
     } else {
         eprintln!("Warning: No config found at /etc/guest-agent.conf - IP reporting disabled");
     }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:487:
         loop {
             interval.tick().await;
             if let Ok(cpu_stats) = read_cpu_stats() {
-                let compressed = ((cpu_stats.0 & 0xFFFF) << 48) |
-                                ((cpu_stats.1 & 0xFFFF) << 32) |
-                                ((cpu_stats.2 & 0xFFFF) << 16) |
-                                (cpu_stats.3 & 0xFFFF);
-                cpu_state_clone.last_cpu.store(compressed, Ordering::Relaxed);
+                let compressed = ((cpu_stats.0 & 0xFFFF) << 48)
+                    | ((cpu_stats.1 & 0xFFFF) << 32)
+                    | ((cpu_stats.2 & 0xFFFF) << 16)
+                    | (cpu_stats.3 & 0xFFFF);
+                cpu_state_clone
+                    .last_cpu
+                    .store(compressed, Ordering::Relaxed);
             }
         }
     });
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/guest-agent/src/main.rs:547:
         }
     }
 }
+
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:88:
             .context("Failed to send create container request")?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to create container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:106:
 
         if !resp.status().is_success() && resp.status().as_u16() != 304 {
             // 304 = already started
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to start container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:127:
 
         if !resp.status().is_success() && resp.status().as_u16() != 304 {
             // 304 = already stopped
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to stop container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:147:
         let resp = self.client.post(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to restart container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:163:
         let resp = self.client.post(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to pause container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:179:
         let resp = self.client.post(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to unpause container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:198:
         let resp = self.client.delete(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to remove container: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:207:
 
     /// Get container stats
     pub async fn get_stats(&self, container_id: &str) -> Result<DockerStats> {
-        let url = format!("{}/containers/{}/stats?stream=false", self.base_url, container_id);
+        let url = format!(
+            "{}/containers/{}/stats?stream=false",
+            self.base_url, container_id
+        );
 
         tracing::debug!(container_id = %container_id, "Getting container stats");
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:214:
         let resp = self.client.get(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to get stats: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:270:
         let resp = self.client.get(&url).send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to get logs: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:312:
             "AttachStderr": true,
         });
 
-        let create_resp = self.client.post(&create_url).json(&create_config).send().await?;
+        let create_resp = self
+            .client
+            .post(&create_url)
+            .json(&create_config)
+            .send()
+            .await?;
 
         if !create_resp.status().is_success() {
-            let error_text = create_resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = create_resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to create exec: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:331:
             "Detach": false,
         });
 
-        let start_resp = self.client.post(&start_url).json(&start_config).send().await?;
+        let start_resp = self
+            .client
+            .post(&start_url)
+            .json(&start_config)
+            .send()
+            .await?;
 
         let output = if start_resp.status().is_success() {
             start_resp.text().await.ok()
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:347:
     }
 
     /// Pull an image from a registry with optional authentication
-    pub async fn pull_image(&self, image: &str, registry_auth: Option<&nexus_types::RegistryAuth>) -> Result<()> {
+    pub async fn pull_image(
+        &self,
+        image: &str,
+        registry_auth: Option<&nexus_types::RegistryAuth>,
+    ) -> Result<()> {
         let url = format!("{}/images/create?fromImage={}", self.base_url, image);
 
         tracing::info!(image = %image, has_auth = registry_auth.is_some(), "Pulling image");
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:364:
             });
 
             let auth_json = serde_json::to_string(&auth_config)?;
-            let auth_base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, auth_json.as_bytes());
+            let auth_base64 = base64::Engine::encode(
+                &base64::engine::general_purpose::STANDARD,
+                auth_json.as_bytes(),
+            );
 
             request = request.header("X-Registry-Auth", auth_base64);
             tracing::debug!("Added registry authentication header");
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:373:
         let resp = request.send().await?;
 
         if !resp.status().is_success() {
-            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+            let error_text = resp
+                .text()
+                .await
+                .unwrap_or_else(|_| "Unknown error".to_string());
             anyhow::bail!("Failed to pull image: {}", error_text);
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/docker.rs:486:
     let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
         - stats.precpu_stats.cpu_usage.total_usage as f64;
 
-    let system_delta = stats
-        .cpu_stats
-        .system_cpu_usage
-        .unwrap_or(0) as f64
+    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
         - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
 
     if system_delta > 0.0 && cpu_delta > 0.0 {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/mod.rs:1:
-use axum::{routing::{get, post}, Router};
+use axum::{
+    routing::{get, post},
+    Router,
+};
 
 pub mod docker;
 pub mod repo;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/mod.rs:9:
 pub fn router() -> Router {
     Router::new()
         .route("/", post(routes::create).get(routes::list))
-        .route("/:id", get(routes::get).put(routes::update).delete(routes::delete))
+        .route(
+            "/:id",
+            get(routes::get).put(routes::update).delete(routes::delete),
+        )
         .route("/:id/start", post(routes::start))
         .route("/:id/stop", post(routes::stop))
         .route("/:id/restart", post(routes::restart))
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:1:
 use anyhow::{Context, Result};
 use chrono::Utc;
-use nexus_types::{Container, ContainerLog, ContainerStats, CreateContainerReq, UpdateContainerReq};
+use nexus_types::{
+    Container, ContainerLog, ContainerStats, CreateContainerReq, UpdateContainerReq,
+};
 use sqlx::PgPool;
 use uuid::Uuid;
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:74:
         .await
         .context("container not found")?;
 
-        let args: Vec<String> = serde_json::from_value(row.args.unwrap_or_else(|| serde_json::json!([])))?;
+        let args: Vec<String> =
+            serde_json::from_value(row.args.unwrap_or_else(|| serde_json::json!([])))?;
         let env_vars: std::collections::HashMap<String, String> =
             serde_json::from_value(row.env_vars.unwrap_or_else(|| serde_json::json!({})))?;
         let volumes: Vec<nexus_types::VolumeMount> =
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:83:
             serde_json::from_value(row.port_mappings.unwrap_or_else(|| serde_json::json!([])))?;
 
         let uptime_seconds = if row.state == "running" {
-            row.started_at.map(|started| (Utc::now() - started).num_seconds())
+            row.started_at
+                .map(|started| (Utc::now() - started).num_seconds())
         } else {
             None
         };
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:116:
         })
     }
 
-    pub async fn list(&self, state_filter: Option<String>, host_filter: Option<Uuid>) -> Result<Vec<Container>> {
+    pub async fn list(
+        &self,
+        state_filter: Option<String>,
+        host_filter: Option<Uuid>,
+    ) -> Result<Vec<Container>> {
         let mut query_str = String::from(
             r#"
             SELECT
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:128:
             FROM containers c
             LEFT JOIN vm v ON c.container_runtime_id = 'vm-' || v.id::text
             WHERE 1=1
-            "#
+            "#,
         );
 
         if state_filter.is_some() {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:158:
             .into_iter()
             .map(|row| {
                 let uptime_seconds = if row.state == "running" {
-                    row.started_at.map(|started| (Utc::now() - started).num_seconds())
+                    row.started_at
+                        .map(|started| (Utc::now() - started).num_seconds())
                 } else {
                     None
                 };
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:168:
                     name: row.name,
                     image: row.image,
                     command: row.command,
-                    args: serde_json::from_value(row.args.unwrap_or_else(|| serde_json::json!([])))?,
-                    env_vars: serde_json::from_value(row.env_vars.unwrap_or_else(|| serde_json::json!({})))?,
-                    volumes: serde_json::from_value(row.volumes.unwrap_or_else(|| serde_json::json!([])))?,
-                    port_mappings: serde_json::from_value(row.port_mappings.unwrap_or_else(|| serde_json::json!([])))?,
+                    args: serde_json::from_value(
+                        row.args.unwrap_or_else(|| serde_json::json!([])),
+                    )?,
+                    env_vars: serde_json::from_value(
+                        row.env_vars.unwrap_or_else(|| serde_json::json!({})),
+                    )?,
+                    volumes: serde_json::from_value(
+                        row.volumes.unwrap_or_else(|| serde_json::json!([])),
+                    )?,
+                    port_mappings: serde_json::from_value(
+                        row.port_mappings.unwrap_or_else(|| serde_json::json!([])),
+                    )?,
                     cpu_limit: row.cpu_limit,
                     memory_limit_mb: row.memory_limit_mb,
                     restart_policy: row.restart_policy.unwrap_or_else(|| "no".to_string()),
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:260:
         Ok(())
     }
 
-    pub async fn update_state(&self, id: Uuid, state: &str, error_message: Option<String>) -> Result<()> {
+    pub async fn update_state(
+        &self,
+        id: Uuid,
+        state: &str,
+        error_message: Option<String>,
+    ) -> Result<()> {
         let now = Utc::now();
         sqlx::query!(
             r#"
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:348:
         Ok(())
     }
 
-    pub async fn get_latest_stats(&self, container_id: Uuid, limit: i64) -> Result<Vec<ContainerStats>> {
+    pub async fn get_latest_stats(
+        &self,
+        container_id: Uuid,
+        limit: i64,
+    ) -> Result<Vec<ContainerStats>> {
         let rows = sqlx::query_as!(
             ContainerStats,
             r#"
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:369:
         Ok(rows)
     }
 
-    pub async fn append_log(&self, container_id: Uuid, stream: &str, message: String) -> Result<()> {
+    pub async fn append_log(
+        &self,
+        container_id: Uuid,
+        stream: &str,
+        message: String,
+    ) -> Result<()> {
         sqlx::query!(
             r#"
             INSERT INTO container_logs (container_id, stream, message, timestamp)
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/repo.rs:385:
         Ok(())
     }
 
-    pub async fn get_logs(&self, container_id: Uuid, tail: Option<i64>) -> Result<Vec<ContainerLog>> {
+    pub async fn get_logs(
+        &self,
+        container_id: Uuid,
+        tail: Option<i64>,
+    ) -> Result<Vec<ContainerLog>> {
         let limit = tail.unwrap_or(100);
 
         let rows = sqlx::query_as!(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:1:
 use crate::AppState;
 use axum::{
-    extract::{Path, Query, ws::{WebSocket, WebSocketUpgrade}},
+    extract::{
+        ws::{WebSocket, WebSocketUpgrade},
+        Path, Query,
+    },
     http::StatusCode,
-    Extension, Json, response::IntoResponse,
+    response::IntoResponse,
+    Extension, Json,
 };
 use nexus_types::{
     ContainerLogsParams, ContainerLogsResp, ContainerPathParams, ContainerStatsResp,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:188:
     Extension(st): Extension<AppState>,
     Path(ContainerPathParams { id }): Path<ContainerPathParams>,
 ) -> Result<Json<OkResponse>, StatusCode> {
-    super::service::stop_container(&st, id)
-        .await
-        .map_err(|e| {
-            eprintln!("Failed to stop container: {}", e);
-            if e.to_string().contains("not running") {
-                StatusCode::BAD_REQUEST
-            } else {
-                StatusCode::INTERNAL_SERVER_ERROR
-            }
-        })?;
+    super::service::stop_container(&st, id).await.map_err(|e| {
+        eprintln!("Failed to stop container: {}", e);
+        if e.to_string().contains("not running") {
+            StatusCode::BAD_REQUEST
+        } else {
+            StatusCode::INTERNAL_SERVER_ERROR
+        }
+    })?;
     Ok(Json(OkResponse::default()))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:329:
         let container = match super::service::get_container(&st.db, container_id).await {
             Ok(resp) => resp.item,
             Err(e) => {
-                let _ = socket.send(axum::extract::ws::Message::Text(
-                    format!("{{\"error\": \"Failed to get container: {}\"}}", e)
-                )).await;
+                let _ = socket
+                    .send(axum::extract::ws::Message::Text(format!(
+                        "{{\"error\": \"Failed to get container: {}\"}}",
+                        e
+                    )))
+                    .await;
                 break;
             }
         };
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:338:
 
         // If container is in error or doesn't have a runtime ID, stop streaming
         if container.state == "error" || container.container_runtime_id.is_none() {
-            let _ = socket.send(axum::extract::ws::Message::Text(
-                format!("{{\"info\": \"Container in {} state\"}}", container.state)
-            )).await;
+            let _ = socket
+                .send(axum::extract::ws::Message::Text(format!(
+                    "{{\"info\": \"Container in {} state\"}}",
+                    container.state
+                )))
+                .await;
             break;
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:356:
                             "message": log.message
                         });
 
-                        if socket.send(axum::extract::ws::Message::Text(log_json.to_string())).await.is_err() {
+                        if socket
+                            .send(axum::extract::ws::Message::Text(log_json.to_string()))
+                            .await
+                            .is_err()
+                        {
                             // Client disconnected
                             return;
                         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:366:
                 }
             }
             Err(e) => {
-                let _ = socket.send(axum::extract::ws::Message::Text(
-                    format!("{{\"error\": \"Failed to fetch logs: {}\"}}", e)
-                )).await;
+                let _ = socket
+                    .send(axum::extract::ws::Message::Text(format!(
+                        "{{\"error\": \"Failed to fetch logs: {}\"}}",
+                        e
+                    )))
+                    .await;
             }
         }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/routes.rs:375:
         // Check if client is still connected by trying to send a ping
-        if socket.send(axum::extract::ws::Message::Ping(vec![])).await.is_err() {
+        if socket
+            .send(axum::extract::ws::Message::Ping(vec![]))
+            .await
+            .is_err()
+        {
             break;
         }
     }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:1:
 use anyhow::{anyhow, Result};
 use nexus_types::{
-    ContainerLogsResp, ContainerStatsResp, CreateContainerReq, CreateContainerResp,
-    ExecCommandReq, ExecCommandResp, GetContainerResp, ListContainersResp, OkResponse,
-    UpdateContainerReq,
+    ContainerLogsResp, ContainerStatsResp, CreateContainerReq, CreateContainerResp, ExecCommandReq,
+    ExecCommandResp, GetContainerResp, ListContainersResp, OkResponse, UpdateContainerReq,
 };
 use sqlx::PgPool;
 use uuid::Uuid;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:22:
 ///    d. Pull container image inside VM
 ///    e. Create and start Docker container inside VM
 ///    f. Update container state to running
-pub async fn create_container(st: &AppState, req: CreateContainerReq) -> Result<CreateContainerResp> {
+pub async fn create_container(
+    st: &AppState,
+    req: CreateContainerReq,
+) -> Result<CreateContainerResp> {
     let repo = ContainerRepository::new(st.db.clone());
 
     // Validate request
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:80:
     eprintln!("[Container {}] Starting VM provisioning", container_id);
 
     // Create dedicated microVM with Docker runtime
-    let vm_id = super::vm::create_container_vm(st, container_id, container_name, vcpu, memory_mb).await?;
+    let vm_id =
+        super::vm::create_container_vm(st, container_id, container_name, vcpu, memory_mb).await?;
 
     eprintln!("[Container {}] VM created: {}", container_id, vm_id);
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:124:
     eprintln!("[Container {}] Got guest IP: {}", container_id, guest_ip);
 
     // Wait for Docker daemon to be ready inside VM
-    repo.update_state(container_id, "initializing", None).await?;
+    repo.update_state(container_id, "initializing", None)
+        .await?;
 
     if let Err(e) = super::vm::wait_for_docker_ready(&guest_ip, 120).await {
         let error_msg = format!("Docker daemon not ready: {}", e);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:141:
     // Pull image if needed
     if !docker.image_exists(&req.image).await.unwrap_or(false) {
         eprintln!("[Container {}] Pulling image: {}", container_id, req.image);
-        if let Err(e) = docker.pull_image(&req.image, req.registry_auth.as_ref()).await {
+        if let Err(e) = docker
+            .pull_image(&req.image, req.registry_auth.as_ref())
+            .await
+        {
             let error_msg = format!("Failed to pull image: {}", e);
             repo.update_state(container_id, "error", Some(error_msg.clone()))
                 .await?;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:176:
     // Update container state to running
     repo.set_started(container_id).await?;
 
-    eprintln!("[Container {}] Container running successfully", container_id);
+    eprintln!(
+        "[Container {}] Container running successfully",
+        container_id
+    );
 
     Ok(())
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:304:
 
     let docker_container_id = extract_docker_container_id(&container)?;
 
-    docker.stop_container(&docker_container_id, Some(10)).await?;
+    docker
+        .stop_container(&docker_container_id, Some(10))
+        .await?;
     repo.set_stopped(id).await?;
 
     tracing::info!(container_id = %id, "Container stopped");
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:322:
     let docker = DockerClient::new(&guest_ip)?;
     let docker_container_id = extract_docker_container_id(&container)?;
 
-    docker.restart_container(&docker_container_id, Some(10)).await?;
+    docker
+        .restart_container(&docker_container_id, Some(10))
+        .await?;
     repo.set_started(id).await?;
 
     tracing::info!(container_id = %id, "Container restarted");
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:439:
 }
 
 /// Execute a command in a container
-pub async fn exec_command(
-    st: &AppState,
-    id: Uuid,
-    req: ExecCommandReq,
-) -> Result<ExecCommandResp> {
+pub async fn exec_command(st: &AppState, id: Uuid, req: ExecCommandReq) -> Result<ExecCommandResp> {
     let repo = ContainerRepository::new(st.db.clone());
 
     let container = repo.get(id).await?;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/service.rs:493:
     // Get VM and extract guest IP
     let vm = crate::features::vms::repo::get(db, vm_id).await?;
 
-    vm.guest_ip
-        .ok_or_else(|| anyhow!("VM has no guest IP"))
+    vm.guest_ip.ok_or_else(|| anyhow!("VM has no guest IP"))
 }
 
 fn extract_docker_container_id(container: &nexus_types::Container) -> Result<String> {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/vm.rs:1:
-use anyhow::{Context, Result};
-use uuid::Uuid;
 use crate::AppState;
+use anyhow::{Context, Result};
 use nexus_types::CreateVmReq;
+use uuid::Uuid;
 
 /// Create a dedicated MicroVM for running a Docker container
 ///
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/vm.rs:30:
     let vm_id = Uuid::new_v4();
     let container_rootfs_path = format!("/srv/images/containers/{}.ext4", vm_id);
 
-    eprintln!("[Container {}] Creating VM {} with dedicated runtime image copy", container_id, vm_id);
-    eprintln!("[Container {}] Copying {} to {}", container_id, base_rootfs_path, container_rootfs_path);
+    eprintln!(
+        "[Container {}] Creating VM {} with dedicated runtime image copy",
+        container_id, vm_id
+    );
+    eprintln!(
+        "[Container {}] Copying {} to {}",
+        container_id, base_rootfs_path, container_rootfs_path
+    );
 
     // Ensure directory exists
-    tokio::fs::create_dir_all("/srv/images/containers").await
+    tokio::fs::create_dir_all("/srv/images/containers")
+        .await
         .context("Failed to create containers image directory")?;
 
     // Copy the base runtime image to a container-specific image
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/vm.rs:45:
         .context("Failed to execute cp command")?;
 
     if !copy_status.success() {
-        anyhow::bail!("Failed to copy runtime image from {} to {}", base_rootfs_path, container_rootfs_path);
+        anyhow::bail!(
+            "Failed to copy runtime image from {} to {}",
+            base_rootfs_path,
+            container_rootfs_path
+        );
     }
 
-    eprintln!("[Container {}] Runtime image copied successfully", container_id);
+    eprintln!(
+        "[Container {}] Runtime image copied successfully",
+        container_id
+    );
 
     // Create VM request using container-specific rootfs copy
-    let vm_name = format!("container-{}-{}", container_name, &container_id.to_string()[..8]);
+    let vm_name = format!(
+        "container-{}-{}",
+        container_name,
+        &container_id.to_string()[..8]
+    );
     let vm_req = CreateVmReq {
         name: vm_name,
         vcpu,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/vm.rs:69:
     // Create and start VM
     crate::features::vms::service::create_and_start(st, vm_id, vm_req, None).await?;
 
-    eprintln!("[Container {}] VM {} created and starting", container_id, vm_id);
+    eprintln!(
+        "[Container {}] VM {} created and starting",
+        container_id, vm_id
+    );
 
     Ok(vm_id)
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/containers/vm.rs:155:
     let container_rootfs_path = format!("/srv/images/containers/{}.ext4", vm_id);
     if tokio::fs::metadata(&container_rootfs_path).await.is_ok() {
         if let Err(e) = tokio::fs::remove_file(&container_rootfs_path).await {
-            eprintln!("[Container VM] Failed to delete rootfs {}: {}", container_rootfs_path, e);
+            eprintln!(
+                "[Container VM] Failed to delete rootfs {}: {}",
+                container_rootfs_path, e
+            );
         } else {
             eprintln!("[Container VM] Deleted rootfs {}", container_rootfs_path);
         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/mod.rs:1:
-use axum::{routing::{get, post}, Router};
+use axum::{
+    routing::{get, post},
+    Router,
+};
 
 pub mod repo;
 pub mod routes;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/mod.rs:8:
 pub fn router() -> Router {
     Router::new()
         .route("/", post(routes::create).get(routes::list))
-        .route("/:id", get(routes::get).put(routes::update).delete(routes::delete))
+        .route(
+            "/:id",
+            get(routes::get).put(routes::update).delete(routes::delete),
+        )
         .route("/:id/invoke", post(routes::invoke))
         .route("/:id/logs", get(routes::logs))
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/repo.rs:187:
     vm_id: Uuid,
     guest_ip: Option<&str>,
 ) -> sqlx::Result<()> {
-    sqlx::query(
-        "UPDATE function SET vm_id = $1, guest_ip = $2, updated_at = now() WHERE id = $3"
-    )
-    .bind(vm_id)
-    .bind(guest_ip)
-    .bind(id)
-    .execute(db)
-    .await?;
+    sqlx::query("UPDATE function SET vm_id = $1, guest_ip = $2, updated_at = now() WHERE id = $3")
+        .bind(vm_id)
+        .bind(guest_ip)
+        .bind(id)
+        .execute(db)
+        .await?;
     Ok(())
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/routes.rs:5:
     Extension, Json,
 };
 use nexus_types::{
-    CreateFunctionReq, CreateFunctionResp, FunctionPathParams, GetFunctionResp,
-    InvokeFunctionReq, InvokeFunctionResp, ListFunctionsResp, ListInvocationsParams,
-    ListInvocationsResp, OkResponse, UpdateFunctionReq,
+    CreateFunctionReq, CreateFunctionResp, FunctionPathParams, GetFunctionResp, InvokeFunctionReq,
+    InvokeFunctionResp, ListFunctionsResp, ListInvocationsParams, ListInvocationsResp, OkResponse,
+    UpdateFunctionReq,
 };
 
 #[utoipa::path(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/routes.rs:46:
 pub async fn list(
     Extension(st): Extension<AppState>,
 ) -> Result<Json<ListFunctionsResp>, StatusCode> {
-    let resp = super::service::list_functions(&st.db)
-        .await
-        .map_err(|e| {
-            eprintln!("Failed to list functions: {}", e);
-            StatusCode::INTERNAL_SERVER_ERROR
-        })?;
+    let resp = super::service::list_functions(&st.db).await.map_err(|e| {
+        eprintln!("Failed to list functions: {}", e);
+        StatusCode::INTERNAL_SERVER_ERROR
+    })?;
     Ok(Json(resp))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:1:
-use anyhow::{Context, Result};
 use crate::AppState;
+use anyhow::{Context, Result};
 use sqlx::PgPool;
 use std::time::Instant;
 use uuid::Uuid;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:71:
                 eprintln!("[Function {}] VM created: {}", function_id, vm_id);
 
                 // Update function with VM ID and state
-                if let Err(e) = super::repo::update_vm_info(&st_clone.db, function_id, vm_id, None).await {
+                if let Err(e) =
+                    super::repo::update_vm_info(&st_clone.db, function_id, vm_id, None).await
+                {
                     eprintln!("[Function {}] Failed to update VM info: {}", function_id, e);
                     let _ = super::repo::update_state(&st_clone.db, function_id, "error").await;
                     return;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:93:
                     }
 
                     if attempt % 10 == 0 {
-                        eprintln!("[Function {}] Still waiting for guest IP... ({}s)", function_id, attempt);
+                        eprintln!(
+                            "[Function {}] Still waiting for guest IP... ({}s)",
+                            function_id, attempt
+                        );
                     }
                 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:109:
                 eprintln!("[Function {}] Got guest IP: {}", function_id, guest_ip);
 
                 // Update function with guest IP and state
-                let _ = super::repo::update_vm_info(&st_clone.db, function_id, vm_id, Some(&guest_ip)).await;
+                let _ =
+                    super::repo::update_vm_info(&st_clone.db, function_id, vm_id, Some(&guest_ip))
+                        .await;
                 let _ = super::repo::update_state(&st_clone.db, function_id, "deploying").await;
 
                 // Inject function code via HTTP (will retry until successful)
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:189:
         let new_handler = req.handler.unwrap_or(existing.handler);
         let runtime = req.runtime.unwrap_or(existing.runtime);
 
-        eprintln!("[Function {}] Code/handler updated, reloading in VM at {}", id, guest_ip);
+        eprintln!(
+            "[Function {}] Code/handler updated, reloading in VM at {}",
+            id, guest_ip
+        );
 
         // Reload code in background (don't block the response)
         tokio::spawn(async move {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:196:
-            if let Err(e) = super::vm::update_function_code(&guest_ip, &runtime, &new_code, &new_handler).await {
+            if let Err(e) =
+                super::vm::update_function_code(&guest_ip, &runtime, &new_code, &new_handler).await
+            {
                 eprintln!("[Function {}] Failed to reload code: {}", id, e);
             } else {
                 eprintln!("[Function {}] Code reloaded successfully", id);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:224:
         let function_rootfs_path = format!("/srv/images/functions/{}.ext4", vm_id);
         if tokio::fs::metadata(&function_rootfs_path).await.is_ok() {
             if let Err(e) = tokio::fs::remove_file(&function_rootfs_path).await {
-                eprintln!("[Function {}] Failed to delete rootfs {}: {}", id, function_rootfs_path, e);
+                eprintln!(
+                    "[Function {}] Failed to delete rootfs {}: {}",
+                    id, function_rootfs_path, e
+                );
             } else {
                 eprintln!("[Function {}] Deleted rootfs {}", id, function_rootfs_path);
             }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:256:
     }
 
     // Check if VM exists and has IP
-    let guest_ip = func.guest_ip.as_ref().context("Function VM has no IP yet")?;
+    let guest_ip = func
+        .guest_ip
+        .as_ref()
+        .context("Function VM has no IP yet")?;
 
     // Generate request ID
     let request_id = Uuid::new_v4().to_string();
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:271:
     let http_result = client
         .post(&url)
         .json(&serde_json::json!({ "event": req.event }))
-        .timeout(std::time::Duration::from_secs(func.timeout_seconds as u64 + 5))
+        .timeout(std::time::Duration::from_secs(
+            func.timeout_seconds as u64 + 5,
+        ))
         .send()
         .await;
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:282:
             if resp.status().is_success() {
                 match resp.json::<serde_json::Value>().await {
                     Ok(result) => {
-                        let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
+                        let status = result
+                            .get("status")
+                            .and_then(|v| v.as_str())
+                            .unwrap_or("unknown")
+                            .to_string();
                         let response = result.get("response").cloned();
-                        let logs = result.get("logs")
+                        let logs = result
+                            .get("logs")
                             .and_then(|v| v.as_array())
-                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
+                            .map(|arr| {
+                                arr.iter()
+                                    .filter_map(|v| v.as_str().map(String::from))
+                                    .collect()
+                            })
                             .unwrap_or_default();
-                        let error = result.get("error").and_then(|v| v.as_str()).map(String::from);
+                        let error = result
+                            .get("error")
+                            .and_then(|v| v.as_str())
+                            .map(String::from);
                         (status, response, logs, error)
                     }
-                    Err(e) => ("error".to_string(), None, vec![], Some(format!("Failed to parse response: {}", e))),
+                    Err(e) => (
+                        "error".to_string(),
+                        None,
+                        vec![],
+                        Some(format!("Failed to parse response: {}", e)),
+                    ),
                 }
             } else {
-                ("error".to_string(), None, vec![], Some(format!("HTTP {}", resp.status())))
+                (
+                    "error".to_string(),
+                    None,
+                    vec![],
+                    Some(format!("HTTP {}", resp.status())),
+                )
             }
         }
-        Err(e) => {
-            ("error".to_string(), None, vec![], Some(format!("HTTP request failed: {}", e)))
-        }
+        Err(e) => (
+            "error".to_string(),
+            None,
+            vec![],
+            Some(format!("HTTP request failed: {}", e)),
+        ),
     };
 
     // Store invocation
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/service.rs:338:
     status: Option<String>,
     limit: Option<i64>,
 ) -> Result<ListInvocationsResp> {
-    let rows =
-        super::repo::list_invocations(db, function_id, status.as_deref(), limit).await?;
+    let rows = super::repo::list_invocations(db, function_id, status.as_deref(), limit).await?;
     let items = rows.into_iter().map(invocation_row_to_type).collect();
     Ok(ListInvocationsResp { items })
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:1:
-use anyhow::{Context, Result};
-use uuid::Uuid;
 use crate::AppState;
+use anyhow::{Context, Result};
 use nexus_types::CreateVmReq;
+use uuid::Uuid;
 
 /// Create a dedicated MicroVM for running a serverless function
 ///
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:34:
     let vm_id = Uuid::new_v4();
     let function_rootfs_path = format!("/srv/images/functions/{}.ext4", vm_id);
 
-    eprintln!("[Function {}] Creating VM {} with dedicated runtime image copy", function_id, vm_id);
-    eprintln!("[Function {}] Copying {} to {}", function_id, base_rootfs_path, function_rootfs_path);
+    eprintln!(
+        "[Function {}] Creating VM {} with dedicated runtime image copy",
+        function_id, vm_id
+    );
+    eprintln!(
+        "[Function {}] Copying {} to {}",
+        function_id, base_rootfs_path, function_rootfs_path
+    );
 
     // Ensure directory exists
-    tokio::fs::create_dir_all("/srv/images/functions").await
+    tokio::fs::create_dir_all("/srv/images/functions")
+        .await
         .context("Failed to create functions image directory")?;
 
     // Copy the base runtime image to a function-specific image
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:49:
         .context("Failed to execute cp command")?;
 
     if !copy_status.success() {
-        anyhow::bail!("Failed to copy runtime image from {} to {}", base_rootfs_path, function_rootfs_path);
+        anyhow::bail!(
+            "Failed to copy runtime image from {} to {}",
+            base_rootfs_path,
+            function_rootfs_path
+        );
     }
 
-    eprintln!("[Function {}] Runtime image copied successfully", function_id);
+    eprintln!(
+        "[Function {}] Runtime image copied successfully",
+        function_id
+    );
 
     // Create VM request using function-specific rootfs copy
     let vm_name = format!("fn-{}-{}", function_name, &function_id.to_string()[..8]);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:121:
 
     // Create temporary mount point
     let mount_point = format!("/tmp/fn-inject-{}", vm_id);
-    fs::create_dir_all(&mount_point)
-        .context("Failed to create mount directory")?;
+    fs::create_dir_all(&mount_point).context("Failed to create mount directory")?;
 
     // Mount the rootfs
     let mount_output = Command::new("sudo")
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:132:
 
     if !mount_output.status.success() {
         let _ = fs::remove_dir_all(&mount_point);
-        anyhow::bail!("Failed to mount rootfs: {}", String::from_utf8_lossy(&mount_output.stderr));
+        anyhow::bail!(
+            "Failed to mount rootfs: {}",
+            String::from_utf8_lossy(&mount_output.stderr)
+        );
     }
 
     // Ensure we unmount on error or success
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:177:
     // Write environment variables if provided
     if let Some(env) = env_vars {
         let env_path = format!("{}/function/env.json", mount_point);
-        let env_json = serde_json::to_string_pretty(env)
-            .context("Failed to serialize env vars")?;
+        let env_json = serde_json::to_string_pretty(env).context("Failed to serialize env vars")?;
         if let Err(e) = fs::write(&env_path, env_json) {
             cleanup();
             anyhow::bail!("Failed to write env vars: {}", e);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:222:
     loop {
         attempt += 1;
 
-        match client
-            .post(&url)
-            .json(&payload)
-            .send()
-            .await
-        {
+        match client.post(&url).json(&payload).send().await {
             Ok(response) => {
                 if !response.status().is_success() {
-                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
+                    let error_text = response
+                        .text()
+                        .await
+                        .unwrap_or_else(|_| "Unknown error".to_string());
                     anyhow::bail!("Write-code failed: {}", error_text);
                 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:237:
-                let result: serde_json::Value = response.json().await
-                    .context("Failed to parse response")?;
+                let result: serde_json::Value =
+                    response.json().await.context("Failed to parse response")?;
 
                 if result.get("success") == Some(&serde_json::Value::Bool(true)) {
-                    eprintln!("[CodeInjection] Successfully wrote and loaded code at {} (attempt {})", guest_ip, attempt);
+                    eprintln!(
+                        "[CodeInjection] Successfully wrote and loaded code at {} (attempt {})",
+                        guest_ip, attempt
+                    );
                     return Ok(());
                 } else {
-                    let error = result.get("error")
+                    let error = result
+                        .get("error")
                         .and_then(|e| e.as_str())
                         .unwrap_or("Unknown error");
                     anyhow::bail!("Code injection failed: {}", error);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:252:
 
                 if attempt >= max_attempts {
                     eprintln!("[CodeInjection] ERROR DETAILS: {:#?}", e);
-                    anyhow::bail!("Failed to call /write-code endpoint after {} attempts: {}", max_attempts, last_error);
+                    anyhow::bail!(
+                        "Failed to call /write-code endpoint after {} attempts: {}",
+                        max_attempts,
+                        last_error
+                    );
                 }
 
                 let wait_secs = std::cmp::min(attempt, 5); // Cap at 5 seconds
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:259:
-                eprintln!("[CodeInjection] Attempt {}/{} failed, retrying in {}s...
+                eprintln!(
+                    "[CodeInjection] Attempt {}/{} failed, retrying in {}s...
 Error type: {}
 Is timeout: {}
 Is connect: {}
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/functions/vm.rs:263:
 URL: {}",
-                    attempt, max_attempts, wait_secs,
-                    if e.is_timeout() { "TIMEOUT" } else if e.is_connect() { "CONNECTION_REFUSED" } else { "OTHER" },
+                    attempt,
+                    max_attempts,
+                    wait_secs,
+                    if e.is_timeout() {
+                        "TIMEOUT"
+                    } else if e.is_connect() {
+                        "CONNECTION_REFUSED"
+                    } else {
+                        "OTHER"
+                    },
                     e.is_timeout(),
                     e.is_connect(),
-                    url);
+                    url
+                );
                 tokio::time::sleep(std::time::Duration::from_secs(wait_secs as u64)).await;
             }
         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/hosts/mod.rs:1:
-use axum::{routing::{delete, get, post}, Router};
+use axum::{
+    routing::{delete, get, post},
+    Router,
+};
 
 pub mod repo;
 pub mod routes;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/hosts/routes.rs:77:
             caps.get("used_disk_gb").and_then(|v| v.as_i64()),
         ) {
             // Update metrics in database
-            if let Err(err) = st.hosts.update_metrics(id, cpus, memory, total_disk, used_disk).await {
+            if let Err(err) = st
+                .hosts
+                .update_metrics(id, cpus, memory, total_disk, used_disk)
+                .await
+            {
                 error!(error = ?err, "failed to update host metrics");
                 // Don't fail the heartbeat if metrics update fails
             }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/hosts/routes.rs:350:
         assert_eq!(after.capabilities_json, json!({"memory": 8192}));
     }
 }
-
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:1:
 use anyhow::{Context, Result};
+use bollard::image::CreateImageOptions;
+use bollard::Docker;
+use futures::StreamExt;
 use serde::{Deserialize, Serialize};
 use std::path::PathBuf;
 use tokio::process::Command;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:5:
-use bollard::Docker;
-use bollard::image::CreateImageOptions;
-use futures::StreamExt;
 
 /// Docker Hub API client for searching and downloading images
 #[derive(Clone)]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:60:
         let auth_token = std::env::var("DOCKER_HUB_TOKEN")
             .ok()
             .filter(|t| !t.is_empty());
-        
-        Self { 
+
+        Self {
             image_root,
             auth_token,
         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:68:
     }
 
     /// Search Docker Hub for images
-    pub async fn search(&self, query: &str, limit: Option<i32>) -> Result<Vec<nexus_types::DockerHubImage>> {
+    pub async fn search(
+        &self,
+        query: &str,
+        limit: Option<i32>,
+    ) -> Result<Vec<nexus_types::DockerHubImage>> {
         let limit = limit.unwrap_or(25).min(100);
 
         let url = format!(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:83:
             .context("Failed to create HTTP client")?;
 
         let mut request = client.get(&url).header("Search-Version", "v3");
-        
+
         // Add authentication if token is available (improves rate limits)
         if let Some(token) = &self.auth_token {
             request = request.header("Authorization", format!("***", token));
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:90:
         }
-        
+
         let response = request
             .send()
             .await
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:151:
             .context("Failed to create HTTP client")?;
 
         let mut request = client.get(&url);
-        
+
         // Add authentication if token is available (improves rate limits)
         if let Some(token) = &self.auth_token {
             request = request.header("Authorization", format!("***", token));
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:158:
         }
-        
+
         let response = request
             .send()
             .await
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:205:
         progress_tracker: crate::DownloadProgressTracker,
     ) -> Result<(PathBuf, String, i64)> {
         // Try bollard (Docker API) first for better progress tracking
-        match self.download_image_with_bollard(image, registry_auth, progress_tracker.clone()).await {
+        match self
+            .download_image_with_bollard(image, registry_auth, progress_tracker.clone())
+            .await
+        {
             Ok(result) => {
                 tracing::info!("Successfully downloaded {} using Docker API", image);
                 return Ok(result);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:222:
 
         // Fallback to CLI-based download
         tracing::info!("Using CLI-based download for {}", image);
-        self.download_image_with_cli(image, registry_auth, progress_tracker).await
+        self.download_image_with_cli(image, registry_auth, progress_tracker)
+            .await
     }
 
     /// Download image using CLI (fallback method)
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:234:
     ) -> Result<(PathBuf, String, i64)> {
         // Create docker images directory if it doesn't exist
         let docker_dir = self.image_root.join("docker");
-        
+
         // Try to create the directory, providing helpful error message if it fails
         if let Err(e) = tokio::fs::create_dir_all(&docker_dir).await {
             anyhow::bail!(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:249:
 
         // Login if authentication provided
         if let Some(auth) = registry_auth {
-            let server = auth.server_address.as_deref().unwrap_or("https://index.docker.io/v1/");
+            let server = auth
+                .server_address
+                .as_deref()
+                .unwrap_or("https://index.docker.io/v1/");
             let status = Command::new("docker")
-                .args([
-                    "login",
-                    server,
-                    "-u", &auth.username,
-                    "-p", &auth.password,
-                ])
+                .args(["login", server, "-u", &auth.username, "-p", &auth.password])
                 .output()
                 .await
                 .context("Failed to execute docker login")?;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:325:
                     }
                 }
 
-                if trimmed.contains("Downloading") || trimmed.contains("Extracting") || trimmed.contains("Pull complete") {
+                if trimmed.contains("Downloading")
+                    || trimmed.contains("Extracting")
+                    || trimmed.contains("Pull complete")
+                {
                     tracing::info!("Docker pull progress: {}", trimmed);
                 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:333:
             }
         }
 
-        let pull_status = pull_process.wait().await
+        let pull_status = pull_process
+            .wait()
+            .await
             .context("Failed to wait for docker pull")?;
 
         if !pull_status.success() {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:340:
             let error_msg = if !error_output.is_empty() {
                 error_output.join("")
             } else {
-                format!("Docker pull exited with code: {}", pull_status.code().unwrap_or(-1))
+                format!(
+                    "Docker pull exited with code: {}",
+                    pull_status.code().unwrap_or(-1)
+                )
             };
-            
+
             tracing::error!("Docker pull failed for {}: {}", image, error_msg);
             anyhow::bail!("Docker pull failed: {}", error_msg);
         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:349:
-        
+
         tracing::info!("Successfully pulled Docker image: {}", image);
 
         // Sanitize image name for filename
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:353:
-        let safe_name = image
-            .replace('/', "_")
-            .replace(':', "_")
-            .replace('.', "_");
+        let safe_name = image.replace('/', "_").replace(':', "_").replace('.', "_");
         let tarball_path = docker_dir.join(format!("{}.tar", safe_name));
 
         // Save image as tarball
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:368:
         }
 
         let mut save_process = Command::new("docker")
-            .args([
-                "save",
-                "-o",
-                tarball_path.to_str().unwrap(),
-                image,
-            ])
+            .args(["save", "-o", tarball_path.to_str().unwrap(), image])
             .stdout(std::process::Stdio::piped())
             .stderr(std::process::Stdio::piped())
             .spawn()
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:380:
             .context("Failed to spawn docker save command")?;
 
-        let save_status = save_process.wait().await
+        let save_status = save_process
+            .wait()
+            .await
             .context("Failed to wait for docker save")?;
 
         if !save_status.success() {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:392:
                 if !buf.is_empty() {
                     buf
                 } else {
-                    format!("Docker save exited with code: {}", save_status.code().unwrap_or(-1))
+                    format!(
+                        "Docker save exited with code: {}",
+                        save_status.code().unwrap_or(-1)
+                    )
                 }
             } else {
-                format!("Docker save exited with code: {}", save_status.code().unwrap_or(-1))
+                format!(
+                    "Docker save exited with code: {}",
+                    save_status.code().unwrap_or(-1)
+                )
             };
-            
+
             tracing::error!("Docker save failed for {}: {}", image, error_msg);
             anyhow::bail!("Docker save failed: {}", error_msg);
         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:404:
-        
+
         tracing::info!("Successfully saved Docker image to: {:?}", tarball_path);
 
         // Get image inspect data for SHA256
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:438:
         registry_auth: Option<&nexus_types::RegistryAuth>,
         progress_tracker: crate::DownloadProgressTracker,
     ) -> Result<(PathBuf, String, i64)> {
-        tracing::info!("🔷 Attempting to download {} using Docker API (bollard)", image);
+        tracing::info!(
+            "🔷 Attempting to download {} using Docker API (bollard)",
+            image
+        );
 
         // Connect to Docker daemon
         let docker = Docker::connect_with_local_defaults()
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:445:
             .context("Failed to connect to Docker daemon - ensure Docker is running and socket is accessible")?;
 
         // Test connection
-        docker.ping().await.context("Docker daemon not responding")?;
+        docker
+            .ping()
+            .await
+            .context("Docker daemon not responding")?;
         tracing::info!("✅ Docker daemon connection successful");
 
         // Parse image name and tag
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:474:
         let mut stream = docker.create_image(Some(options), None, auth_config);
 
         // Track total progress across all layers
-        let mut layer_progress: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();
+        let mut layer_progress: std::collections::HashMap<String, (u64, u64)> =
+            std::collections::HashMap::new();
 
         while let Some(result) = stream.next().await {
             match result {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:487:
                     // Update progress based on layer information
                     if let Some(id) = &info.id {
                         if let Some(progress_detail) = &info.progress_detail {
-                            if let (Some(current), Some(total)) = (progress_detail.current, progress_detail.total) {
+                            if let (Some(current), Some(total)) =
+                                (progress_detail.current, progress_detail.total)
+                            {
                                 tracing::debug!("Layer {} progress: {} / {}", id, current, total);
                                 layer_progress.insert(id.clone(), (current as u64, total as u64));
                             }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:495:
                             // Log when progress_detail is missing
                             if let Some(status) = &info.status {
                                 if status.contains("Downloading") || status.contains("Extracting") {
-                                    tracing::debug!("Status '{}' but no progress_detail for layer {}", status, id);
+                                    tracing::debug!(
+                                        "Status '{}' but no progress_detail for layer {}",
+                                        status,
+                                        id
+                                    );
                                 }
                             }
                         }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:531:
                             // Update status based on Docker's status message
                             if let Some(status) = &info.status {
                                 match status.as_str() {
-                                    "Pulling fs layer" => progress.status = "Pulling layers...".to_string(),
+                                    "Pulling fs layer" => {
+                                        progress.status = "Pulling layers...".to_string()
+                                    }
                                     "Downloading" => {
                                         progress.status = "Downloading layers...".to_string();
                                         // Force update even if no progress_detail
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:538:
                                         if total_total == 0 {
-                                            tracing::warn!("Downloading but no size information available yet");
+                                            tracing::warn!(
+                                                "Downloading but no size information available yet"
+                                            );
                                         }
-                                    },
-                                    "Extracting" => progress.status = "Extracting layers...".to_string(),
-                                    "Pull complete" => progress.status = "Pull complete".to_string(),
+                                    }
+                                    "Extracting" => {
+                                        progress.status = "Extracting layers...".to_string()
+                                    }
+                                    "Pull complete" => {
+                                        progress.status = "Pull complete".to_string()
+                                    }
                                     "Already exists" => {
                                         progress.status = "Using cached layers...".to_string();
                                         tracing::info!("Layer already exists (cached)");
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:547:
-                                    },
-                                    "Download complete" => progress.status = "Download complete".to_string(),
-                                    "Status: Downloaded newer image" => progress.status = "Download complete".to_string(),
+                                    }
+                                    "Download complete" => {
+                                        progress.status = "Download complete".to_string()
+                                    }
+                                    "Status: Downloaded newer image" => {
+                                        progress.status = "Download complete".to_string()
+                                    }
                                     _ => {}
                                 }
                             }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:556:
                     // Log progress for monitoring (every 10% to avoid spam)
                     if let Some(status) = &info.status {
                         if total_total > 0 {
-                            let percentage = (total_current as f64 / total_total as f64 * 100.0) as u32;
+                            let percentage =
+                                (total_current as f64 / total_total as f64 * 100.0) as u32;
                             if percentage % 10 == 0 && total_current > 0 {
                                 tracing::info!(
                                     "📦 Docker pull progress: {} - {}% ({} MB / {} MB)",
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:593:
             .context("Failed to create docker images directory")?;
 
         // Sanitize image name for filename
-        let safe_name = image
-            .replace('/', "_")
-            .replace(':', "_")
-            .replace('.', "_");
+        let safe_name = image.replace('/', "_").replace(':', "_").replace('.', "_");
         let tarball_path = docker_dir.join(format!("{}.tar", safe_name));
 
         // Export image as tarball using bollard
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:605:
         // Note: Bollard doesn't have a direct "save" equivalent yet
         // We'll fall back to CLI for this part
         let mut save_process = Command::new("docker")
-            .args([
-                "save",
-                "-o",
-                tarball_path.to_str().unwrap(),
-                image,
-            ])
+            .args(["save", "-o", tarball_path.to_str().unwrap(), image])
             .stdout(std::process::Stdio::piped())
             .stderr(std::process::Stdio::piped())
             .spawn()
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:617:
             .context("Failed to spawn docker save command")?;
 
-        let save_status = save_process.wait().await
+        let save_status = save_process
+            .wait()
+            .await
             .context("Failed to wait for docker save")?;
 
         if !save_status.success() {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:623:
-            anyhow::bail!("Docker save failed with code: {}", save_status.code().unwrap_or(-1));
+            anyhow::bail!(
+                "Docker save failed with code: {}",
+                save_status.code().unwrap_or(-1)
+            );
         }
 
         // Get image inspect data for SHA256
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/dockerhub.rs:627:
-        let inspect_result = docker.inspect_image(image).await
+        let inspect_result = docker
+            .inspect_image(image)
+            .await
             .context("Failed to inspect image")?;
 
-        let sha256 = inspect_result.id
+        let sha256 = inspect_result
+            .id
             .unwrap_or_default()
             .trim_start_matches("sha256:")
             .to_string();
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/mod.rs:17:
         .route("/dockerhub/search", post(routes::dockerhub_search))
         .route("/dockerhub/tags", post(routes::dockerhub_tags))
         .route("/dockerhub/download", post(routes::dockerhub_download))
-        .route("/dockerhub/download/progress/:image_name", get(routes::dockerhub_download_progress))
+        .route(
+            "/dockerhub/download/progress/:image_name",
+            get(routes::dockerhub_download_progress),
+        )
         .route("/dockerhub/preload", post(routes::dockerhub_preload))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/preload.rs:23:
     let dockerhub = DockerHubClient::new(image_root);
     let mut loaded_ids = Vec::new();
 
-    tracing::info!("Starting pre-load of {} default images", DEFAULT_IMAGES.len());
+    tracing::info!(
+        "Starting pre-load of {} default images",
+        DEFAULT_IMAGES.len()
+    );
 
     for image_name in DEFAULT_IMAGES {
         tracing::info!("Pre-loading image: {}", image_name);
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/preload.rs:43:
         }
 
         // Download the image
-        match dockerhub.download_image(image_name, None, progress_tracker.clone()).await {
+        match dockerhub
+            .download_image(image_name, None, progress_tracker.clone())
+            .await
+        {
             Ok((tarball_path, sha256, size)) => {
                 // Register in database
                 let image_req = CreateImageReq {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/preload.rs:96:
         tracing::info!("Pre-loading image: {}", image_name);
 
         // Download the image
-        match dockerhub.download_image(image_name, None, progress_tracker.clone()).await {
+        match dockerhub
+            .download_image(image_name, None, progress_tracker.clone())
+            .await
+        {
             Ok((tarball_path, sha256, size)) => {
                 // Register in database
                 let image_req = CreateImageReq {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:7:
     Extension, Json,
 };
 use nexus_types::{
-    CreateImageReq, CreateImageResp, DockerHubSearchReq, DockerHubSearchResp,
-    DockerImageTagsResp, DownloadDockerImageReq, DownloadDockerImageResp,
-    GetImageResp, ImageFilter, ImagePathParams, ListImagesResp, OkResponse,
+    CreateImageReq, CreateImageResp, DockerHubSearchReq, DockerHubSearchResp, DockerImageTagsResp,
+    DownloadDockerImageReq, DownloadDockerImageResp, GetImageResp, ImageFilter, ImagePathParams,
+    ListImagesResp, OkResponse,
 };
 
 #[utoipa::path(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:129:
 ) -> Result<Json<DockerHubSearchResp>, StatusCode> {
     let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());
 
-    let items = dockerhub
-        .search(&req.query, req.limit)
-        .await
-        .map_err(|e| {
-            tracing::error!("Docker Hub search failed: {}", e);
-            StatusCode::INTERNAL_SERVER_ERROR
-        })?;
+    let items = dockerhub.search(&req.query, req.limit).await.map_err(|e| {
+        tracing::error!("Docker Hub search failed: {}", e);
+        StatusCode::INTERNAL_SERVER_ERROR
+    })?;
 
     Ok(Json(DockerHubSearchResp { items }))
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:156:
 ) -> Result<Json<DockerImageTagsResp>, StatusCode> {
     let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());
 
-    let items = dockerhub
-        .get_tags(&image_name)
-        .await
-        .map_err(|e| {
-            tracing::error!("Failed to get Docker image tags: {}", e);
-            StatusCode::INTERNAL_SERVER_ERROR
-        })?;
+    let items = dockerhub.get_tags(&image_name).await.map_err(|e| {
+        tracing::error!("Failed to get Docker image tags: {}", e);
+        StatusCode::INTERNAL_SERVER_ERROR
+    })?;
 
     Ok(Json(DockerImageTagsResp { items }))
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:186:
     // Initialize progress tracking
     {
         let mut progress_map = st.download_progress.lock().await;
-        progress_map.insert(req.image.clone(), DownloadProgress {
-            image: req.image.clone(),
-            status: "Initializing...".to_string(),
-            current_bytes: 0,
-            total_bytes: 0,
-            completed: false,
-            error: None,
-        });
+        progress_map.insert(
+            req.image.clone(),
+            DownloadProgress {
+                image: req.image.clone(),
+                status: "Initializing...".to_string(),
+                current_bytes: 0,
+                total_bytes: 0,
+                completed: false,
+                error: None,
+            },
+        );
     }
 
     let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:200:
 
     // Download the image and save as tarball
     let download_result = dockerhub
-        .download_image(&req.image, req.registry_auth.as_ref(), st.download_progress.clone())
+        .download_image(
+            &req.image,
+            req.registry_auth.as_ref(),
+            st.download_progress.clone(),
+        )
         .await;
 
     let (tarball_path, sha256, size) = match download_result {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:271:
     let decoded_name = urlencoding::decode(&image_name)
         .map_err(|_| StatusCode::BAD_REQUEST)?
         .to_string();
-    
+
     let progress_map = st.download_progress.lock().await;
 
     // Try both encoded and decoded names for compatibility
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:278:
-    if let Some(progress) = progress_map.get(&decoded_name).or_else(|| progress_map.get(&image_name)) {
+    if let Some(progress) = progress_map
+        .get(&decoded_name)
+        .or_else(|| progress_map.get(&image_name))
+    {
         Ok(Json(progress.clone()))
     } else {
         Err(StatusCode::NOT_FOUND)
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:305:
         StatusCode::INTERNAL_SERVER_ERROR
     })?;
 
-    Ok(Json(loaded_ids.into_iter().map(|id| id.to_string()).collect()))
+    Ok(Json(
+        loaded_ids.into_iter().map(|id| id.to_string()).collect(),
+    ))
 }
 
 #[utoipa::path(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:328:
     let mut project: Option<String> = None;
 
     // Extract metadata fields first
-    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
+    while let Some(field) = multipart
+        .next_field()
+        .await
+        .map_err(|_| StatusCode::BAD_REQUEST)?
+    {
         let field_name = field.name().unwrap_or("").to_string();
 
         match field_name.as_str() {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:370:
             Ok(name) => name,
             Err(e) => {
                 tracing::warn!("Failed to load Docker image, using filename: {}", e);
-                name.unwrap_or_else(|| {
-                    file_path
-                        .file_name()
-                        .unwrap()
-                        .to_string_lossy()
-                        .to_string()
-                })
+                name.unwrap_or_else(|| file_path.file_name().unwrap().to_string_lossy().to_string())
             }
         }
     } else {
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:383:
-        name.unwrap_or_else(|| {
-            file_path
-                .file_name()
-                .unwrap()
-                .to_string_lossy()
-                .to_string()
-        })
+        name.unwrap_or_else(|| file_path.file_name().unwrap().to_string_lossy().to_string())
     };
 
     // Register in database
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:424:
         let storage = crate::features::storage::LocalStorage::new();
         storage.init().await.unwrap();
         let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
-        let download_progress = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
+        let download_progress =
+            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
         let state = crate::AppState {
             db: pool.clone(),
             hosts,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/images/routes.rs:480:
         let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
         let storage = crate::features::storage::LocalStorage::new();
         storage.init().await.unwrap();
-        let download_progress = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
+        let download_progress =
+            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
         let state = crate::AppState {
             db: pool,
             hosts,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/mod.rs:17:
 
 pub fn router(state: AppState) -> Router {
     Router::new()
-        .nest("/v1/auth", users::auth_router()
-            .route_layer(axum::middleware::from_fn_with_state(
+        .nest(
+            "/v1/auth",
+            users::auth_router().route_layer(axum::middleware::from_fn_with_state(
                 state.clone(),
                 users::middleware::auth_middleware,
-            )))
-        .nest("/v1/users", users::users_router()
-            .layer(axum::middleware::from_fn(users::middleware::require_admin))
-            .layer(axum::middleware::from_fn_with_state(
-                state.clone(),
-                users::middleware::auth_middleware,
-            )))
+            )),
+        )
+        .nest(
+            "/v1/users",
+            users::users_router()
+                .layer(axum::middleware::from_fn(users::middleware::require_admin))
+                .layer(axum::middleware::from_fn_with_state(
+                    state.clone(),
+                    users::middleware::auth_middleware,
+                )),
+        )
         .nest("/v1/hosts", hosts::router())
         .nest("/v1/images", images::router())
         .nest("/v1/networks", networks::router())
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/networks/mod.rs:9:
 pub fn router() -> Router {
     Router::new()
         .route("/", post(routes::create).get(routes::list))
-        .route("/:id", get(routes::get).patch(routes::update).delete(routes::delete))
+        .route(
+            "/:id",
+            get(routes::get)
+                .patch(routes::update)
+                .delete(routes::delete),
+        )
         .route("/:id/vms", get(routes::get_vms))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/networks/routes.rs:140:
 
         // Get host name if host_id is present
         let host_name = if let Some(host_id) = network.host_id {
-            st.hosts
-                .get(host_id)
-                .await
-                .ok()
-                .map(|h| h.name)
+            st.hosts.get(host_id).await.ok().map(|h| h.name)
         } else {
             None
         };
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/networks/routes.rs:195:
 
     // Get host name
     let host_name = if let Some(host_id) = network.host_id {
-        st.hosts
-            .get(host_id)
-            .await
-            .ok()
-            .map(|h| h.name)
+        st.hosts.get(host_id).await.ok().map(|h| h.name)
     } else {
         None
     };
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/templates/mod.rs:9:
 pub fn router() -> Router {
     Router::new()
         .route("/", post(routes::create).get(routes::list))
-        .route("/:id", get(routes::get).put(routes::update).delete(routes::delete))
+        .route(
+            "/:id",
+            get(routes::get).put(routes::update).delete(routes::delete),
+        )
         .route("/:id/instantiate", post(routes::instantiate))
 }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/templates/routes.rs:178:
         let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
         let storage = crate::features::storage::LocalStorage::new();
         storage.init().await.unwrap();
-        let download_progress = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
+        let download_progress =
+            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
         let state = crate::AppState {
             db: pool.clone(),
             hosts: hosts.clone(),
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/audit.rs:2:
 ///
 /// This module provides functions to log user actions to the audit_logs table
 /// for compliance, security auditing, and debugging purposes.
-
 use anyhow::Result;
 use nexus_types::{AuditAction, AuditLog, AuditLogQueryParams, ListAuditLogsResponse};
 use sqlx::PgPool;
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/audit.rs:179:
 
     // Get total count
     let count_query = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);
-    let total: i64 = sqlx::query_scalar(&count_query)
-        .fetch_one(pool)
-        .await?;
+    let total: i64 = sqlx::query_scalar(&count_query).fetch_one(pool).await?;
 
     // Get paginated results
     let logs_query = format!(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/authz.rs:2:
 ///
 /// This module provides permission checking functions for the RBAC system.
 /// It supports three roles (Admin, User, Viewer) with resource ownership checks.
-
 use nexus_types::Role;
 use uuid::Uuid;
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/authz.rs:45:
 /// A resource with `owner_id = None` can only be modified by admins.
 pub fn can_modify_resource(role: Role, owner_id: Option<Uuid>, user_id: Uuid) -> bool {
     match role {
-        Role::Admin => true,  // Admins can modify everything
+        Role::Admin => true,   // Admins can modify everything
         Role::Viewer => false, // Viewers cannot modify anything
         Role::User => {
             // Users can only modify resources they own
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/authz.rs:121:
         assert!(can_view_resource(Role::Admin, None, admin_id));
 
         // Viewer can view everything
-        assert!(can_view_resource(Role::Viewer, Some(user_id), other_user_id));
+        assert!(can_view_resource(
+            Role::Viewer,
+            Some(user_id),
+            other_user_id
+        ));
         assert!(can_view_resource(Role::Viewer, None, other_user_id));
 
         // User can view own resources
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/authz.rs:155:
         assert!(!can_modify_resource(Role::User, None, user_id));
 
         // User cannot modify other users' resources
-        assert!(!can_modify_resource(Role::User, Some(other_user_id), user_id));
+        assert!(!can_modify_resource(
+            Role::User,
+            Some(other_user_id),
+            user_id
+        ));
     }
 
     #[test]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/authz.rs:162:
     fn test_can_delete_resource() {
         // Same logic as modify
         let user_id = Uuid::new_v4();
-        assert!(can_delete_resource(Role::Admin, Some(user_id), Uuid::new_v4()));
+        assert!(can_delete_resource(
+            Role::Admin,
+            Some(user_id),
+            Uuid::new_v4()
+        ));
         assert!(can_delete_resource(Role::User, Some(user_id), user_id));
         assert!(!can_delete_resource(Role::Viewer, Some(user_id), user_id));
     }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/middleware.rs:1:
+use crate::features::users::repo::{AuthenticatedUser, UserRepoError};
+use crate::AppState;
 use axum::{
     extract::Request,
     http::{header::AUTHORIZATION, StatusCode},
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/middleware.rs:5:
     response::Response,
     Extension,
 };
-use crate::AppState;
-use crate::features::users::repo::{AuthenticatedUser, UserRepoError};
 use nexus_types::Role;
 
 pub async fn auth_middleware(
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/middleware.rs:34:
         return Err(StatusCode::UNAUTHORIZED);
     }
 
-    let user = st
-        .users
-        .validate_token(token)
-        .await
-        .map_err(|e| match e {
-            UserRepoError::InvalidToken | UserRepoError::TokenExpired => StatusCode::UNAUTHORIZED,
-            _ => StatusCode::INTERNAL_SERVER_ERROR,
-        })?;
+    let user = st.users.validate_token(token).await.map_err(|e| match e {
+        UserRepoError::InvalidToken | UserRepoError::TokenExpired => StatusCode::UNAUTHORIZED,
+        _ => StatusCode::INTERNAL_SERVER_ERROR,
+    })?;
 
     req.extensions_mut().insert(user);
     Ok(next.run(req).await)
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/middleware.rs:65:
     if let Some(forwarded) = req.headers().get("x-forwarded-for") {
         if let Ok(forwarded_str) = forwarded.to_str() {
             // Take the first IP if there are multiple
-            return forwarded_str.split(',').next().map(|s| s.trim().to_string());
+            return forwarded_str
+                .split(',')
+                .next()
+                .map(|s| s.trim().to_string());
         }
     }
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/middleware.rs:79:
     // TODO: Could extract from connection info if available
     None
 }
-
 
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/mod.rs:1:
-use axum::{routing::{get, post, patch, delete}, Router};
+use axum::{
+    routing::{delete, get, patch, post},
+    Router,
+};
 
+pub mod audit;
+pub mod authz;
+pub mod middleware;
 pub mod repo;
 pub mod routes;
-pub mod middleware;
-pub mod authz;
-pub mod audit;
 
 pub fn auth_router() -> Router {
     Router::new()
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/apps/manager/src/features/users/mod.rs:11:
     pub timeout_seconds: i32,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:604:
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub guest_ip: Option<String>,
     pub port: i32,
-    pub state: String,  // creating, ready, error, stopped
+    pub state: String, // creating, ready, error, stopped
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub created_by_user_id: Option<uuid::Uuid>,
     pub created_at: chrono::DateTime<chrono::Utc>,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:617:
 pub struct FunctionInvocation {
     pub id: uuid::Uuid,
     pub function_id: uuid::Uuid,
-    pub status: String,  // success, error, timeout
+    pub status: String, // success, error, timeout
     pub duration_ms: i64,
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub memory_used_mb: Option<i32>,
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:753:
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub memory_limit_mb: Option<i32>,
     pub restart_policy: String,
-    pub state: String,  // creating, running, stopped, restarting, error, paused
+    pub state: String, // creating, running, stopped, restarting, error, paused
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub host_id: Option<uuid::Uuid>,
     #[serde(default, skip_serializing_if = "Option::is_none")]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:783:
 pub struct PortMapping {
     pub host: i32,
     pub container: i32,
-    pub protocol: String,  // tcp, udp
+    pub protocol: String, // tcp, udp
 }
 
 #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:799:
     pub username: String,
     pub password: String,
     #[serde(default, skip_serializing_if = "Option::is_none")]
-    pub server_address: Option<String>,  // e.g., "registry.example.com" or leave None for Docker Hub
+    pub server_address: Option<String>, // e.g., "registry.example.com" or leave None for Docker Hub
 }
 
 #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:892:
     pub id: uuid::Uuid,
     pub container_id: uuid::Uuid,
     pub timestamp: chrono::DateTime<chrono::Utc>,
-    pub stream: String,  // stdout, stderr
+    pub stream: String, // stdout, stderr
     pub message: String,
     pub created_at: chrono::DateTime<chrono::Utc>,
 }
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:918:
 #[derive(Debug, Clone, Deserialize, IntoParams)]
 pub struct ContainerLogsParams {
     #[serde(default, skip_serializing_if = "Option::is_none")]
-    pub since: Option<String>,  // RFC3339 timestamp
+    pub since: Option<String>, // RFC3339 timestamp
     #[serde(default, skip_serializing_if = "Option::is_none")]
-    pub until: Option<String>,  // RFC3339 timestamp
+    pub until: Option<String>, // RFC3339 timestamp
     #[serde(default, skip_serializing_if = "Option::is_none")]
-    pub tail: Option<i64>,      // Last N lines
+    pub tail: Option<i64>, // Last N lines
     #[serde(default, skip_serializing_if = "Option::is_none")]
-    pub follow: Option<bool>,   // Stream logs
+    pub follow: Option<bool>, // Stream logs
 }
 
 #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
Diff in /home/runner/work/NQRust-MicroVM/NQRust-MicroVM/crates/nexus-types/src/lib.rs:971:
             Role::Viewer => "viewer",
         }
     }
-
 }
 
 impl std::str::FromStr for Role {