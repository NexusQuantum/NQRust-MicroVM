use anyhow::*;
use tokio::process::Command;

/// Spawn firecracker under a transient systemd scope so it is tracked and killed on stop.
pub async fn spawn_fc_scope(unit: &str, sock: &str) -> Result<()> {
    spawn_fc_scope_with_screen(unit, sock, None).await
}

/// Spawn firecracker inside a screen session for console access
/// The screen session name will be the same as the unit name (e.g., "fc-{vm-id}")
pub async fn spawn_fc_scope_with_screen(unit: &str, sock: &str, screen_name: Option<&str>) -> Result<()> {
    // Ensure parent dir exists is done by caller.
    let session_name = screen_name.unwrap_or(unit);

    // Use screen to create a detached session with a PTY for interactive console
    // The screen session allows us to attach to Firecracker's stdin/stdout later
    let status = Command::new("sudo")
        .args([
            "systemd-run",
            "--scope",
            "--unit",
            unit,
            "--property",
            "KillMode=mixed",
            "--property",
            "TimeoutStopSec=5s",
            "--",
            "screen",
            "-dmS",  // Create detached session with name
            session_name,
            "firecracker",
            "--api-sock",
            sock,
        ])
        .status()
        .await?;
    ensure!(status.success(), "systemd-run failed for firecracker with screen");
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
