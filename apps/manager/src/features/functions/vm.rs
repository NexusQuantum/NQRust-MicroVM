use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tracing::{info, warn};
use uuid::Uuid;

use crate::AppState;
use nexus_types::CreateVmReq;

/// Create a dedicated MicroVM for running a serverless function
///
/// This spawns a lightweight VM with:
/// - Runtime-specific rootfs (Node.js or Python) - COPIED per-function for isolation
/// - Function code written to /function/code.{js,py}
/// - Runtime server auto-starting on boot
/// - Minimal resources (configurable vCPU and memory)
///
/// If a golden snapshot exists for the runtime, uses fast restore path (~5-15s).
/// Otherwise, falls back to traditional boot path (~120s).

/// Detect IP from bridge by monitoring neighbor table (ARP cache)
/// This works for snapshot-restored VMs that keep their old IP without DHCP
async fn detect_ip_from_bridge(tap_device: &str, bridge: &str, timeout_secs: u64) -> anyhow::Result<Option<String>> {
    use tokio::process::Command;
    use std::collections::HashSet;
    
    info!(tap_device = %tap_device, bridge = %bridge, "Starting IP detection from neighbor table");
    
    // Take a baseline snapshot of existing IPs on the bridge BEFORE we start looking
    // This helps us identify which IP is NEW (belongs to our just-started VM)
    let mut baseline_ips = HashSet::new();
    if let Ok(output) = Command::new("ip")
        .args(["neigh", "show", "dev", bridge])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(ip) = line.split_whitespace().next() {
                if ip != "127.0.0.1" && !ip.is_empty() {
                    baseline_ips.insert(ip.to_string());
                }
            }
        }
    }
    
    info!(
        tap_device = %tap_device,
        baseline_count = baseline_ips.len(),
        "Captured baseline neighbor table"
    );
    
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut attempt = 0;
    
    while Instant::now() < deadline {
        attempt += 1;
        
        // Query neighbor table for all entries on the bridge
        let neighbor_output = Command::new("ip")
            .args(["neigh", "show", "dev", bridge])
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&neighbor_output.stdout);
        
        // Look for REACHABLE entries (active connections)
        for line in stdout.lines() {
            if line.contains("REACHABLE") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 1 {
                    let ip = parts[0];
                    
                    // Skip localhost and empty
                    if ip == "127.0.0.1" || ip.is_empty() {
                        continue;
                    }
                    
                    // Check if this is a NEW IP (not in baseline)
                    if !baseline_ips.contains(ip) {
                        info!(
                            tap_device = %tap_device,
                            ip = %ip,
                            attempt = attempt,
                            "Found new REACHABLE IP in neighbor table"
                        );
                        return Ok(Some(ip.to_string()));
                    }
                }
            }
        }
        
        if attempt % 5 == 0 {
            info!(
                tap_device = %tap_device,
                attempt = attempt,
                "Still waiting for VM to appear in neighbor table..."
            );
        }
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    warn!(tap_device = %tap_device, "Failed to detect IP from neighbor table after timeout");
    Ok(None)
}

