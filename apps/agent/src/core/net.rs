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
            "-n",
            "iptables",
            "-t",
            "nat",
            "-C",
            "PREROUTING",
            "-p",
            protocol,
            "--dport",
            &hp,
            "-j",
            "DNAT",
            "--to-destination",
            &dest,
        ])
        .status()
        .await?;

    if !check.success() {
        let status = Command::new("sudo")
            .args([
                "-n",
                "iptables",
                "-t",
                "nat",
                "-A",
                "PREROUTING",
                "-p",
                protocol,
                "--dport",
                &hp,
                "-j",
                "DNAT",
                "--to-destination",
                &dest,
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
            "-n",
            "iptables",
            "-C",
            "FORWARD",
            "-p",
            protocol,
            "-d",
            guest_ip,
            "--dport",
            &guest_port.to_string(),
            "-j",
            "ACCEPT",
        ])
        .status()
        .await?;

    if !fwd_check.success() {
        let _ = Command::new("sudo")
            .args([
                "-n",
                "iptables",
                "-A",
                "FORWARD",
                "-p",
                protocol,
                "-d",
                guest_ip,
                "--dport",
                &guest_port.to_string(),
                "-j",
                "ACCEPT",
            ])
            .status()
            .await?;
    }

    Ok(())
}

/// Provision a NAT network: bridge + IP + masquerade + dnsmasq DHCP
pub async fn provision_nat_network(
    bridge: &str,
    cidr: &str,
    gateway: &str,
    dhcp_enabled: bool,
    dhcp_start: &str,
    dhcp_end: &str,
) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping NAT network provisioning for {bridge}");
        return Ok(());
    }

    // Create bridge
    create_bridge(bridge).await?;

    // Assign IP (idempotent: replace instead of add)
    let prefix = cidr.split('/').nth(1).unwrap_or("24");
    let gw_cidr = format!("{}/{}", gateway, prefix);
    run_cmd("ip", &["addr", "replace", &gw_cidr, "dev", bridge]).await?;

    // Enable IP forwarding
    run_cmd("sysctl", &["-w", "net.ipv4.ip_forward=1"]).await?;

    // Detect default outgoing interface
    let default_iface = detect_default_interface().await?;

    // Add masquerade + forward rules (idempotent: -C check before -A)
    ensure_iptables_rule(
        "nat",
        "POSTROUTING",
        &["-s", cidr, "-o", &default_iface, "-j", "MASQUERADE"],
    )
    .await?;
    ensure_iptables_rule(
        "filter",
        "FORWARD",
        &["-i", bridge, "-o", &default_iface, "-j", "ACCEPT"],
    )
    .await?;
    ensure_iptables_rule(
        "filter",
        "FORWARD",
        &[
            "-i",
            &default_iface,
            "-o",
            bridge,
            "-m",
            "state",
            "--state",
            "RELATED,ESTABLISHED",
            "-j",
            "ACCEPT",
        ],
    )
    .await?;

    // Write dnsmasq config and reload (only if DHCP enabled)
    if dhcp_enabled {
        write_dnsmasq_config(bridge, dhcp_start, dhcp_end, Some(gateway)).await?;
        reload_dnsmasq().await?;
    }

    // Write systemd service for boot persistence
    write_network_service(
        bridge,
        "nat",
        &NetworkServiceParams {
            gateway_cidr: Some(gw_cidr),
            cidr: Some(cidr.to_string()),
            default_iface: Some(default_iface),
            is_gateway: false,
            vni: None,
            local_ip: None,
        },
    )
    .await;

    Ok(())
}

