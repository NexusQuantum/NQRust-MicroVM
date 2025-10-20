/// Guest agent automatic installation for VMs
use anyhow::{Result, bail};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

const OPENRC_SERVICE: &str = r#"#!/sbin/openrc-run

name="guest-agent"
description="Guest metrics agent"
command="/usr/local/bin/guest-agent"
command_background=true
pidfile="/run/${RC_SVCNAME}.pid"
output_log="/var/log/guest-agent.log"
error_log="/var/log/guest-agent.err"

depend() {
    need net
    after firewall
}

start_post() {
    # Wait for network and report IP to manager
    sleep 3
    /usr/local/bin/report-ip.sh &
}
"#;

/// Install guest agent into a VM's rootfs
/// This is called during VM creation before the VM starts
pub async fn install_to_rootfs(rootfs_path: &str, vm_id: Uuid, manager_url: &str) -> Result<()> {
    // Check if guest agent binary exists
    let guest_agent_binary = "target/x86_64-unknown-linux-musl/release/guest-agent";
    if !Path::new(guest_agent_binary).exists() {
        tracing::warn!("Guest agent binary not found at {}, skipping installation", guest_agent_binary);
        return Ok(());
    }

    tracing::info!(rootfs = %rootfs_path, vm_id = %vm_id, "Installing guest agent to rootfs");

    // Mount the rootfs
    let mount_point = format!("/tmp/vm-{}-rootfs", vm_id);
    fs::create_dir_all(&mount_point).await?;

    // Mount the rootfs image
    let mount_result = Command::new("sudo")
        .args([
            "mount",
            "-o",
            "loop",
            rootfs_path,
            &mount_point,
        ])
        .status()
        .await?;

    if !mount_result.success() {
        bail!("Failed to mount rootfs at {}", mount_point);
    }

    // Ensure we unmount on any error
    let result = install_files(&mount_point, vm_id, manager_url, guest_agent_binary).await;

    // Always unmount
    let unmount_result = Command::new("sudo")
        .args(["umount", &mount_point])
        .status()
        .await;

    if let Err(e) = unmount_result {
        tracing::error!("Failed to unmount {}: {}", mount_point, e);
    }

    let _ = fs::remove_dir(&mount_point).await;

    result
}

async fn install_files(mount_point: &str, vm_id: Uuid, manager_url: &str, guest_agent_binary: &str) -> Result<()> {
    // 1. Copy guest-agent binary to /usr/local/bin/
    let agent_dest = format!("{}/usr/local/bin/guest-agent", mount_point);

    // Create directory if needed
    let bin_dir = format!("{}/usr/local/bin", mount_point);
    Command::new("sudo")
        .args(["mkdir", "-p", &bin_dir])
        .status()
        .await?;

    // Copy binary
    Command::new("sudo")
        .args(["cp", guest_agent_binary, &agent_dest])
        .status()
        .await?;

    // Make executable
    Command::new("sudo")
        .args(["chmod", "+x", &agent_dest])
        .status()
        .await?;

    tracing::debug!("Installed guest-agent binary to {}", agent_dest);

    // 2. Create OpenRC service file
    let service_temp = format!("/tmp/guest-agent-service-{}", vm_id);
    fs::write(&service_temp, OPENRC_SERVICE).await?;

    let service_dest = format!("{}/etc/init.d/guest-agent", mount_point);
    Command::new("sudo")
        .args(["cp", &service_temp, &service_dest])
        .status()
        .await?;

    Command::new("sudo")
        .args(["chmod", "+x", &service_dest])
        .status()
        .await?;

    fs::remove_file(&service_temp).await?;
    tracing::debug!("Created OpenRC service at {}", service_dest);

    // 3. Enable the service (create symlink in runlevels)
    let runlevel_dir = format!("{}/etc/runlevels/default", mount_point);
    Command::new("sudo")
        .args(["mkdir", "-p", &runlevel_dir])
        .status()
        .await?;

    let symlink_path = format!("{}/guest-agent", runlevel_dir);

    // Create symlink
    Command::new("sudo")
        .args(["ln", "-sf", "/etc/init.d/guest-agent", &symlink_path])
        .status()
        .await?;

    tracing::debug!("Enabled guest-agent service in default runlevel");

    // 4. Create IP reporting script
    let report_script = format!(
        r#"#!/bin/sh
# Report VM IP to manager

MAX_RETRIES=30
RETRY=0

while [ $RETRY -lt $MAX_RETRIES ]; do
    # Get IP address
    MY_IP=$(ip addr show eth0 2>/dev/null | grep 'inet ' | awk '{{print $2}}' | cut -d/ -f1)

    if [ -n "$MY_IP" ] && [ "$MY_IP" != "127.0.0.1" ]; then
        # Report to manager
        if command -v curl >/dev/null 2>&1; then
            curl -s -X POST {}/v1/vms/{}/guest-ip \
                -H "Content-Type: application/json" \
                -d "{{\\"guest_ip\\":\\"$MY_IP\\"}}" 2>/dev/null && break
        elif command -v wget >/dev/null 2>&1; then
            wget -q -O- --post-data="{{\\"guest_ip\\":\\"$MY_IP\\"}}" \
                --header="Content-Type: application/json" \
                {}/v1/vms/{}/guest-ip 2>/dev/null && break
        fi

        logger -t guest-agent "Reported IP $MY_IP to manager (attempt $RETRY)"
    fi

    RETRY=$((RETRY + 1))
    sleep 2
done

if [ $RETRY -ge $MAX_RETRIES ]; then
    logger -t guest-agent "Failed to report IP after $MAX_RETRIES attempts"
fi
"#,
        manager_url, vm_id, manager_url, vm_id
    );

    let report_temp = format!("/tmp/report-ip-{}.sh", vm_id);
    fs::write(&report_temp, report_script).await?;

    let report_dest = format!("{}/usr/local/bin/report-ip.sh", mount_point);
    Command::new("sudo")
        .args(["cp", &report_temp, &report_dest])
        .status()
        .await?;

    Command::new("sudo")
        .args(["chmod", "+x", &report_dest])
        .status()
        .await?;

    fs::remove_file(&report_temp).await?;
    tracing::debug!("Created IP reporting script at {}", report_dest);

    Ok(())
}

/// Check if guest agent binary exists
pub fn is_available() -> bool {
    Path::new("target/x86_64-unknown-linux-musl/release/guest-agent").exists()
}