pub async fn create_with_vm(
    st: &AppState,
    function_id: Uuid,
    function_name: &str,
    runtime: &str,
    code: &str,
    handler: &str,
    vcpu: u8,
    memory_mb: u32,
    env_vars: &Option<serde_json::Value>,
) -> Result<Uuid> {
    use tokio::process::Command;

    // Check if golden snapshot exists for this runtime
    let snapshot_vm_path = format!("/srv/snapshots/{}-golden-vm.snap", runtime);
    let snapshot_mem_path = format!("/srv/snapshots/{}-golden-mem.snap", runtime);

    let has_snapshot = tokio::fs::metadata(&snapshot_vm_path).await.is_ok()
        && tokio::fs::metadata(&snapshot_mem_path).await.is_ok();

    if has_snapshot {
        eprintln!("[Function {}] ⚡ FAST PATH: Restoring from golden snapshot via API", function_id);
        return create_function_vm_from_snapshot(
            st,
            function_id,
            function_name,
            runtime,
            code,
            handler,
            vcpu,
            memory_mb,
            env_vars,
            &snapshot_vm_path,
            &snapshot_mem_path,
        ).await;
    }

    eprintln!("[Function {}] Using traditional boot path (no snapshot available)", function_id);

    // Get runtime-specific image paths (base images)
    let (kernel_path, base_rootfs_path) = get_runtime_image_paths(runtime)?;

    // Create a per-function copy of the runtime image
    // This is necessary because:
    // 1. Firecracker requires exclusive write access to rootfs
    // 2. Guest agent installation modifies the rootfs
    // 3. Multiple VMs cannot share the same writable rootfs file
    let vm_id = Uuid::new_v4();
    let function_rootfs_path = format!("/srv/images/functions/{}.ext4", vm_id);

    eprintln!(
        "[Function {}] Creating VM {} with dedicated runtime image copy",
        function_id, vm_id
    );
    eprintln!(
        "[Function {}] Copying {} to {}",
        function_id, base_rootfs_path, function_rootfs_path
    );

    // Ensure directory exists
    tokio::fs::create_dir_all("/srv/images/functions")
        .await
        .context("Failed to create functions image directory")?;

    // Use fast reflink copy (instant on btrfs, falls back to regular copy otherwise)
    if let Err(e) = crate::features::vms::fast_provisioning::reflink_copy(
        &base_rootfs_path,
        &function_rootfs_path,
    )
    .await
    {
        anyhow::bail!(
            "Failed to copy runtime image from {} to {}: {}",
            base_rootfs_path,
            function_rootfs_path,
            e
        );
    }

    eprintln!(
        "[Function {}] Runtime image copied successfully",
        function_id
    );

    // Inject function code before the VM boots so the runtime loads it immediately
    match inject_function_code(
        vm_id,
        runtime,
        code,
        handler,
        env_vars,
        &function_rootfs_path,
    )
    .await
    {
        Ok(_) => {
            eprintln!(
                "[Function {}] Function code injected into rootfs prior to boot",
                function_id
            );
        }
        Err(e) => {
            eprintln!(
                "[Function {}] Pre-boot code injection skipped (will fall back to runtime API): {}",
                function_id, e
            );
        }
    }

    // Create VM request using function-specific rootfs copy
    let vm_name = format!("fn-{}-{}", function_name, &function_id.to_string()[..8]);
    let vm_req = CreateVmReq {
        name: vm_name,
        vcpu,
        mem_mib: memory_mb,
        kernel_image_id: None,
        rootfs_image_id: None,
        kernel_path: Some(kernel_path),
        rootfs_path: Some(function_rootfs_path),
        source_snapshot_id: None,
        username: Some("root".to_string()),
        password: Some("function".to_string()),
    };

    // Create and start VM
    crate::features::vms::service::create_and_start(st, vm_id, vm_req, None).await?;

    // Runtime code is already present in the rootfs; the service layer can fall back
    // to HTTP-based injection if the runtime fails to load it during boot.

    Ok(vm_id)
}

/// Get kernel and rootfs paths for a given runtime
pub fn get_runtime_image_paths(runtime: &str) -> Result<(String, String)> {
    // TODO: These paths should be configurable via environment variables
    // or stored in a database/config file
    //
    // For now, return placeholder paths
    // In production, you'd build custom runtime images with:
    // - Alpine Linux base
    // - Node.js/Python installed
    // - Runtime server (server.js or server.py) in /usr/local/bin/
    // - Systemd/OpenRC service to auto-start runtime server

    let kernel = "/srv/images/vmlinux-5.10.fc.bin".to_string();

    let rootfs = match runtime {
        "node" => "/srv/images/node-runtime.ext4",
        "python" => "/srv/images/python-runtime.ext4",
        "go" => "/srv/images/go-runtime.ext4",
        "rust" => "/srv/images/rust-runtime.ext4",
        _ => anyhow::bail!("Unsupported runtime: {}", runtime),
    };

    Ok((kernel, rootfs.to_string()))
}