/// Provision an isolated network: bridge + IP + dnsmasq DHCP, no internet
pub async fn provision_isolated_network(
    bridge: &str,
    cidr: &str,
    gateway: &str,
    dhcp_enabled: bool,
    dhcp_start: &str,
    dhcp_end: &str,
) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping isolated network provisioning for {bridge}");
        return Ok(());
    }

    // Create bridge
    create_bridge(bridge).await?;

    // Assign IP (idempotent: replace instead of add)
    let prefix = cidr.split('/').nth(1).unwrap_or("24");
    let gw_cidr = format!("{}/{}", gateway, prefix);
    run_cmd("ip", &["addr", "replace", &gw_cidr, "dev", bridge]).await?;

    // No IP forwarding, no masquerade — isolated
    // Write dnsmasq config WITHOUT router option (only if DHCP enabled)
    if dhcp_enabled {
        write_dnsmasq_config(bridge, dhcp_start, dhcp_end, None).await?;
        reload_dnsmasq().await?;
    }

    // Write systemd service for boot persistence
    write_network_service(
        bridge,
        "isolated",
        &NetworkServiceParams {
            gateway_cidr: Some(gw_cidr),
            cidr: None,
            default_iface: None,
            is_gateway: false,
            vni: None,
            local_ip: None,
        },
    )
    .await;

    Ok(())
}

/// Teardown a provisioned network: remove iptables, dnsmasq, bridge
pub async fn teardown_network(network_type: &str, bridge: &str, cidr: &str) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping network teardown for {bridge}");
        return Ok(());
    }

    // Remove dnsmasq config
    let conf_path = format!("/etc/dnsmasq.d/nqrust-{}.conf", bridge);
    let _ = tokio::fs::remove_file(&conf_path).await;
    let _ = reload_dnsmasq().await;

    // Remove iptables rules if NAT
    if network_type == "nat" {
        if let std::result::Result::Ok(default_iface) = detect_default_interface().await {
            // Ignore errors — rules may not exist
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-t",
                    "nat",
                    "-D",
                    "POSTROUTING",
                    "-s",
                    cidr,
                    "-o",
                    &default_iface,
                    "-j",
                    "MASQUERADE",
                ],
            )
            .await;
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-D",
                    "FORWARD",
                    "-i",
                    bridge,
                    "-o",
                    &default_iface,
                    "-j",
                    "ACCEPT",
                ],
            )
            .await;
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-D",
                    "FORWARD",
                    "-i",
                    &default_iface,
                    "-o",
                    bridge,
                    "-m",
                    "state",
                    "--state",
                    "RELATED,ESTABLISHED",
                    "-j",
                    "ACCEPT",
                ],
            )
            .await;
        }
    }

    // Delete bridge
    let _ = run_cmd_ignore("ip", &["link", "set", bridge, "down"]).await;
    let _ = run_cmd_ignore("ip", &["link", "del", bridge]).await;

    // Remove systemd boot persistence service
    remove_network_service(bridge).await;

    Ok(())
}

