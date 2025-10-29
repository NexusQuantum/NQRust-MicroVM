/// Guest agent automatic installation for VMs
use anyhow::{bail, Result};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

// Universal service configurations for different init systems
const SYSTEMD_SERVICE: &str = r#"[Unit]
Description=Guest metrics agent
After=network.target
Wants=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/guest-agent
Restart=always
RestartSec=5
User=root
Group=root

# Logging
StandardOutput=journal
StandardError=journal

# Security
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
"#;

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

const SYSV_INIT: &str = r#"#!/bin/sh
### BEGIN INIT INFO
# Provides:          guest-agent
# Required-Start:    $remote_fs $network
# Required-Stop:     $remote_fs $network
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: Guest metrics agent
# Description:       Guest metrics agent for VM monitoring
### END INIT INFO

NAME="guest-agent"
DAEMON="/usr/local/bin/guest-agent"
PIDFILE="/var/run/$NAME.pid"
LOGFILE="/var/log/$NAME.log"

. /etc/init.d/functions || . /etc/rc.d/init.d/functions || exit 1

case "$1" in
    start)
        echo -n "Starting $NAME: "
        start-stop-daemon --start --quiet --background --make-pidfile --pidfile $PIDFILE --exec $DAEMON -- $DAEMON_OPTS
        echo "done"
        # Wait for network and report IP
        sleep 3
        /usr/local/bin/report-ip.sh &
        ;;
    stop)
        echo -n "Stopping $NAME: "
        start-stop-daemon --stop --quiet --pidfile $PIDFILE
        rm -f $PIDFILE
        echo "done"
        ;;
    restart)
        $0 stop
        $0 start
        ;;
    status)
        if [ -f $PIDFILE ]; then
            echo "$NAME is running (pid $(cat $PIDFILE))"
        else
            echo "$NAME is not running"
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        exit 1
        ;;
esac

exit 0
"#;

