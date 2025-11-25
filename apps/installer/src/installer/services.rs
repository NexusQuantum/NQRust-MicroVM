//! Systemd service management module.

use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::Result;

use crate::app::{InstallConfig, LogEntry};
use crate::installer::{run_command, run_sudo};

/// Install systemd services
pub fn install_services(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info("Installing systemd services..."));

    // Install manager service if needed
    if config.mode.includes_manager() {
        logs.extend(install_manager_service(config)?);
    }

    // Install agent service if needed
    if config.mode.includes_agent() {
        logs.extend(install_agent_service(config)?);
    }

    // Install UI service if needed
    if config.mode.includes_ui() {
        logs.extend(install_ui_service(config)?);
    }

    // Reload systemd
    let _ = run_sudo("systemctl", &["daemon-reload"]);
    logs.push(LogEntry::success("Systemd services installed"));

    Ok(logs)
}

/// Install manager systemd service
fn install_manager_service(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let bin_path = config.install_dir.join("bin/nqrust-manager");
    let env_file = config.config_dir.join("manager.env");

    let service_content = format!(
        r#"[Unit]
Description=NQR-MicroVM Manager Service
Documentation=https://github.com/your-org/nqrust-microvm
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=nqrust
Group=nqrust
EnvironmentFile={}
ExecStart={}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nqrust-manager

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths={} {} {} /srv/images

[Install]
WantedBy=multi-user.target
"#,
        env_file.display(),
        bin_path.display(),
        config.data_dir.display(),
        config.data_dir.join("vms").display(),
        config.data_dir.join("images").display()
    );

    write_service_file("nqrust-manager.service", &service_content)?;
    logs.push(LogEntry::success("Manager service file created"));

    Ok(logs)
}

/// Install agent systemd service
fn install_agent_service(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let bin_path = config.install_dir.join("bin/nqrust-agent");
    let env_file = config.config_dir.join("agent.env");

    let service_content = format!(
        r#"[Unit]
Description=NQR-MicroVM Agent Service
Documentation=https://github.com/your-org/nqrust-microvm
After=network.target nqrust-bridge.service
Wants=nqrust-bridge.service

[Service]
Type=simple
User=root
EnvironmentFile={}
ExecStart={}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nqrust-agent

# Agent needs elevated privileges for KVM/networking
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN

[Install]
WantedBy=multi-user.target
"#,
        env_file.display(),
        bin_path.display()
    );

    write_service_file("nqrust-agent.service", &service_content)?;
    logs.push(LogEntry::success("Agent service file created"));

    Ok(logs)
}

/// Install UI systemd service
fn install_ui_service(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let ui_dir = config.install_dir.join("ui");
    let env_file = config.config_dir.join("ui.env");

    let service_content = format!(
        r#"[Unit]
Description=NQR-MicroVM Web UI Service
Documentation=https://github.com/your-org/nqrust-microvm
After=network.target nqrust-manager.service
Wants=nqrust-manager.service

[Service]
Type=simple
User=nqrust
Group=nqrust
WorkingDirectory={}
EnvironmentFile={}
ExecStart=/usr/bin/pnpm start
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nqrust-ui

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
"#,
        ui_dir.display(),
        env_file.display()
    );

    write_service_file("nqrust-ui.service", &service_content)?;
    logs.push(LogEntry::success("UI service file created"));

    Ok(logs)
}

/// Write a systemd service file
fn write_service_file(name: &str, content: &str) -> Result<()> {
    let path = format!("/etc/systemd/system/{}", name);
    let cmd = format!(
        "echo '{}' | sudo tee {} > /dev/null",
        content.replace('\'', "'\"'\"'"),
        path
    );
    run_command("sh", &["-c", &cmd])?;
    Ok(())
}

/// Enable and start services
pub fn start_services(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info("Starting services..."));

    // Start manager first if needed
    if config.mode.includes_manager() {
        logs.push(LogEntry::info("Starting manager service..."));
        let _ = run_sudo("systemctl", &["enable", "nqrust-manager.service"]);
        let _output = run_sudo("systemctl", &["start", "nqrust-manager.service"])?;

        // Wait for service to start
        thread::sleep(Duration::from_secs(2));

        if check_service_status("nqrust-manager.service") {
            logs.push(LogEntry::success("Manager service started"));
        } else {
            logs.push(LogEntry::warning(
                "Manager service may not have started correctly",
            ));
        }
    }

    // Start agent if needed
    if config.mode.includes_agent() {
        logs.push(LogEntry::info("Starting agent service..."));
        let _ = run_sudo("systemctl", &["enable", "nqrust-agent.service"]);
        let _ = run_sudo("systemctl", &["start", "nqrust-agent.service"]);

        thread::sleep(Duration::from_secs(2));

        if check_service_status("nqrust-agent.service") {
            logs.push(LogEntry::success("Agent service started"));
        } else {
            logs.push(LogEntry::warning(
                "Agent service may not have started correctly",
            ));
        }
    }

    // Start UI if needed
    if config.mode.includes_ui() {
        logs.push(LogEntry::info("Starting UI service..."));
        let _ = run_sudo("systemctl", &["enable", "nqrust-ui.service"]);
        let _ = run_sudo("systemctl", &["start", "nqrust-ui.service"]);

        thread::sleep(Duration::from_secs(2));

        if check_service_status("nqrust-ui.service") {
            logs.push(LogEntry::success("UI service started"));
        } else {
            logs.push(LogEntry::warning(
                "UI service may not have started correctly",
            ));
        }
    }

    logs.push(LogEntry::success("Services started"));

    Ok(logs)
}

/// Check if a service is active
fn check_service_status(name: &str) -> bool {
    if let Ok(output) = run_command("systemctl", &["is-active", name]) {
        let status = String::from_utf8_lossy(&output.stdout);
        return status.trim() == "active";
    }
    false
}

/// Stop and disable all services
pub fn stop_services() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info("Stopping services..."));

    let services = [
        "nqrust-ui.service",
        "nqrust-agent.service",
        "nqrust-manager.service",
        "nqrust-bridge.service",
    ];

    for service in &services {
        let _ = run_sudo("systemctl", &["stop", service]);
        let _ = run_sudo("systemctl", &["disable", service]);
    }

    logs.push(LogEntry::success("Services stopped"));

    Ok(logs)
}

/// Remove service files
pub fn remove_services() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info("Removing service files..."));

    let services = [
        "/etc/systemd/system/nqrust-ui.service",
        "/etc/systemd/system/nqrust-agent.service",
        "/etc/systemd/system/nqrust-manager.service",
        "/etc/systemd/system/nqrust-bridge.service",
    ];

    for service in &services {
        if Path::new(service).exists() {
            let _ = run_sudo("rm", &[service]);
        }
    }

    let _ = run_sudo("systemctl", &["daemon-reload"]);

    logs.push(LogEntry::success("Service files removed"));

    Ok(logs)
}

/// Get service status summary
pub fn get_service_status(config: &InstallConfig) -> Vec<(String, bool)> {
    let mut status = Vec::new();

    if config.mode.includes_manager() {
        status.push((
            "nqrust-manager".to_string(),
            check_service_status("nqrust-manager.service"),
        ));
    }

    if config.mode.includes_agent() {
        status.push((
            "nqrust-agent".to_string(),
            check_service_status("nqrust-agent.service"),
        ));
    }

    if config.mode.includes_ui() {
        status.push((
            "nqrust-ui".to_string(),
            check_service_status("nqrust-ui.service"),
        ));
    }

    status
}
