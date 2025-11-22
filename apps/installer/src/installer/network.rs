//! Network setup module.

use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::app::{LogEntry, NetworkMode};
use crate::installer::{run_command, run_sudo};

/// Setup network bridge
pub fn setup_network(mode: NetworkMode, bridge_name: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info(format!(
        "Setting up network in {} mode...",
        mode.name()
    )));

    // Check if bridge already exists
    if bridge_exists(bridge_name) {
        logs.push(LogEntry::warning(format!(
            "Bridge '{}' already exists",
            bridge_name
        )));

        // Check if it's UP
        if is_bridge_up(bridge_name) {
            logs.push(LogEntry::info(format!("Bridge '{}' is UP", bridge_name)));
            return Ok(logs);
        }
    }

    match mode {
        NetworkMode::Nat => setup_nat_network(bridge_name, &mut logs)?,
        NetworkMode::Bridged => {
            logs.push(LogEntry::warning(
                "Bridged mode requires manual network configuration",
            ));
            logs.push(LogEntry::info(
                "Please ensure you have console access before proceeding",
            ));
            setup_bridged_network(bridge_name, &mut logs)?;
        }
    }

    Ok(logs)
}

/// Setup NAT network (default)
fn setup_nat_network(bridge_name: &str, logs: &mut Vec<LogEntry>) -> Result<()> {
    let bridge_ip = "10.0.0.1";
    let bridge_cidr = "10.0.0.1/24";
    let dhcp_range_start = "10.0.0.10";
    let dhcp_range_end = "10.0.0.250";

    // Create bridge
    logs.push(LogEntry::info(format!(
        "Creating bridge '{}'...",
        bridge_name
    )));

    let _ = run_sudo(
        "ip",
        &["link", "add", "name", bridge_name, "type", "bridge"],
    );
    let _ = run_sudo("ip", &["addr", "add", bridge_cidr, "dev", bridge_name]);
    let _ = run_sudo("ip", &["link", "set", bridge_name, "up"]);

    logs.push(LogEntry::success(format!(
        "Bridge '{}' created with IP {}",
        bridge_name, bridge_ip
    )));

    // Enable IP forwarding
    logs.push(LogEntry::info("Enabling IP forwarding..."));
    let _ = run_command(
        "sh",
        &[
            "-c",
            "echo 1 | sudo tee /proc/sys/net/ipv4/ip_forward > /dev/null",
        ],
    );

    // Make IP forwarding persistent
    let sysctl_content = "net.ipv4.ip_forward = 1";
    let sysctl_file = "/etc/sysctl.d/99-nqrust-ipforward.conf";
    let write_cmd = format!(
        "echo '{}' | sudo tee {} > /dev/null",
        sysctl_content, sysctl_file
    );
    let _ = run_command("sh", &["-c", &write_cmd]);

    logs.push(LogEntry::success("IP forwarding enabled"));

    // Setup iptables NAT rules
    logs.push(LogEntry::info("Configuring NAT rules..."));

    // Get default interface
    let default_iface = get_default_interface().unwrap_or_else(|| "eth0".to_string());

    // Add masquerade rule
    let _ = run_sudo(
        "iptables",
        &[
            "-t",
            "nat",
            "-A",
            "POSTROUTING",
            "-s",
            "10.0.0.0/24",
            "-o",
            &default_iface,
            "-j",
            "MASQUERADE",
        ],
    );

    // Allow forwarding
    let _ = run_sudo(
        "iptables",
        &[
            "-A",
            "FORWARD",
            "-i",
            bridge_name,
            "-o",
            &default_iface,
            "-j",
            "ACCEPT",
        ],
    );
    let _ = run_sudo(
        "iptables",
        &[
            "-A",
            "FORWARD",
            "-i",
            &default_iface,
            "-o",
            bridge_name,
            "-m",
            "state",
            "--state",
            "RELATED,ESTABLISHED",
            "-j",
            "ACCEPT",
        ],
    );

    logs.push(LogEntry::success("NAT rules configured"));

    // Setup dnsmasq for DHCP
    logs.push(LogEntry::info("Configuring DHCP server (dnsmasq)..."));

    let dnsmasq_config = format!(
        r#"# NQRust-MicroVM DHCP Configuration
interface={}
bind-interfaces
dhcp-range={},{},12h
dhcp-option=option:router,{}
dhcp-option=option:dns-server,8.8.8.8,8.8.4.4,1.1.1.1
"#,
        bridge_name, dhcp_range_start, dhcp_range_end, bridge_ip
    );

    let dnsmasq_file = "/etc/dnsmasq.d/nqrust-microvm.conf";
    let write_cmd = format!(
        "echo '{}' | sudo tee {} > /dev/null",
        dnsmasq_config, dnsmasq_file
    );
    let _ = run_command("sh", &["-c", &write_cmd]);

    // Restart dnsmasq
    let _ = run_sudo("systemctl", &["enable", "dnsmasq"]);
    let _ = run_sudo("systemctl", &["restart", "dnsmasq"]);

    logs.push(LogEntry::success(format!(
        "DHCP configured (range: {} - {})",
        dhcp_range_start, dhcp_range_end
    )));

    // Create systemd service to recreate bridge on boot
    create_bridge_service(bridge_name, bridge_cidr)?;
    logs.push(LogEntry::success("Bridge persistence service created"));

    Ok(())
}