/// Install guest agent into a VM's rootfs
/// This is called during VM creation before the VM starts
pub async fn install_to_rootfs(rootfs_path: &str, vm_id: Uuid, manager_url: &str) -> Result<()> {
    tracing::info!("=== GUEST AGENT INSTALLATION STARTED ===");
    tracing::info!(rootfs = %rootfs_path, vm_id = %vm_id, manager_url = %manager_url, "Installing guest agent to rootfs");

    // Check if guest agent binary exists
    let guest_agent_binary = "target/x86_64-unknown-linux-musl/release/guest-agent";
    if !Path::new(guest_agent_binary).exists() {
        tracing::warn!(
            "Guest agent binary not found at {}, skipping installation",
            guest_agent_binary
        );
        return Ok(());
    }

    tracing::info!("Guest agent binary found at {}", guest_agent_binary);

    // Mount the rootfs
    let mount_point = format!("/tmp/vm-{}-rootfs", vm_id);
    fs::create_dir_all(&mount_point).await?;

    // Mount the rootfs image
    let mount_result = Command::new("sudo")
        .args(["mount", "-o", "loop", rootfs_path, &mount_point])
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

async fn install_files(
    mount_point: &str,
    vm_id: Uuid,
    manager_url: &str,
    guest_agent_binary: &str,
) -> Result<()> {
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

    tracing::info!("✅ Installed guest-agent binary to {}", agent_dest);

    // 2. Detect init system and install appropriate service
    let init_system = detect_init_system(mount_point).await?;
    tracing::info!(init_system = %init_system, "Detected init system");

    match init_system.as_str() {
        "systemd" => install_systemd_service(mount_point, vm_id).await?,
        "openrc" => install_openrc_service(mount_point, vm_id).await?,
        "sysvinit" => install_sysvinit_service(mount_point, vm_id).await?,
        _ => {
            tracing::warn!(init_system = %init_system, "Unsupported init system, installing as standalone binary");
            install_standalone(mount_point, vm_id, manager_url).await?;
        }
    }

    // 3. Create universal IP reporting script
    let report_script = create_ip_reporting_script(manager_url, vm_id);
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
    tracing::info!("✅ Created IP reporting script at {}", report_dest);

    // 4. Create config file for guest agent
    let config_content = format!(
        r#"# Guest Agent Configuration
# Auto-generated during VM creation
VM_ID={}
MANAGER_URL={}
"#,
        vm_id, manager_url
    );

    let config_temp = format!("/tmp/guest-agent-config-{}", vm_id);
    fs::write(&config_temp, config_content).await?;

    let config_dest = format!("{}/etc/guest-agent.conf", mount_point);
    Command::new("sudo")
        .args(["cp", &config_temp, &config_dest])
        .status()
        .await?;

    fs::remove_file(&config_temp).await?;
    tracing::info!("✅ Created guest agent config at {}", config_dest);

    // 5. Network restart script injection REMOVED
    // Old approach (unreliable marker-based script) replaced with:
    // - Guest agent /clean-network endpoint that stops networking BEFORE snapshot
    // - Ensures restored VMs start with clean network state and request fresh DHCP
    // - 100% reliable, no scripts or markers needed
    // inject_network_restart_script_if_openrc(mount_point).await?;

    tracing::info!("=== GUEST AGENT INSTALLATION COMPLETED ===");

    Ok(())
}

/// Detect which init system the VM uses
async fn detect_init_system(mount_point: &str) -> Result<String> {
    tracing::info!(
        "Detecting init system in mounted filesystem at {}",
        mount_point
    );

    // List what's actually in /etc to help with debugging
    if let Ok(output) = Command::new("sudo")
        .args(["ls", "-la", &format!("{}/etc", mount_point)])
        .output()
        .await
    {
        let content = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Contents of /etc: {}", content);
    }

    // Check for systemd (most common)
    if Command::new("sudo")
        .args(["test", "-d", &format!("{}/etc/systemd", mount_point)])
        .status()
        .await?
        .success()
    {
        tracing::info!("Detected systemd init system");
        return Ok("systemd".to_string());
    }

    // Check for OpenRC (Alpine, Gentoo)
    if Command::new("sudo")
        .args(["test", "-d", &format!("{}/etc/init.d", mount_point)])
        .status()
        .await?
        .success()
        && Command::new("sudo")
            .args(["test", "-f", &format!("{}/etc/rc.conf", mount_point)])
            .status()
            .await?
            .success()
    {
        tracing::info!("Detected OpenRC init system");
        return Ok("openrc".to_string());
    }

    // Check for SysV init (has /etc/init.d but not OpenRC)
    if Command::new("sudo")
        .args(["test", "-d", &format!("{}/etc/init.d", mount_point)])
        .status()
        .await?
        .success()
    {
        // Check for typical SysV init files
        if Command::new("sudo")
            .args(["test", "-f", &format!("{}/etc/inittab", mount_point)])
            .status()
            .await?
            .success()
            || Command::new("sudo")
                .args(["ls", &format!("{}/etc/rc*.d", mount_point)])
                .status()
                .await?
                .success()
        {
            tracing::info!("Detected SysV init system");
            return Ok("sysvinit".to_string());
        }

        // If we have /etc/init.d but no clear indicators, assume SysV-compatible
        tracing::info!("Found /etc/init.d directory, assuming SysV-compatible init system");
        return Ok("sysvinit".to_string());
    }

    // Check for runit (Void Linux, Alpine alternative)
    if Command::new("sudo")
        .args(["test", "-d", &format!("{}/etc/runit", mount_point)])
        .status()
        .await?
        .success()
    {
        tracing::info!("Detected runit init system");
        return Ok("runit".to_string());
    }

    tracing::warn!("Could not detect init system, falling back to standalone");
    Ok("unknown".to_string())
}

/// Install systemd service
async fn install_systemd_service(mount_point: &str, vm_id: Uuid) -> Result<()> {
    let service_temp = format!("/tmp/guest-agent-{}.service", vm_id);
    fs::write(&service_temp, SYSTEMD_SERVICE).await?;

    let service_dest = format!("{}/etc/systemd/system/guest-agent.service", mount_point);
    Command::new("sudo")
        .args([
            "mkdir",
            "-p",
            &format!("{}/etc/systemd/system", mount_point),
        ])
        .status()
        .await?;

    Command::new("sudo")
        .args(["cp", &service_temp, &service_dest])
        .status()
        .await?;

    fs::remove_file(&service_temp).await?;

    // Enable the service
    let enable_dir = format!("{}/etc/systemd/system/multi-user.target.wants", mount_point);
    Command::new("sudo")
        .args(["mkdir", "-p", &enable_dir])
        .status()
        .await?;

    Command::new("sudo")
        .args([
            "ln",
            "-sf",
            "/etc/systemd/system/guest-agent.service",
            &format!("{}/guest-agent.service", enable_dir),
        ])
        .status()
        .await?;

    tracing::debug!("Installed systemd service");
    Ok(())
}

/// Install OpenRC service
async fn install_openrc_service(mount_point: &str, vm_id: Uuid) -> Result<()> {
    tracing::info!("📦 Installing OpenRC guest-agent service...");

    let service_temp = format!("/tmp/guest-agent-service-{}", vm_id);
    fs::write(&service_temp, OPENRC_SERVICE).await?;
    tracing::info!("  ✅ Created service file at {}", service_temp);

    let service_dest = format!("{}/etc/init.d/guest-agent", mount_point);
    Command::new("sudo")
        .args(["cp", &service_temp, &service_dest])
        .status()
        .await?;
    tracing::info!("  ✅ Copied service to {}", service_dest);

    Command::new("sudo")
        .args(["chmod", "+x", &service_dest])
        .status()
        .await?;
    tracing::info!("  ✅ Made service executable");

    fs::remove_file(&service_temp).await?;

    // Enable the service
    let runlevel_dir = format!("{}/etc/runlevels/default", mount_point);
    Command::new("sudo")
        .args(["mkdir", "-p", &runlevel_dir])
        .status()
        .await?;
    tracing::info!("  ✅ Created runlevel directory");

    let symlink_path = format!("{}/guest-agent", runlevel_dir);
    Command::new("sudo")
        .args([
            "ln",
            "-sf",
            "/etc/init.d/guest-agent",
            &symlink_path,
        ])
        .status()
        .await?;
    tracing::info!("  ✅ Enabled service in default runlevel at {}", symlink_path);

    tracing::info!("✅ OpenRC guest-agent service installed and enabled");
    Ok(())
}

/// Install SysV init script
async fn install_sysvinit_service(mount_point: &str, vm_id: Uuid) -> Result<()> {
    let service_temp = format!("/tmp/guest-agent-init-{}", vm_id);
    fs::write(&service_temp, SYSV_INIT).await?;

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

    // Enable the service (create runlevel symlinks)
    for runlevel in ["2", "3", "4", "5"] {
        let rc_dir = format!("{}/etc/rc{}.d", mount_point, runlevel);
        Command::new("sudo")
            .args(["mkdir", "-p", &rc_dir])
            .status()
            .await?;

        Command::new("sudo")
            .args([
                "ln",
                "-sf",
                "../init.d/guest-agent",
                &format!("{}/S99guest-agent", rc_dir),
            ])
            .status()
            .await?;
    }

    tracing::debug!("Installed SysV init script");
    Ok(())
}

/// Install as standalone binary (no init system support)
async fn install_standalone(mount_point: &str, vm_id: Uuid, manager_url: &str) -> Result<()> {
    // Create a startup script in /etc/rc.local or equivalent
    let startup_script = format!(
        r#"#!/bin/sh
# Guest agent startup script
/usr/local/bin/guest-agent &
/usr/local/bin/report-ip.sh &
"#,
    );

    let script_temp = format!("/tmp/guest-agent-startup-{}", vm_id);
    fs::write(&script_temp, startup_script).await?;

    // Try different locations for startup scripts
    let locations = [
        format!("{}/etc/rc.local", mount_point),
        format!("{}/etc/rc.d/rc.local", mount_point),
        format!("{}/etc/init.d/boot.local", mount_point),
    ];

    for dest in locations {
        if Command::new("sudo")
            .args(["test", "-f", &dest])
            .status()
            .await
            .ok()
            .map_or(false, |s| s.success())
        {
            // Append to existing file
            Command::new("sudo")
                .args(["sh", "-c", &format!("cat {} >> {}", script_temp, dest)])
                .status()
                .await?;
            break;
        } else if Command::new("sudo")
            .args(["touch", &dest])
            .status()
            .await
            .ok()
            .map_or(false, |s| s.success())
        {
            // Create new file
            Command::new("sudo")
                .args(["cp", &script_temp, &dest])
                .status()
                .await?;
            Command::new("sudo")
                .args(["chmod", "+x", &dest])
                .status()
                .await?;
            break;
        }
    }

    fs::remove_file(&script_temp).await?;
    tracing::debug!("Installed standalone startup");
    Ok(())
}

/// Create universal IP reporting script
fn create_ip_reporting_script(manager_url: &str, vm_id: Uuid) -> String {
    format!(
        r#"#!/bin/sh
# Universal IP reporting script for guest agent

MAX_RETRIES=30
RETRY=0

# Function to detect IP address
detect_ip() {{
    # Try multiple methods to get IP
    ip addr show eth0 2>/dev/null | grep 'inet ' | head -1 | awk '{{print $2}}' | cut -d/ -f1 && return 0
    ifconfig eth0 2>/dev/null | grep 'inet ' | head -1 | awk '{{print $2}}' && return 0
    ip route get 1 2>/dev/null | awk '{{print $7}}' | head -1 && return 0
    return 1
}}

# Function to report IP to manager
report_ip() {{
    local ip="$1"
    
    # Try curl first
    if command -v curl >/dev/null 2>&1; then
        curl -s -X POST {}/v1/vms/{}/guest-ip \
            -H "Content-Type: application/json" \
            -d "{{\\"guest_ip\\":\\"$ip\\"}}" 2>/dev/null && return 0
    fi
    
    # Try wget
    if command -v wget >/dev/null 2>&1; then
        wget -q -O- --post-data="{{\\"guest_ip\\":\\"$ip\\"}}" \
            --header="Content-Type: application/json" \
            {}/v1/vms/{}/guest-ip 2>/dev/null && return 0
    fi
    
    # Try netcat (if available)
    if command -v nc >/dev/null 2>&1; then
        echo "{{\\"guest_ip\\":\\"$ip\\"}}" | nc {} 80 2>/dev/null && return 0
    fi
    
    return 1
}}

# Main loop
while [ $RETRY -lt $MAX_RETRIES ]; do
    MY_IP=$(detect_ip)
    
    if [ -n "$MY_IP" ] && [ "$MY_IP" != "127.0.0.1" ] && [ "$MY_IP" != "" ]; then
        if report_ip "$MY_IP"; then
            logger -t guest-agent "Successfully reported IP $MY_IP to manager"
            break
        else
            logger -t guest-agent "Failed to report IP $MY_IP to manager (attempt $RETRY)"
        fi
    else
        logger -t guest-agent "No valid IP address found (attempt $RETRY)"
    fi
    
    RETRY=$((RETRY + 1))
    sleep 2
done

if [ $RETRY -ge $MAX_RETRIES ]; then
    logger -t guest-agent "Failed to report IP after $MAX_RETRIES attempts"
fi
"#,
        manager_url,
        vm_id,
        manager_url,
        vm_id,
        manager_url.split("://").nth(1).unwrap_or(manager_url)
    )
}

/// Inject network restart script for OpenRC-based systems (Alpine Linux)
/// This ensures VMs restored from snapshots get fresh DHCP leases
async fn inject_network_restart_script_if_openrc(mount_point: &str) -> Result<()> {
    // Check if this is an OpenRC system
    let local_d_path = format!("{}/etc/local.d", mount_point);
    if !Path::new(&local_d_path).exists() {
        tracing::info!("Not an OpenRC system, skipping network restart script injection");
        return Ok(());
    }

    tracing::info!("Injecting network restart script for snapshot-based VMs");

    let script_content = r#"#!/bin/sh
# Auto-generated script to refresh network on snapshot restore
# This ensures each restored VM gets a unique IP via DHCP

# IMPORTANT: Only run this on snapshot restore, not on initial boot
# We detect snapshot restore by checking if this is the second boot
MARKER=/var/run/first-boot-done
if [ ! -f "$MARKER" ]; then
    # First boot (golden template creation) - just mark and skip
    touch "$MARKER"
    exit 0
fi

# This is snapshot restore (second+ boot) - restart networking
# Kill any existing DHCP client
pkill udhcpc 2>/dev/null || true

# Restart networking to get fresh DHCP lease
rc-service networking restart
"#;

    let script_temp = format!("/tmp/99-refresh-network-{}.start", uuid::Uuid::new_v4());
    fs::write(&script_temp, script_content).await?;

    let script_dest = format!("{}/etc/local.d/99-refresh-network.start", mount_point);
    Command::new("sudo")
        .args(["cp", &script_temp, &script_dest])
        .status()
        .await?;

    Command::new("sudo")
        .args(["chmod", "+x", &script_dest])
        .status()
        .await?;

    fs::remove_file(&script_temp).await?;

    // Enable local service if not already enabled (via chroot)
    let _ = Command::new("sudo")
        .args(["chroot", mount_point, "rc-update", "add", "local", "default"])
        .output()
        .await;

    tracing::info!("✅ Network restart script injected at {}", script_dest);

    Ok(())
}

/// Check if guest agent binary exists
pub fn is_available() -> bool {
    Path::new("target/x86_64-unknown-linux-musl/release/guest-agent").exists()
}