/// Inject function code into the VM's rootfs before it starts
///
/// This mounts the rootfs, writes the function code, handler config, and env vars,
/// then unmounts it.
pub async fn inject_function_code(
    vm_id: Uuid,
    runtime: &str,
    code: &str,
    handler: &str,
    env_vars: &Option<serde_json::Value>,
    rootfs_path: &str,
) -> Result<()> {
    use std::fs;
    use std::process::Command;

    // Create temporary mount point
    let mount_point = format!("/tmp/fn-inject-{}", vm_id);
    fs::create_dir_all(&mount_point).context("Failed to create mount directory")?;

    // Mount the rootfs
    let mount_output = Command::new("sudo")
        .args(&["mount", "-o", "loop", rootfs_path, &mount_point])
        .output()
        .context("Failed to execute mount command")?;

    if !mount_output.status.success() {
        let _ = fs::remove_dir_all(&mount_point);
        anyhow::bail!(
            "Failed to mount rootfs: {}",
            String::from_utf8_lossy(&mount_output.stderr)
        );
    }

    // Ensure we unmount on error or success
    let cleanup = || {
        let _ = Command::new("sudo")
            .args(&["umount", &mount_point])
            .output();
        let _ = fs::remove_dir_all(&mount_point);
    };

    // Write function code
    let file_extension = match runtime {
        "node" => "js",
        "python" => "py",
        _ => {
            cleanup();
            anyhow::bail!("Unsupported runtime: {}", runtime);
        }
    };

    let code_path = format!("{}/function/code.{}", mount_point, file_extension);

    // For Node.js, we need to export the handler
    let code_content = if runtime == "node" {
        format!("{}\n\nmodule.exports = {{ {} }};", code, handler)
    } else {
        code.to_string()
    };

    if let Err(e) = fs::write(&code_path, code_content) {
        cleanup();
        anyhow::bail!("Failed to write function code: {}", e);
    }

    // Write handler name to a config file (optional, for debugging)
    let handler_path = format!("{}/function/handler.txt", mount_point);
    if let Err(e) = fs::write(&handler_path, handler) {
        cleanup();
        anyhow::bail!("Failed to write handler config: {}", e);
    }

    // Write environment variables if provided
    if let Some(env) = env_vars {
        let env_path = format!("{}/function/env.json", mount_point);
        let env_json = serde_json::to_string_pretty(env).context("Failed to serialize env vars")?;
        if let Err(e) = fs::write(&env_path, env_json) {
            cleanup();
            anyhow::bail!("Failed to write env vars: {}", e);
        }
    }

    // Unmount
    cleanup();

    Ok(())
}

/// Update function code in an existing VM via HTTP
///
/// This calls the /write-code endpoint on the runtime server to write
/// the function code and automatically reload it.
pub async fn update_function_code(
    guest_ip: &str,
    _runtime: &str,
    code: &str,
    handler: &str,
) -> Result<()> {
    let url = format!("http://{}:3000/write-code", guest_ip);

    let payload = serde_json::json!({
        "code": code,
        "handler": handler,
    });

    eprintln!("[CodeInjection] Writing code to {} via HTTP", guest_ip);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    // Retry with exponential backoff until successful (max 2 minutes)
    let mut attempt = 0;
    let max_attempts = 60; // ~5 minutes total with 5s delays
    let mut last_error = String::new();

    loop {
        attempt += 1;

        match client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    anyhow::bail!("Write-code failed: {}", error_text);
                }

                let result: serde_json::Value =
                    response.json().await.context("Failed to parse response")?;

                if result.get("success") == Some(&serde_json::Value::Bool(true)) {
                    eprintln!(
                        "[CodeInjection] Successfully wrote and loaded code at {} (attempt {})",
                        guest_ip, attempt
                    );
                    return Ok(());
                } else {
                    let error = result
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown error");
                    anyhow::bail!("Code injection failed: {}", error);
                }
            }
            Err(e) => {
                last_error = format!("{:?}", e);

                if attempt >= max_attempts {
                    eprintln!("[CodeInjection] ERROR DETAILS: {:#?}", e);
                    anyhow::bail!(
                        "Failed to call /write-code endpoint after {} attempts: {}",
                        max_attempts,
                        last_error
                    );
                }

                let wait_secs = std::cmp::min(attempt, 5); // Cap at 5 seconds
                eprintln!(
                    "[CodeInjection] Attempt {}/{} failed, retrying in {}s...
Error type: {}
Is timeout: {}
Is connect: {}
URL: {}",
                    attempt,
                    max_attempts,
                    wait_secs,
                    if e.is_timeout() {
                        "TIMEOUT"
                    } else if e.is_connect() {
                        "CONNECTION_REFUSED"
                    } else {
                        "OTHER"
                    },
                    e.is_timeout(),
                    e.is_connect(),
                    url
                );
                tokio::time::sleep(std::time::Duration::from_secs(wait_secs as u64)).await;
            }
        }
    }
}

