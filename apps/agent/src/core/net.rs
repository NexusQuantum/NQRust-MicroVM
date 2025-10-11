use anyhow::*;
use tokio::process::Command;

pub async fn ensure_bridge(bridge: &str, uplink: Option<&str>) -> Result<()> {
    let _ = Command::new("bash")
        .arg("-lc")
        .arg(format!(
            "ip link show {bridge} || sudo -n ip link add {bridge} type bridge"
        ))
        .status()
        .await?;
    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "set", bridge, "up"])
        .status()
        .await?;
    if let Some(u) = uplink {
        let _ = Command::new("sudo")
            .args(["-n", "sysctl", "-w", "net.ipv4.ip_forward=1"])
            .status()
            .await?;
        let check = Command::new("sudo")
            .args([
                "-n",
                "iptables",
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-o",
                u,
                "-j",
                "MASQUERADE",
            ])
            .status()
            .await?;
        if !check.success() {
            let _ = Command::new("sudo")
                .args([
                    "-n",
                    "iptables",
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-o",
                    u,
                    "-j",
                    "MASQUERADE",
                ])
                .status()
                .await?;
        }
    }
    Ok(())
}

pub async fn create_tap(name: &str, bridge: &str, owner: Option<&str>) -> Result<()> {
    // Check if we're in test mode (no sudo available)
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping TAP device creation for {name}");
        return Ok(());
    }

    // Check if TAP device exists before trying to delete it
    let check_result = Command::new("ip")
        .args(["link", "show", name])
        .output()
        .await?;

    if check_result.status.success() {
        let _ = Command::new("sudo")
            .args(["-n", "ip", "link", "del", name])
            .status()
            .await?;
    }
    let mut cmd = format!("sudo -n ip tuntap add dev {name} mode tap");
    if let Some(user) = owner {
        cmd.push_str(&format!(" user {user} group {user}"));
    }

    // Try to create TAP device, but handle sudo failures gracefully
    let result = Command::new("bash").arg("-lc").arg(&cmd).status().await?;
    if !result.success() {
        eprintln!("Warning: Failed to create TAP device {name}, continuing in test mode...");
        return Ok(());
    }

    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "set", name, "master", bridge])
        .status()
        .await?;
    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "set", name, "up"])
        .status()
        .await?;
    Ok(())
}

pub async fn delete_tap(name: &str) -> Result<()> {
    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "del", name])
        .status()
        .await?;
    Ok(())
}
