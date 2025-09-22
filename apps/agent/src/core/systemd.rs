use anyhow::*;
use tokio::process::Command;

/// Spawn firecracker under a transient systemd scope so it is tracked and killed on stop.
pub async fn spawn_fc_scope(unit: &str, sock: &str) -> Result<()> {
    // Ensure parent dir exists is done by caller.
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
            "firecracker",
            "--api-sock",
            sock,
        ])
        .status()
        .await?;
    ensure!(status.success(), "systemd-run failed for firecracker");
    Ok(())
}

pub async fn stop_unit(unit: &str) -> Result<()> {
    let _ = Command::new("sudo")
        .args(["systemctl", "stop", unit])
        .status()
        .await?;
    Ok(())
}