/// Wait until the runtime server reports that code is loaded and ready.
pub async fn wait_for_runtime_ready(guest_ip: &str, timeout_secs: u64) -> Result<()> {
    let url = format!("http://{}:3000/health", guest_ip);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut attempt = 0usize;

    loop {
        attempt += 1;

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let payload: serde_json::Value = resp
                    .json()
                    .await
                    .context("Failed to parse runtime /health response")?;

                let code_loaded = payload
                    .get("codeLoaded")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if code_loaded {
                    eprintln!(
                        "[RuntimeReady] Runtime at {} reported ready after {} attempts",
                        guest_ip, attempt
                    );
                    return Ok(());
                }
            }
            Ok(resp) => {
                eprintln!(
                    "[RuntimeReady] /health returned HTTP {} on attempt {}",
                    resp.status(),
                    attempt
                );
            }
            Err(err) => {
                eprintln!(
                    "[RuntimeReady] Health check attempt {} failed: {}",
                    attempt, err
                );
            }
        }

        if Instant::now() >= deadline {
            anyhow::bail!(
                "Runtime at {} did not become ready within {} seconds",
                guest_ip,
                timeout_secs
            );
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Create a function VM by restoring from a golden snapshot
///
/// This is the fast path that takes ~5-15 seconds instead of 120+ seconds
async fn create_function_vm_from_snapshot(
    st: &AppState,
    function_id: Uuid,
    function_name: &str,
    runtime: &str,
    code: &str,
    handler: &str,
    vcpu: u8,
    memory_mb: u32,
    _env_vars: &Option<serde_json::Value>,
    snapshot_vm_path: &str,
    snapshot_mem_path: &str,
) -> Result<Uuid> {
    let start_time = Instant::now();

    // Get runtime-specific image paths
    let (kernel_path, _base_rootfs_path) = get_runtime_image_paths(runtime)?;

    // Create unique VM ID
    let vm_id = Uuid::new_v4();

    // IMPORTANT: For snapshot restore, we MUST use the same rootfs path that was used
    // when creating the golden snapshot. The snapshot includes drive configuration with
    // the original path baked in. We cannot override this during restore.
    //
    // The golden snapshot rootfs is stored at /srv/images/functions/{runtime}-golden.ext4
    // This is the path that was used during snapshot creation and must be used for restore.
    let golden_rootfs_path = format!("/srv/images/functions/{}-golden.ext4", runtime);

    eprintln!("[Function {}] Using golden snapshot rootfs: {}", function_id, golden_rootfs_path);
    eprintln!("[Function {}] Snapshot includes drive config - no per-function copy needed", function_id);

    // Create VM request using the golden snapshot rootfs
    let vm_name = format!("fn-{}-{}", function_name, &function_id.to_string()[..8]);
    let mut vm_req = nexus_types::CreateVmReq {
        name: vm_name,
        vcpu,
        mem_mib: memory_mb,
        kernel_image_id: None,
        rootfs_image_id: None,
        kernel_path: Some(kernel_path),
        rootfs_path: Some(golden_rootfs_path.clone()), // Use golden snapshot rootfs
        source_snapshot_id: None,
        username: Some("root".to_string()),
        password: Some("function".to_string()),
    };

    eprintln!("[Function {}] Creating VM with snapshot restore...", function_id);

    // Select a host to run the VM
    let selected_host = st
        .hosts
        .first_healthy()
        .await
        .context("no healthy hosts available")?;

    // Create a dummy snapshot row to use the existing create_from_snapshot infrastructure
    // Note: We use a fixed UUID for golden snapshots since they're managed externally
    let golden_snapshot_id = match runtime {
        "node" => Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        "python" => Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        "go" => Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        "rust" => Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap(),
        _ => Uuid::new_v4(),
    };
    
    let dummy_snapshot = crate::features::snapshots::repo::SnapshotRow {
        id: golden_snapshot_id,
        vm_id: Uuid::new_v4(), // Dummy source VM ID
        snapshot_path: snapshot_vm_path.to_string(),
        mem_path: snapshot_mem_path.to_string(),
        size_bytes: 0,
        state: "ready".to_string(),
        snapshot_type: "Full".to_string(),
        parent_id: None,
        track_dirty_pages: false,
        name: Some(format!("{}-golden", runtime)),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Create a dummy source VM row with the configuration we need
    let dummy_source_vm = crate::features::vms::repo::VmRow {
        id: Uuid::new_v4(),
        name: format!("{}-golden-source", runtime),
        state: "stopped".to_string(),
        host_id: selected_host.id, // Use the selected host
        template_id: None,
        host_addr: selected_host.addr.clone(),
        api_sock: String::new(),
        tap: String::new(),
        log_path: String::new(),
        http_port: 0,
        fc_unit: String::new(),
        vcpu: vcpu as i32,
        mem_mib: memory_mb as i32,
        kernel_path: vm_req.kernel_path.clone().unwrap(),
        rootfs_path: vm_req.rootfs_path.clone().unwrap(),
        source_snapshot_id: None,
        guest_ip: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Use the existing create_from_snapshot which handles all the complexity
    match crate::features::vms::service::create_from_snapshot(
        st,
        vm_id,
        vm_req.name.clone(),
        None,
        dummy_snapshot,
        Some(dummy_source_vm),
    )
    .await
    {
        Ok(_) => {},
        Err(e) => {
            eprintln!("[Function {}] Snapshot restore failed: {:#}", function_id, e);
            anyhow::bail!("Failed to create VM from snapshot: {:#}", e);
        }
    }

    eprintln!(
        "[Function {}] VM {} restored from snapshot in {:.2}s",
        function_id,
        vm_id,
        start_time.elapsed().as_secs_f64()
    );

    // The guest agent in the snapshot will automatically detect snapshot restore on its next heartbeat
    // First heartbeat after restore happens immediately (~3s), then network restart (~5s), then DHCP (~5s)
    eprintln!("[Function {}] Waiting for guest agent to detect snapshot restore and restart networking...", function_id);
    eprintln!("[Function {}] (Expected: ~13 seconds total)", function_id);
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Detect the VM's IP address
    // We use ARP scanning since the guest agent in the snapshot has the wrong VM ID
    // and won't report to our VM's database record initially
    eprintln!("[Function {}] Detecting VM IP via ARP scan...", function_id);

    let guest_ip = 'outer: {
        let tap_device = format!("tap-{}", &vm_id.to_string()[..8]);
        let bridge = std::env::var("FC_BRIDGE").unwrap_or_else(|_| "fcbr0".to_string());

        // Wait a moment for VM networking to initialize
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Try to detect IP via ARP
        match detect_ip_from_bridge(&tap_device, &bridge, 15).await {
            Ok(Some(ip)) => {
                eprintln!("[Function {}] Detected IP via ARP: {}", function_id, ip);
                ip
            }
            Ok(None) | Err(_) => {
                // Fallback: The guest agent in the snapshot has an old VM ID baked in
                // It will report the NEW IP to the OLD VM's database record
                // We need to find which VM recently got an IP update
                eprintln!("[Function {}] ARP detection failed, checking for recently reported IPs...", function_id);

                // Since ARP failed, we know the VM kept its old IP from the snapshot
                // The guest agent IS reporting, but to the wrong VM ID (the golden template)
                // Just grab the most recently updated IP - it's almost certainly ours

                tokio::time::sleep(Duration::from_secs(2)).await;

                let rows = sqlx::query!(
                    "SELECT id, guest_ip, updated_at FROM vm
                     WHERE guest_ip IS NOT NULL
                     ORDER BY updated_at DESC
                     LIMIT 3"
                )
                .fetch_all(&st.db)
                .await?;

                eprintln!("[Function {}] Found {} VMs with IPs (showing most recent):", function_id, rows.len());
                for (i, row) in rows.iter().enumerate() {
                    eprintln!("[Function {}]   {}. VM {} - IP: {:?}, updated: {:?}",
                        function_id, i + 1, row.id, row.guest_ip, row.updated_at);
                }

                // First check our own VM
                for row in &rows {
                    if row.id == vm_id {
                        if let Some(ref ip) = row.guest_ip {
                            eprintln!("[Function {}] ✅ Our VM has IP: {}", function_id, ip);
                            break 'outer ip.clone();
                        }
                    }
                }

                // Use the most recently updated IP (guest agent reporting to wrong VM)
                if let Some(row) = rows.first() {
                    if let Some(ref ip) = row.guest_ip {
                        eprintln!("[Function {}] Using most recent IP: {} (from VM {}, guest agent has wrong VM ID)",
                            function_id, ip, row.id);
                        break 'outer ip.clone();
                    }
                }

                anyhow::bail!("No VMs with IPs found in database")
            }
        }
    };

    eprintln!(
        "[Function {}] Got IP: {} in {:.2}s",
        function_id,
        guest_ip,
        start_time.elapsed().as_secs_f64()
    );

    // Update guest agent config so it starts reporting to the correct VM ID
    eprintln!("[Function {}] Updating guest agent to use correct VM ID...", function_id);
    
    let mut config_updated = false;
    for attempt in 1..=10 {
        match crate::features::vms::service::update_guest_agent_config(
            &guest_ip,
            vm_id,
            "http://127.0.0.1:8080",
        ).await {
            Ok(_) => {
                eprintln!("[Function {}] ✅ Guest agent config updated (attempt {})", function_id, attempt);
                config_updated = true;
                break;
            }
            Err(e) => {
                if attempt >= 10 {
                    eprintln!("[Function {}] ⚠️  Failed to update guest agent config: {}", function_id, e);
                } else if attempt % 3 == 0 {
                    eprintln!("[Function {}] Retrying config update... (attempt {})", function_id, attempt);
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    if config_updated {
        // Verify guest agent now reports to our VM
        tokio::time::sleep(Duration::from_secs(2)).await;
        let vm_check = crate::features::vms::repo::get(&st.db, vm_id).await?;
        if vm_check.guest_ip.as_ref() == Some(&guest_ip) {
            eprintln!("[Function {}] ✅ Guest agent now reporting to correct VM", function_id);
        } else {
            eprintln!("[Function {}] ⚠️  Guest agent config updated but not yet reporting to us", function_id);
        }
    }

    // Inject function code into the restored VM via runtime API
    eprintln!("[Function {}] Injecting function code...", function_id);
    inject_code_via_http(&guest_ip, runtime, code, handler).await?;

    eprintln!(
        "[Function {}] ✅ Function ready in {:.2}s (snapshot fast path)",
        function_id,
        start_time.elapsed().as_secs_f64()
    );

    Ok(vm_id)
}

/// Inject code via HTTP to the runtime server
async fn inject_code_via_http(guest_ip: &str, runtime: &str, code: &str, handler: &str) -> Result<()> {
    let url = format!("http://{}:3000/write-code", guest_ip);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let file_name = match runtime {
        "node" => "code.js",
        "python" => "code.py",
        _ => anyhow::bail!("Unsupported runtime: {}", runtime),
    };

    let body = serde_json::json!({
        "code": code,
        "handler": handler,
        "file_name": file_name,
    });

    for attempt in 1..=30 {
        match client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                eprintln!("[CodeInjection] Code injected successfully");
                return Ok(());
            }
            Ok(resp) => {
                if attempt >= 30 {
                    anyhow::bail!("Code injection failed: HTTP {}", resp.status());
                }
            }
            Err(e) if e.is_timeout() || e.is_connect() => {
                if attempt >= 30 {
                    anyhow::bail!("Code injection failed after {} attempts: timeout/connection error", attempt);
                }
            }
            Err(e) => {
                if attempt >= 30 {
                    anyhow::bail!("Code injection failed: {}", e);
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}