/// List physical network interfaces available for bridging.
pub async fn list_interfaces() -> Result<Vec<serde_json::Value>> {
    let default_iface = detect_default_interface().await.unwrap_or_default();

    // Get all interfaces via `ip -j link show`
    let output = Command::new("ip")
        .args(["-j", "link", "show"])
        .output()
        .await
        .context("failed to run ip link show")?;
    let links: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap_or_default();

    // Get addresses via `ip -j addr show`
    let addr_output = Command::new("ip")
        .args(["-j", "addr", "show"])
        .output()
        .await
        .context("failed to run ip addr show")?;
    let addrs: Vec<serde_json::Value> =
        serde_json::from_slice(&addr_output.stdout).unwrap_or_default();

    // Build address map: iface_name -> [addr/prefix, ...]
    let mut addr_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for entry in &addrs {
        let name = entry["ifname"].as_str().unwrap_or_default().to_string();
        if let Some(addr_info) = entry["addr_info"].as_array() {
            for ai in addr_info {
                if let (Some(addr), Some(prefix)) = (ai["local"].as_str(), ai["prefixlen"].as_u64())
                {
                    addr_map
                        .entry(name.clone())
                        .or_default()
                        .push(format!("{}/{}", addr, prefix));
                }
            }
        }
    }

    let mut result = Vec::new();
    for link in &links {
        let name = link["ifname"].as_str().unwrap_or_default();
        let state = link["operstate"].as_str().unwrap_or("UNKNOWN");
        let link_type = link["link_type"].as_str().unwrap_or("");
        let mac = link["address"].as_str().unwrap_or("");

        // Skip: loopback, virtual interfaces, existing bridges/taps/veths
        if name == "lo"
            || name.starts_with("veth")
            || name.starts_with("docker")
            || name.starts_with("br-")
            || name.starts_with("virbr")
            || name.starts_with("tap")
            || name.starts_with("nqbr")
            || name.starts_with("fcbr")
            || link_type == "loopback"
        {
            continue;
        }

        // Check if it's a bridge itself
        let is_bridge = link.get("linkinfo").and_then(|li| li.get("info_kind"))
            == Some(&serde_json::Value::String("bridge".to_string()));
        if is_bridge {
            continue;
        }

        // Check if already enslaved to a bridge
        let master = link.get("master").and_then(|m| m.as_str());
        let is_management = name == default_iface;
        let addresses = addr_map.get(name).cloned().unwrap_or_default();

        result.push(serde_json::json!({
            "name": name,
            "mac": mac,
            "state": state,
            "addresses": addresses,
            "is_management": is_management,
            "master": master,
        }));
    }

    Ok(result)
}

/// Provision a bridged network: create bridge and attach a physical NIC.
/// The external network (router/switch) provides DHCP and routing.
pub async fn provision_bridged_network(bridge: &str, uplink_interface: &str) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping bridged network provisioning for {bridge}");
        return Ok(());
    }

    // Safety: refuse to bridge the management interface
    let default_iface = detect_default_interface().await.unwrap_or_default();
    if uplink_interface == default_iface {
        return Err(anyhow!(
            "cannot bridge management interface '{}' — this would break agent connectivity. \
             Use the installer for management interface bridging, or select a secondary NIC.",
            uplink_interface
        ));
    }

    // Verify the interface exists
    let check = Command::new("ip")
        .args(["link", "show", uplink_interface])
        .output()
        .await?;
    if !check.status.success() {
        return Err(anyhow!(
            "interface '{}' not found on this host",
            uplink_interface
        ));
    }

    // Create bridge
    create_bridge(bridge).await?;

    // Attach physical NIC to bridge
    run_cmd("ip", &["link", "set", uplink_interface, "master", bridge])
        .await
        .with_context(|| format!("failed to attach {} to bridge {}", uplink_interface, bridge))?;

    // Ensure the uplink is up
    run_cmd("ip", &["link", "set", uplink_interface, "up"]).await?;

    Ok(())
}

/// Teardown a bridged network: detach the uplink NIC and delete the bridge.
pub async fn teardown_bridged_network(bridge: &str) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping bridged network teardown for {bridge}");
        return Ok(());
    }

    // Find any interfaces enslaved to this bridge and release them
    let output = Command::new("ip")
        .args(["-j", "link", "show", "master", bridge])
        .output()
        .await;
    if let std::result::Result::Ok(out) = output {
        if let std::result::Result::Ok(slaves) =
            serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
        {
            for slave in &slaves {
                if let Some(name) = slave["ifname"].as_str() {
                    // Skip tap devices (VM interfaces) — they'll be cleaned up by VM lifecycle
                    if !name.starts_with("tap") && !name.starts_with("veth") {
                        let _ = run_cmd_ignore("ip", &["link", "set", name, "nomaster"]).await;
                    }
                }
            }
        }
    }

    // Delete bridge
    let _ = run_cmd_ignore("ip", &["link", "set", bridge, "down"]).await;
    let _ = run_cmd_ignore("ip", &["link", "del", bridge]).await;

    Ok(())
}

