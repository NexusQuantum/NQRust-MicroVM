use crate::AppState;
use anyhow::{Context, Result};
use nexus_types::CreateVmReq;
use uuid::Uuid;

/// Create a dedicated MicroVM for running a serverless function
///
/// This spawns a lightweight VM with:
/// - Runtime-specific rootfs (Node.js or Python) - COPIED per-function for isolation
/// - Function code written to /function/code.{js,py}
/// - Runtime server auto-starting on boot
/// - Minimal resources (configurable vCPU and memory)
#[allow(clippy::too_many_arguments)]
pub async fn create_function_vm(
    st: &AppState,
    function_id: Uuid,
    function_name: &str,
    runtime: &str,
    _code: &str,
    _handler: &str,
    vcpu: u8,
    memory_mb: u32,
    _env_vars: &Option<serde_json::Value>,
) -> Result<Uuid> {
    use tokio::process::Command;

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

    // Copy the base runtime image to a function-specific image
    let copy_status = Command::new("cp")
        .args([&base_rootfs_path, &function_rootfs_path])
        .status()
        .await
        .context("Failed to execute cp command")?;

    if !copy_status.success() {
        anyhow::bail!(
            "Failed to copy runtime image from {} to {}",
            base_rootfs_path,
            function_rootfs_path
        );
    }

    eprintln!(
        "[Function {}] Runtime image copied successfully",
        function_id
    );

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
        tags: vec!["type:function".to_string()],
        rootfs_size_mb: None,
        network_id: None,
    };

    // Create and start VM
    crate::features::vms::service::create_and_start(st, vm_id, vm_req, None, None, "system")
        .await?;

    // Note: Function code will be injected after VM boots and guest IP is available
    // This is done in the service layer via the update_function_code() function

    Ok(vm_id)
}

/// Get kernel and rootfs paths for a given runtime
fn get_runtime_image_paths(runtime: &str) -> Result<(String, String)> {
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
        "python" => "/srv/images/python-runtime.ext4",
        "javascript" | "typescript" => "/srv/images/bun-runtime.ext4",
        _ => anyhow::bail!(
            "Unsupported runtime: {}. Supported: python, javascript, typescript",
            runtime
        ),
    };

    Ok((kernel, rootfs.to_string()))
}

/// Inject function code into the VM's rootfs before it starts
///
/// This mounts the rootfs, writes the function code, handler config, and env vars,
/// then unmounts it.
#[allow(dead_code)]
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
        .args(["mount", "-o", "loop", rootfs_path, &mount_point])
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
        let _ = Command::new("sudo").args(["umount", &mount_point]).output();
        let _ = fs::remove_dir_all(&mount_point);
    };

    // Write function code
    let file_extension = match runtime {
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        _ => {
            cleanup();
            anyhow::bail!("Unsupported runtime: {}", runtime);
        }
    };

    let code_path = format!("{}/function/code.{}", mount_point, file_extension);

    // For Bun/JavaScript/TypeScript, we use ES modules natively
    let code_content = code.to_string();

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
                if attempt >= max_attempts {
                    eprintln!("[CodeInjection] ERROR DETAILS: {:#?}", e);
                    anyhow::bail!(
                        "Failed to call /write-code endpoint after {} attempts: {:?}",
                        max_attempts,
                        e
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
