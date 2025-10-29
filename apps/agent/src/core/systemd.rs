use anyhow::*;
use tokio::process::Command;

/// Spawn firecracker under a transient systemd scope so it is tracked and killed on stop.
pub async fn spawn_fc_scope(unit: &str, sock: &str) -> Result<()> {
    spawn_fc_scope_with_screen(unit, sock, None).await
}

/// Spawn firecracker with optional snapshot restore support
/// Note: Snapshots are loaded via API after firecracker starts, not via command-line
pub async fn spawn_fc_scope_with_snapshot(
    unit: &str,
    sock: &str,
    _snapshot_path: Option<&str>,
    _mem_path: Option<&str>,
) -> Result<()> {
    let session_name = unit;

    // Firecracker doesn't support --snapshot-path command-line args in many versions
    // Snapshots should be loaded via /snapshot/load API after firecracker starts
    // For now, just start firecracker normally
    let fc_args = vec!["firecracker", "--api-sock", sock];

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
            "-dmS",
            session_name,
        ])
        .args(&fc_args)
        .status()
        .await?;
    ensure!(
        status.success(),
        "systemd-run failed for firecracker with screen"
    );
    Ok(())
}

/// Spawn firecracker inside a screen session for console access
/// The screen session name will be the same as the unit name (e.g., "fc-{vm-id}")
pub async fn spawn_fc_scope_with_screen(
    unit: &str,
    sock: &str,
    screen_name: Option<&str>,
) -> Result<()> {
    spawn_fc_scope_with_snapshot(unit, sock, None, None).await
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
