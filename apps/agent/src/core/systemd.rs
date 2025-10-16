use anyhow::*;
use tokio::process::Command;

/// Spawn firecracker under a transient systemd scope so it is tracked and killed on stop.
pub async fn spawn_fc_scope(unit: &str, sock: &str) -> Result<()> {
    spawn_fc_scope_with_console(unit, sock, None).await
}

/// Spawn firecracker with optional console socket support
pub async fn spawn_fc_scope_with_console(unit: &str, sock: &str, console_sock: Option<&str>) -> Result<()> {
    // Ensure parent dir exists is done by caller.
    let mut args = vec![
        "systemd-run",
        "--scope",
        "--unit",
        unit,
        "--property",
        "KillMode=mixed",
        "--property",
        "TimeoutStopSec=5s",
        "--",
        "firecracker",
        "--api-sock",
        sock,
    ];

    // Add console socket if provided
    if let Some(console) = console_sock {
        args.push("--console-sock");
        args.push(console);
    }

    let status = Command::new("sudo")
        .args(&args)
        .status()
        .await?;
    ensure!(status.success(), "systemd-run failed for firecracker");
    Ok(())
}

pub async fn stop_unit(unit: &str) -> Result<()> {
    let output = Command::new("sudo")
        .args(["-n", "systemctl", "stop", unit])
        .output()
        .await?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_trimmed = stderr.trim();
    if stderr_trimmed.contains("not loaded") || stderr_trimmed.contains("could not be found") {
        return Ok(());
    }

    Err(anyhow!(
        "failed to stop systemd unit {unit}: {stderr_trimmed}"
    ))
}