/// Provision a VXLAN overlay network: VXLAN interface + bridge, optionally gateway (NAT + DHCP).
#[allow(clippy::too_many_arguments)]
pub async fn provision_vxlan_network(
    bridge: &str,
    vni: u32,
    local_ip: &str,
    cidr: &str,
    gateway: &str,
    is_gateway: bool,
    dhcp_enabled: bool,
    dhcp_start: &str,
    dhcp_end: &str,
) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping VXLAN network provisioning for {bridge}");
        return Ok(());
    }

    let vxlan_dev = format!("vxlan{}", vni);

    // Create VXLAN interface (idempotent: check if exists first)
    let vxlan_exists = Command::new("ip")
        .args(["link", "show", &vxlan_dev])
        .output()
        .await?;
    if !vxlan_exists.status.success() {
        run_cmd(
            "ip",
            &[
                "link",
                "add",
                &vxlan_dev,
                "type",
                "vxlan",
                "id",
                &vni.to_string(),
                "dstport",
                "4789",
                "local",
                local_ip,
                "nolearning",
            ],
        )
        .await
        .with_context(|| format!("failed to create VXLAN interface {vxlan_dev}"))?;
    }

    // Create bridge
    create_bridge(bridge).await?;

    // Attach VXLAN interface to bridge
    run_cmd("ip", &["link", "set", &vxlan_dev, "master", bridge]).await?;
    run_cmd("ip", &["link", "set", &vxlan_dev, "up"]).await?;

    let mut svc_gw_cidr = None;
    let mut svc_default_iface = None;

    if is_gateway {
        // Assign gateway IP to bridge (idempotent: replace instead of add)
        let prefix = cidr.split('/').nth(1).unwrap_or("24");
        let gw_cidr = format!("{}/{}", gateway, prefix);
        run_cmd("ip", &["addr", "replace", &gw_cidr, "dev", bridge]).await?;

        // Enable IP forwarding
        run_cmd("sysctl", &["-w", "net.ipv4.ip_forward=1"]).await?;

        // Detect default outgoing interface
        let default_iface = detect_default_interface().await?;

        // Add masquerade + forward rules (idempotent: -C check before -A)
        ensure_iptables_rule(
            "nat",
            "POSTROUTING",
            &["-s", cidr, "-o", &default_iface, "-j", "MASQUERADE"],
        )
        .await?;
        ensure_iptables_rule(
            "filter",
            "FORWARD",
            &["-i", bridge, "-o", &default_iface, "-j", "ACCEPT"],
        )
        .await?;
        ensure_iptables_rule(
            "filter",
            "FORWARD",
            &[
                "-i",
                &default_iface,
                "-o",
                bridge,
                "-m",
                "state",
                "--state",
                "RELATED,ESTABLISHED",
                "-j",
                "ACCEPT",
            ],
        )
        .await?;

        // Write dnsmasq config with MTU 1450 for VXLAN overhead
        if dhcp_enabled {
            write_vxlan_dnsmasq_config(bridge, dhcp_start, dhcp_end, gateway).await?;
            reload_dnsmasq().await?;
        }

        svc_gw_cidr = Some(gw_cidr);
        svc_default_iface = Some(default_iface);
    }

    // Write systemd service for boot persistence
    write_network_service(
        bridge,
        "vxlan",
        &NetworkServiceParams {
            gateway_cidr: svc_gw_cidr,
            cidr: if is_gateway {
                Some(cidr.to_string())
            } else {
                None
            },
            default_iface: svc_default_iface,
            is_gateway,
            vni: Some(vni),
            local_ip: Some(local_ip.to_string()),
        },
    )
    .await;

    Ok(())
}

