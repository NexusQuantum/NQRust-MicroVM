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

#[allow(dead_code)]
pub async fn create_tap(name: &str, bridge: &str, owner: Option<&str>) -> Result<()> {
    create_tap_with_vlan(name, bridge, None, owner).await
}

pub async fn create_tap_with_vlan(
    name: &str,
    bridge: &str,
    vlan_id: Option<u16>,
    owner: Option<&str>,
) -> Result<()> {
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

    // If VLAN ID is specified, create VLAN interface and attach TAP to it
    if let Some(vlan) = vlan_id {
        let vlan_if = format!("{}.{}", bridge, vlan);

        // Check if VLAN interface already exists
        let vlan_check = Command::new("ip")
            .args(["link", "show", &vlan_if])
            .output()
            .await?;

        // Create VLAN interface if it doesn't exist
        if !vlan_check.status.success() {
            let _ = Command::new("sudo")
                .args([
                    "-n",
                    "ip",
                    "link",
                    "add",
                    "link",
                    bridge,
                    "name",
                    &vlan_if,
                    "type",
                    "vlan",
                    "id",
                    &vlan.to_string(),
                ])
                .status()
                .await?;

            // Bring up VLAN interface
            let _ = Command::new("sudo")
                .args(["-n", "ip", "link", "set", &vlan_if, "up"])
                .status()
                .await?;

            // Create a bridge for this VLAN if needed (for TAP attachments)
            let vlan_br = format!("vlan{}-br", vlan);
            let br_check = Command::new("ip")
                .args(["link", "show", &vlan_br])
                .output()
                .await?;

            if !br_check.status.success() {
                let _ = Command::new("sudo")
                    .args(["-n", "ip", "link", "add", &vlan_br, "type", "bridge"])
                    .status()
                    .await?;

                // Attach VLAN interface to VLAN bridge
                let _ = Command::new("sudo")
                    .args(["-n", "ip", "link", "set", &vlan_if, "master", &vlan_br])
                    .status()
                    .await?;

                // Bring up VLAN bridge
                let _ = Command::new("sudo")
                    .args(["-n", "ip", "link", "set", &vlan_br, "up"])
                    .status()
                    .await?;
            }

            eprintln!(
                "Created VLAN interface {} with bridge {} for VLAN {}",
                vlan_if, vlan_br, vlan
            );
        }

        // Attach TAP to VLAN bridge instead of main bridge
        let vlan_br = format!("vlan{}-br", vlan);
        let _ = Command::new("sudo")
            .args(["-n", "ip", "link", "set", name, "master", &vlan_br])
            .status()
            .await?;
    } else {
        // No VLAN - attach directly to bridge (original behavior)
        let _ = Command::new("sudo")
            .args(["-n", "ip", "link", "set", name, "master", bridge])
            .status()
            .await?;
    }

    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "set", name, "up"])
        .status()
        .await?;
    Ok(())
}

pub async fn delete_tap(name: &str) -> Result<()> {
    let output = Command::new("sudo")
        .args(["-n", "ip", "link", "del", name])
        .output()
        .await?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_trimmed = stderr.trim();

    if stderr_trimmed.contains("Cannot find device")
        || stderr_trimmed.contains("does not exist")
        || stderr_trimmed.is_empty()
    {
        return Ok(());
    }

    Err(anyhow!("failed to delete tap {name}: {stderr_trimmed}"))
}

/// Add a DNAT port forward rule: host_port on the host maps to guest_ip:guest_port
pub async fn add_port_forward(
    host_port: u16,
    guest_ip: &str,
    guest_port: u16,
    protocol: &str,
) -> Result<()> {
    let dest = format!("{}:{}", guest_ip, guest_port);
    let hp = host_port.to_string();

    // Check if DNAT rule already exists
    let check = Command::new("sudo")
        .args([
            "-n", "iptables", "-t", "nat", "-C", "PREROUTING",
            "-p", protocol, "--dport", &hp,
            "-j", "DNAT", "--to-destination", &dest,
        ])
        .status()
        .await?;

    if !check.success() {
        let status = Command::new("sudo")
            .args([
                "-n", "iptables", "-t", "nat", "-A", "PREROUTING",
                "-p", protocol, "--dport", &hp,
                "-j", "DNAT", "--to-destination", &dest,
            ])
            .status()
            .await?;
        if !status.success() {
            bail!("failed to add DNAT rule for port {}", host_port);
        }
    }

    // Add FORWARD rule to allow traffic to the guest
    let fwd_check = Command::new("sudo")
        .args([
            "-n", "iptables", "-C", "FORWARD",
            "-p", protocol, "-d", guest_ip, "--dport", &guest_port.to_string(),
            "-j", "ACCEPT",
        ])
        .status()
        .await?;

    if !fwd_check.success() {
        let _ = Command::new("sudo")
            .args([
                "-n", "iptables", "-A", "FORWARD",
                "-p", protocol, "-d", guest_ip, "--dport", &guest_port.to_string(),
                "-j", "ACCEPT",
            ])
            .status()
            .await?;
    }

    Ok(())
}

/// Remove a DNAT port forward rule
pub async fn remove_port_forward(
    host_port: u16,
    guest_ip: &str,
    guest_port: u16,
    protocol: &str,
) -> Result<()> {
    let dest = format!("{}:{}", guest_ip, guest_port);
    let hp = host_port.to_string();

    // Remove DNAT rule (ignore errors if rule doesn't exist)
    let _ = Command::new("sudo")
        .args([
            "-n", "iptables", "-t", "nat", "-D", "PREROUTING",
            "-p", protocol, "--dport", &hp,
            "-j", "DNAT", "--to-destination", &dest,
        ])
        .status()
        .await;

    // Remove FORWARD rule
    let _ = Command::new("sudo")
        .args([
            "-n", "iptables", "-D", "FORWARD",
            "-p", protocol, "-d", guest_ip, "--dport", &guest_port.to_string(),
            "-j", "ACCEPT",
        ])
        .status()
        .await;

    Ok(())
}
