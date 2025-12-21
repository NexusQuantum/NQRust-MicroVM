//! Port forwarding management for containers
//!
//! This module handles setting up and cleaning up iptables rules to forward
//! host ports to container VM ports.

use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::process::Stdio;
use std::sync::{LazyLock, Mutex};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Global set to track used ports (in-memory, cleared on restart)
static USED_PORTS: LazyLock<Mutex<HashSet<u16>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Check if a host port is available
pub async fn check_port_available(port: u16) -> Result<bool> {
    // First check our in-memory registry
    {
        let used = USED_PORTS.lock().unwrap();
        if used.contains(&port) {
            return Ok(false);
        }
    }

    // Then check with the OS using ss command
    let output = Command::new("ss")
        .args(["-tlnp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .context("Failed to execute ss command")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let port_pattern = format!(":{}", port);

    // Check if any line shows the port in use
    for line in output_str.lines() {
        if line.contains(&port_pattern) {
            // Check if it's actually listening on this exact port
            // ss output format: "LISTEN  0  128  0.0.0.0:5432  0.0.0.0:*"
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.ends_with(&port_pattern) || part.contains(&format!(":{}$", port)) {
                    // More precise check - the port should be at the end
                    if let Some((_addr, p)) = part.rsplit_once(':') {
                        if p.parse::<u16>().ok() == Some(port) {
                            return Ok(false);
                        }
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Check multiple ports and return list of unavailable ports
pub async fn check_ports_available(ports: &[u16]) -> Result<Vec<u16>> {
    let mut unavailable = Vec::new();

    for &port in ports {
        if !check_port_available(port).await? {
            unavailable.push(port);
        }
    }

    Ok(unavailable)
}

/// Reserve a port in the in-memory registry
pub fn reserve_port(port: u16) {
    let mut used = USED_PORTS.lock().unwrap();
    used.insert(port);
    debug!(port = %port, "Port reserved");
}

/// Release a port from the in-memory registry
pub fn release_port(port: u16) {
    let mut used = USED_PORTS.lock().unwrap();
    used.remove(&port);
    debug!(port = %port, "Port released");
}

/// Set up port forwarding from host to container VM using iptables
///
/// This creates DNAT rules to forward traffic from the host port to the container VM's port.
pub async fn setup_port_forward(
    host_port: u16,
    vm_ip: &str,
    container_port: u16,
    protocol: &str,
) -> Result<()> {
    let protocol = protocol.to_lowercase();
    if protocol != "tcp" && protocol != "udp" {
        return Err(anyhow!("Invalid protocol: {}. Must be tcp or udp", protocol));
    }

    info!(
        host_port = %host_port,
        vm_ip = %vm_ip,
        container_port = %container_port,
        protocol = %protocol,
        "Setting up port forwarding"
    );

    // Add PREROUTING rule (for external traffic)
    let prerouting_result = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-A",
            "PREROUTING",
            "-p",
            &protocol,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", vm_ip, container_port),
        ])
        .output()
        .await;

    match prerouting_result {
        Ok(output) if output.status.success() => {
            debug!("PREROUTING rule added successfully");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("PREROUTING rule may have failed: {}", stderr);
        }
        Err(e) => {
            error!("Failed to add PREROUTING rule: {}", e);
        }
    }

    // Add OUTPUT rule (for local traffic from host machine itself)
    let output_result = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-A",
            "OUTPUT",
            "-p",
            &protocol,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", vm_ip, container_port),
        ])
        .output()
        .await;

    match output_result {
        Ok(output) if output.status.success() => {
            debug!("OUTPUT rule added successfully");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("OUTPUT rule may have failed: {}", stderr);
        }
        Err(e) => {
            error!("Failed to add OUTPUT rule: {}", e);
        }
    }

    // Reserve the port in memory
    reserve_port(host_port);

    info!(
        host_port = %host_port,
        vm_ip = %vm_ip,
        container_port = %container_port,
        "Port forwarding setup complete"
    );

    Ok(())
}

/// Remove port forwarding rules for a specific host port
pub async fn remove_port_forward(
    host_port: u16,
    vm_ip: &str,
    container_port: u16,
    protocol: &str,
) -> Result<()> {
    let protocol = protocol.to_lowercase();

    info!(
        host_port = %host_port,
        vm_ip = %vm_ip,
        container_port = %container_port,
        protocol = %protocol,
        "Removing port forwarding"
    );

    // Remove PREROUTING rule
    let _ = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-D",
            "PREROUTING",
            "-p",
            &protocol,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", vm_ip, container_port),
        ])
        .output()
        .await;

    // Remove OUTPUT rule
    let _ = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-D",
            "OUTPUT",
            "-p",
            &protocol,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", vm_ip, container_port),
        ])
        .output()
        .await;

    // Release the port
    release_port(host_port);

    info!(host_port = %host_port, "Port forwarding removed");

    Ok(())
}

/// Remove all port forwards for a container (given its port mappings and VM IP)
pub async fn cleanup_port_forwards(
    port_mappings: &[nexus_types::PortMapping],
    vm_ip: &str,
) -> Result<()> {
    for mapping in port_mappings {
        if let Err(e) = remove_port_forward(
            mapping.host as u16,
            vm_ip,
            mapping.container as u16,
            &mapping.protocol,
        )
        .await
        {
            warn!(
                host_port = %mapping.host,
                error = %e,
                "Failed to remove port forward"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_port_reservation() {
        // Reserve a port
        reserve_port(9999);

        // Check it's not available anymore in our registry
        {
            let used = USED_PORTS.lock().unwrap();
            assert!(used.contains(&9999));
        }

        // Release it
        release_port(9999);

        // Should be available now
        {
            let used = USED_PORTS.lock().unwrap();
            assert!(!used.contains(&9999));
        }
    }
}