/// Add a VXLAN VTEP peer via FDB entry (for BUM flooding).
/// Idempotent: checks if the FDB entry already exists before appending.
pub async fn add_vxlan_peer(vni: u32, peer_ip: &str) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        return Ok(());
    }
    let vxlan_dev = format!("vxlan{}", vni);

    // Check if FDB entry already exists
    let output = Command::new("bridge")
        .args(["fdb", "show", "dev", &vxlan_dev])
        .output()
        .await?;
    let fdb_out = String::from_utf8_lossy(&output.stdout);
    if fdb_out.lines().any(|line| {
        line.contains("00:00:00:00:00:00") && line.contains(&format!("dst {}", peer_ip))
    }) {
        return Ok(()); // peer already exists
    }

    run_cmd(
        "bridge",
        &[
            "fdb",
            "append",
            "00:00:00:00:00:00",
            "dev",
            &vxlan_dev,
            "dst",
            peer_ip,
        ],
    )
    .await
    .with_context(|| format!("failed to add VXLAN peer {peer_ip} on {vxlan_dev}"))
}

/// Remove a VXLAN VTEP peer FDB entry.
pub async fn remove_vxlan_peer(vni: u32, peer_ip: &str) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        return Ok(());
    }
    let vxlan_dev = format!("vxlan{}", vni);
    let _ = run_cmd_ignore(
        "bridge",
        &[
            "fdb",
            "del",
            "00:00:00:00:00:00",
            "dev",
            &vxlan_dev,
            "dst",
            peer_ip,
        ],
    )
    .await;
    Ok(())
}

/// Teardown a VXLAN network: remove NAT/DHCP (if gateway), delete bridge + vxlan interface.
pub async fn teardown_vxlan_network(
    bridge: &str,
    vni: u32,
    cidr: &str,
    is_gateway: bool,
) -> Result<()> {
    if std::env::var("AGENT_TEST_MODE").is_ok() {
        eprintln!("AGENT_TEST_MODE: Skipping VXLAN network teardown for {bridge}");
        return Ok(());
    }

    // If gateway, remove dnsmasq + iptables
    if is_gateway {
        let conf_path = format!("/etc/dnsmasq.d/nqrust-{}.conf", bridge);
        let _ = tokio::fs::remove_file(&conf_path).await;
        let _ = reload_dnsmasq().await;

        if let std::result::Result::Ok(default_iface) = detect_default_interface().await {
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-t",
                    "nat",
                    "-D",
                    "POSTROUTING",
                    "-s",
                    cidr,
                    "-o",
                    &default_iface,
                    "-j",
                    "MASQUERADE",
                ],
            )
            .await;
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-D",
                    "FORWARD",
                    "-i",
                    bridge,
                    "-o",
                    &default_iface,
                    "-j",
                    "ACCEPT",
                ],
            )
            .await;
            let _ = run_cmd_ignore(
                "iptables",
                &[
                    "-D",
                    "FORWARD",
                    "-i",
                    &default_iface,
                    "-o",
                    bridge,
                    "-m",
                    "state",
                    "--state",
                    "RELATED,ESTABLISHED",
                    "-j",
                    "ACCEPT",
                ],
            )
            .await;
        }
    }

    // Delete bridge (this also removes enslaved vxlan interface)
    let _ = run_cmd_ignore("ip", &["link", "set", bridge, "down"]).await;
    let _ = run_cmd_ignore("ip", &["link", "del", bridge]).await;

    // Explicitly delete vxlan device in case it wasn't attached
    let vxlan_dev = format!("vxlan{}", vni);
    let _ = run_cmd_ignore("ip", &["link", "del", &vxlan_dev]).await;

    // Remove systemd boot persistence service
    remove_network_service(bridge).await;

    Ok(())
}

/// Check if a network's infrastructure exists on the host
pub async fn check_network_status(bridge: &str) -> Result<serde_json::Value> {
    let bridge_exists = Command::new("ip")
        .args(["link", "show", bridge])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let conf_path = format!("/etc/dnsmasq.d/nqrust-{}.conf", bridge);
    let dnsmasq_configured = tokio::fs::metadata(&conf_path).await.is_ok();

    Ok(serde_json::json!({
        "bridge_exists": bridge_exists,
        "dnsmasq_configured": dnsmasq_configured,
    }))
}