/// Setup bridged network
fn setup_bridged_network(bridge_name: &str, logs: &mut Vec<LogEntry>) -> Result<()> {
    logs.push(LogEntry::warning(
        "Bridged network setup requires manual configuration",
    ));
    logs.push(LogEntry::info(format!(
        "Creating bridge '{}' without IP configuration...",
        bridge_name
    )));

    // Create bridge
    let _ = run_sudo(
        "ip",
        &["link", "add", "name", bridge_name, "type", "bridge"],
    );
    let _ = run_sudo("ip", &["link", "set", bridge_name, "up"]);

    // Enable promiscuous mode
    let _ = run_sudo("ip", &["link", "set", bridge_name, "promisc", "on"]);

    logs.push(LogEntry::success(format!(
        "Bridge '{}' created (bridged mode)",
        bridge_name
    )));
    logs.push(LogEntry::info(
        "You will need to manually attach your network interface to this bridge",
    ));

    Ok(())
}

/// Create systemd service for bridge persistence
fn create_bridge_service(bridge_name: &str, bridge_cidr: &str) -> Result<()> {
    let service_content = format!(
        r#"[Unit]
Description=NQRust-MicroVM Bridge Setup
After=network.target

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/sbin/ip link add name {} type bridge
ExecStart=/sbin/ip addr add {} dev {}
ExecStart=/sbin/ip link set {} up
ExecStop=/sbin/ip link del {}

[Install]
WantedBy=multi-user.target
"#,
        bridge_name, bridge_cidr, bridge_name, bridge_name, bridge_name
    );

    let service_file = "/etc/systemd/system/nqrust-bridge.service";
    let write_cmd = format!(
        "echo '{}' | sudo tee {} > /dev/null",
        service_content, service_file
    );
    run_command("sh", &["-c", &write_cmd])?;

    // Enable service
    let _ = run_sudo("systemctl", &["daemon-reload"]);
    let _ = run_sudo("systemctl", &["enable", "nqrust-bridge.service"]);

    Ok(())
}

/// Check if a bridge exists
fn bridge_exists(name: &str) -> bool {
    Path::new(&format!("/sys/class/net/{}/bridge", name)).exists()
}

/// Check if a bridge is UP
fn is_bridge_up(name: &str) -> bool {
    if let Ok(operstate) = fs::read_to_string(format!("/sys/class/net/{}/operstate", name)) {
        return operstate.trim() == "up";
    }
    false
}

/// Get the default network interface
fn get_default_interface() -> Option<String> {
    if let Ok(output) = run_command("ip", &["route", "show", "default"]) {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Parse "default via X.X.X.X dev ethX ..."
        for part in output_str.split_whitespace() {
            if part.starts_with("eth") || part.starts_with("ens") || part.starts_with("enp") {
                return Some(part.to_string());
            }
        }
        // Try to find 'dev' keyword and get next word
        let parts: Vec<&str> = output_str.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "dev" && i + 1 < parts.len() {
                return Some(parts[i + 1].to_string());
            }
        }
    }
    None
}

/// Verify network is configured
pub fn verify_network(bridge_name: &str) -> Result<bool> {
    // Check bridge exists
    if !bridge_exists(bridge_name) {
        return Ok(false);
    }

    // Check bridge is UP
    if !is_bridge_up(bridge_name) {
        return Ok(false);
    }

    // Check IP forwarding
    if let Ok(forward) = fs::read_to_string("/proc/sys/net/ipv4/ip_forward") {
        if forward.trim() != "1" {
            return Ok(false);
        }
    }

    Ok(true)
}
