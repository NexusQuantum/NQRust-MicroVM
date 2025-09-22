use anyhow::*;
use tokio::process::Command;

pub async fn ensure_bridge(bridge: &str, uplink: Option<&str>) -> Result<()> {
    let _ = Command::new("bash")
        .arg("-lc")
        .arg(format!(
            "ip link show {bridge} || sudo ip link add {bridge} type bridge"
        ))
        .status()
        .await?;
    let _ = Command::new("sudo")
        .args(["ip", "link", "set", bridge, "up"])
        .status()
        .await?;
    if let Some(u) = uplink {
        let _ = Command::new("sudo")
            .args(["sysctl", "-w", "net.ipv4.ip_forward=1"])
            .status()
            .await?;
        let check = Command::new("sudo")
            .args([
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
    let _ = Command::new("bash")
        .arg("-lc")
        .arg(format!(
            "ip link show {name} && sudo ip link del {name} || true"
        ))
        .status()
        .await?;
    let mut cmd = format!("sudo ip tuntap add dev {name} mode tap");
    if let Some(user) = owner {
        cmd.push_str(&format!(" user {user} group {user}"));
    }
    let _ = Command::new("bash").arg("-lc").arg(cmd).status().await?;
    let _ = Command::new("sudo")
        .args(["ip", "link", "set", name, "master", bridge])
        .status()
        .await?;
    let _ = Command::new("sudo")
        .args(["ip", "link", "set", name, "up"])
        .status()
        .await?;
    Ok(())
}

pub async fn delete_tap(name: &str) -> Result<()> {
    let _ = Command::new("sudo")
        .args(["ip", "link", "del", name])
        .status()
        .await?;
    Ok(())
}