// --- Helper functions ---

async fn create_bridge(bridge: &str) -> Result<()> {
    // Ensure NetworkManager won't interfere with our bridges
    ensure_nm_unmanaged().await;

    // Check if bridge already exists
    let check = Command::new("ip")
        .args(["link", "show", bridge])
        .output()
        .await?;
    if !check.status.success() {
        run_cmd("ip", &["link", "add", bridge, "type", "bridge"]).await?;
    }
    run_cmd("ip", &["link", "set", bridge, "up"]).await?;
    Ok(())
}

async fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let mut full_args = vec!["-n", cmd];
    full_args.extend_from_slice(args);
    let output = Command::new("sudo").args(&full_args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "command `sudo {} {}` failed: {}",
            cmd,
            args.join(" "),
            stderr.trim()
        );
    }
    Ok(())
}

async fn run_cmd_ignore(cmd: &str, args: &[&str]) -> Result<()> {
    let mut full_args = vec!["-n", cmd];
    full_args.extend_from_slice(args);
    let _ = Command::new("sudo").args(&full_args).output().await;
    Ok(())
}

/// Ensure NetworkManager ignores NQRust-managed interfaces (bridges, taps, VXLAN).
/// No-op if NM is not running or config already exists.
async fn ensure_nm_unmanaged() {
    let conf_path = "/etc/NetworkManager/conf.d/99-nqrust-unmanaged.conf";
    if tokio::fs::metadata(conf_path).await.is_ok() {
        return; // already configured
    }

    let nm_active = Command::new("systemctl")
        .args(["is-active", "NetworkManager"])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);

    if !nm_active {
        return;
    }

    let conf = "[keyfile]\nunmanaged-devices=interface-name:fcbr*;interface-name:nqbr*;interface-name:br-vx*;interface-name:tap-*;interface-name:vxlan*\n";
    if std::path::Path::new("/etc/NetworkManager/conf.d").exists() {
        let _ = tokio::fs::write(conf_path, conf).await;
        let _ = Command::new("sudo")
            .args(["-n", "systemctl", "reload", "NetworkManager"])
            .status()
            .await;
    }
}

/// Write a systemd oneshot service that recreates a network bridge on boot.
/// This makes runtime-created networks survive host reboots without needing the manager.
async fn write_network_service(bridge: &str, network_type: &str, params: &NetworkServiceParams) {
    let service_name = format!("nqrust-{}.service", bridge);
    let service_path = format!("/etc/systemd/system/{}", service_name);

    let mut exec_lines = String::new();

    // VXLAN: create VXLAN interface first
    if let (Some(vni), Some(local_ip)) = (params.vni, params.local_ip.as_deref()) {
        let vxlan_dev = format!("vxlan{}", vni);
        exec_lines.push_str(&format!(
            "ExecStart=/bin/sh -c 'ip link show {vx} 2>/dev/null || ip link add {vx} type vxlan id {vni} dstport 4789 local {lip} nolearning'\n\
             ExecStart=/sbin/ip link set {vx} up\n",
            vx = vxlan_dev,
            vni = vni,
            lip = local_ip,
        ));
    }

    // Bridge: create + assign IP
    exec_lines.push_str(&format!(
        "ExecStart=/bin/sh -c 'ip link show {br} 2>/dev/null || ip link add {br} type bridge'\n\
         ExecStart=/sbin/ip link set {br} up\n",
        br = bridge,
    ));

    // VXLAN: attach VXLAN interface to bridge
    if let Some(vni) = params.vni {
        let vxlan_dev = format!("vxlan{}", vni);
        exec_lines.push_str(&format!(
            "ExecStart=/bin/sh -c 'ip link set {vx} master {br} 2>/dev/null || true'\n",
            vx = vxlan_dev,
            br = bridge,
        ));
    }

    // Assign gateway IP if present
    if let Some(ref gw_cidr) = params.gateway_cidr {
        exec_lines.push_str(&format!(
            "ExecStart=/bin/sh -c '/sbin/ip addr replace {} dev {}'\n",
            gw_cidr, bridge,
        ));
    }

    // NAT / VXLAN gateway: ip_forward + iptables
    if (network_type == "nat" || params.is_gateway)
        && params.default_iface.is_some()
        && params.cidr.is_some()
    {
        let ifc = params.default_iface.as_deref().unwrap();
        let sub = params.cidr.as_deref().unwrap();
        exec_lines.push_str(&format!(
            "ExecStart=/sbin/sysctl -w net.ipv4.ip_forward=1\n\
             ExecStart=/bin/sh -c 'iptables -t nat -C POSTROUTING -s {sub} -o {ifc} -j MASQUERADE || iptables -t nat -A POSTROUTING -s {sub} -o {ifc} -j MASQUERADE'\n\
             ExecStart=/bin/sh -c 'iptables -C FORWARD -i {br} -o {ifc} -j ACCEPT || iptables -A FORWARD -i {br} -o {ifc} -j ACCEPT'\n\
             ExecStart=/bin/sh -c 'iptables -C FORWARD -i {ifc} -o {br} -m state --state RELATED,ESTABLISHED -j ACCEPT || iptables -A FORWARD -i {ifc} -o {br} -m state --state RELATED,ESTABLISHED -j ACCEPT'\n",
            sub = sub,
            ifc = ifc,
            br = bridge,
        ));
    }

    let service_content = format!(
        "[Unit]\n\
         Description=NQRust Network {br}\n\
         After=network.target\n\
         Before=dnsmasq.service\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         RemainAfterExit=yes\n\
         {exec}\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        br = bridge,
        exec = exec_lines,
    );

    // Write service file and enable it
    let write_result = Command::new("sudo")
        .args(["-n", "tee", &service_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn();

    if let std::result::Result::Ok(mut child) = write_result {
        if let Some(ref mut stdin) = child.stdin {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(service_content.as_bytes()).await;
        }
        let _ = child.wait().await;
    }

    let _ = Command::new("sudo")
        .args(["-n", "systemctl", "daemon-reload"])
        .status()
        .await;
    let _ = Command::new("sudo")
        .args(["-n", "systemctl", "enable", &service_name])
        .status()
        .await;
}

/// Remove the systemd service for a runtime network (called on teardown).
async fn remove_network_service(bridge: &str) {
    let service_name = format!("nqrust-{}.service", bridge);
    let service_path = format!("/etc/systemd/system/{}", service_name);

    let _ = Command::new("sudo")
        .args(["-n", "systemctl", "disable", &service_name])
        .status()
        .await;
    let _ = Command::new("sudo")
        .args(["-n", "rm", "-f", &service_path])
        .status()
        .await;
    let _ = Command::new("sudo")
        .args(["-n", "systemctl", "daemon-reload"])
        .status()
        .await;
}

/// Parameters for writing a network systemd service.
struct NetworkServiceParams {
    gateway_cidr: Option<String>,
    cidr: Option<String>,
    default_iface: Option<String>,
    is_gateway: bool,
    vni: Option<u32>,
    local_ip: Option<String>,
}

/// Ensure an iptables rule exists (idempotent: check with -C before appending with -A).
async fn ensure_iptables_rule(table: &str, chain: &str, rule_args: &[&str]) -> Result<()> {
    let mut check_args = vec!["-n", "iptables", "-t", table, "-C", chain];
    check_args.extend_from_slice(rule_args);
    let check = Command::new("sudo").args(&check_args).status().await?;
    if !check.success() {
        let mut add_args = vec!["-n", "iptables", "-t", table, "-A", chain];
        add_args.extend_from_slice(rule_args);
        let status = Command::new("sudo").args(&add_args).status().await?;
        if !status.success() {
            bail!(
                "failed to add iptables rule: -t {} -A {} {}",
                table,
                chain,
                rule_args.join(" ")
            );
        }
    }
    Ok(())
}

async fn detect_default_interface() -> Result<String> {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse: "default via 10.0.0.1 dev eth0 ..."
    let iface = stdout
        .split_whitespace()
        .skip_while(|w| *w != "dev")
        .nth(1)
        .ok_or_else(|| anyhow!("could not detect default network interface"))?
        .to_string();
    Ok(iface)
}

async fn write_dnsmasq_config(
    bridge: &str,
    dhcp_start: &str,
    dhcp_end: &str,
    router_ip: Option<&str>,
) -> Result<()> {
    let mut config = format!(
        "# Auto-generated by NQRust agent for network {bridge}\n\
         interface={bridge}\n\
         bind-dynamic\n\
         port=0\n\
         dhcp-range={dhcp_start},{dhcp_end},12h\n"
    );
    if let Some(router) = router_ip {
        config.push_str(&format!("dhcp-option=option:router,{router}\n"));
        config.push_str("dhcp-option=option:dns-server,8.8.8.8,8.8.4.4,1.1.1.1\n");
    }

    // Ensure dnsmasq config directory exists (agent runs as root)
    let dir = std::path::Path::new("/etc/dnsmasq.d");
    if !dir.exists() {
        tokio::fs::create_dir_all(dir)
            .await
            .context("failed to create /etc/dnsmasq.d")?;
    }

    let conf_path = format!("/etc/dnsmasq.d/nqrust-{}.conf", bridge);
    tokio::fs::write(&conf_path, config.as_bytes())
        .await
        .with_context(|| format!("failed to write dnsmasq config to {conf_path}"))?;
    Ok(())
}

/// Write dnsmasq config for VXLAN networks with MTU 1450 to account for encapsulation overhead.
async fn write_vxlan_dnsmasq_config(
    bridge: &str,
    dhcp_start: &str,
    dhcp_end: &str,
    router_ip: &str,
) -> Result<()> {
    let config = format!(
        "# Auto-generated by NQRust agent for VXLAN network {bridge}\n\
         interface={bridge}\n\
         bind-dynamic\n\
         port=0\n\
         dhcp-range={dhcp_start},{dhcp_end},12h\n\
         dhcp-option=option:router,{router_ip}\n\
         dhcp-option=option:dns-server,8.8.8.8,8.8.4.4,1.1.1.1\n\
         dhcp-option=option:mtu,1450\n"
    );

    let dir = std::path::Path::new("/etc/dnsmasq.d");
    if !dir.exists() {
        tokio::fs::create_dir_all(dir)
            .await
            .context("failed to create /etc/dnsmasq.d")?;
    }

    let conf_path = format!("/etc/dnsmasq.d/nqrust-{}.conf", bridge);
    tokio::fs::write(&conf_path, config.as_bytes())
        .await
        .with_context(|| format!("failed to write dnsmasq config to {conf_path}"))?;
    Ok(())
}

async fn reload_dnsmasq() -> Result<()> {
    let output = Command::new("sudo")
        .args(["-n", "systemctl", "reload", "dnsmasq"])
        .output()
        .await?;
    if !output.status.success() {
        // Fallback to restart if reload fails
        let _ = Command::new("sudo")
            .args(["-n", "systemctl", "restart", "dnsmasq"])
            .output()
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
            "-n",
            "iptables",
            "-t",
            "nat",
            "-D",
            "PREROUTING",
            "-p",
            protocol,
            "--dport",
            &hp,
            "-j",
            "DNAT",
            "--to-destination",
            &dest,
        ])
        .status()
        .await;

    // Remove FORWARD rule
    let _ = Command::new("sudo")
        .args([
            "-n",
            "iptables",
            "-D",
            "FORWARD",
            "-p",
            protocol,
            "-d",
            guest_ip,
            "--dport",
            &guest_port.to_string(),
            "-j",
            "ACCEPT",
        ])
        .status()
        .await;

    Ok(())
}
