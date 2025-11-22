//! Installation verification module.

use std::path::Path;
use std::thread;
use std::time::Duration;

#[allow(unused_imports)]
use anyhow::Result;

use crate::app::{CheckItem, InstallConfig, Status};
use crate::installer::run_command;

/// Run all verification checks
pub fn run_verification(config: &InstallConfig) -> Vec<CheckItem> {
    let mut checks = Vec::new();

    // Binary checks
    checks.extend(verify_binaries(config));

    // Service checks
    checks.extend(verify_services(config));

    // Health endpoint checks
    checks.extend(verify_health_endpoints(config));

    // Infrastructure checks
    checks.extend(verify_infrastructure(config));

    checks
}

/// Verify binaries are installed
fn verify_binaries(config: &InstallConfig) -> Vec<CheckItem> {
    let mut checks = Vec::new();
    let bin_dir = config.install_dir.join("bin");

    if config.mode.includes_manager() {
        let path = bin_dir.join("nqrust-manager");
        let status = if path.exists() && is_executable(&path) {
            Status::Success
        } else {
            Status::Error
        };
        checks.push(
            CheckItem::new("Manager Binary", "nqrust-manager exists and is executable")
                .with_status(status)
                .with_message(path.display().to_string()),
        );
    }

    if config.mode.includes_agent() {
        let path = bin_dir.join("nqrust-agent");
        let status = if path.exists() && is_executable(&path) {
            Status::Success
        } else {
            Status::Error
        };
        checks.push(
            CheckItem::new("Agent Binary", "nqrust-agent exists and is executable")
                .with_status(status)
                .with_message(path.display().to_string()),
        );

        let path = bin_dir.join("guest-agent");
        let status = if path.exists() && is_executable(&path) {
            Status::Success
        } else {
            Status::Warning
        };
        checks.push(
            CheckItem::new("Guest Agent", "guest-agent for VMs")
                .with_status(status)
                .with_message(path.display().to_string()),
        );
    }

    checks
}

/// Verify services are running
fn verify_services(config: &InstallConfig) -> Vec<CheckItem> {
    let mut checks = Vec::new();

    if config.mode.includes_manager() {
        let (active, enabled) = get_service_state("nqrust-manager.service");
        let status = if active {
            Status::Success
        } else if enabled {
            Status::Warning
        } else {
            Status::Error
        };
        let msg = format!(
            "{}{}",
            if active { "active" } else { "inactive" },
            if enabled { " (enabled)" } else { " (disabled)" }
        );
        checks.push(
            CheckItem::new("Manager Service", "nqrust-manager.service")
                .with_status(status)
                .with_message(msg),
        );
    }

    if config.mode.includes_agent() {
        let (active, enabled) = get_service_state("nqrust-agent.service");
        let status = if active {
            Status::Success
        } else if enabled {
            Status::Warning
        } else {
            Status::Error
        };
        let msg = format!(
            "{}{}",
            if active { "active" } else { "inactive" },
            if enabled { " (enabled)" } else { " (disabled)" }
        );
        checks.push(
            CheckItem::new("Agent Service", "nqrust-agent.service")
                .with_status(status)
                .with_message(msg),
        );
    }

    if config.mode.includes_ui() {
        let (active, enabled) = get_service_state("nqrust-ui.service");
        let status = if active {
            Status::Success
        } else if enabled {
            Status::Warning
        } else {
            Status::Error
        };
        let msg = format!(
            "{}{}",
            if active { "active" } else { "inactive" },
            if enabled { " (enabled)" } else { " (disabled)" }
        );
        checks.push(
            CheckItem::new("UI Service", "nqrust-ui.service")
                .with_status(status)
                .with_message(msg),
        );
    }

    checks
}

/// Verify health endpoints
fn verify_health_endpoints(config: &InstallConfig) -> Vec<CheckItem> {
    let mut checks = Vec::new();

    if config.mode.includes_manager() {
        let status = check_health_endpoint("http://localhost:18080/health", 10);
        checks.push(
            CheckItem::new("Manager Health", "API responds on port 18080")
                .with_status(status)
                .with_message("http://localhost:18080/health"),
        );
    }

    if config.mode.includes_agent() {
        let status = check_health_endpoint("http://localhost:9090/health", 10);
        checks.push(
            CheckItem::new("Agent Health", "API responds on port 9090")
                .with_status(status)
                .with_message("http://localhost:9090/health"),
        );
    }

    if config.mode.includes_ui() {
        let status = check_health_endpoint("http://localhost:3000", 10);
        checks.push(
            CheckItem::new("UI Health", "Web UI responds on port 3000")
                .with_status(status)
                .with_message("http://localhost:3000"),
        );
    }

    checks
}

/// Verify infrastructure components
fn verify_infrastructure(config: &InstallConfig) -> Vec<CheckItem> {
    let mut checks = Vec::new();

    // Database check
    if config.mode.includes_manager() {
        let status = check_database_connection(&config.db_name, &config.db_user);
        checks.push(
            CheckItem::new("Database", "PostgreSQL connection")
                .with_status(status)
                .with_message(format!("{}@{}", config.db_name, config.db_host)),
        );
    }

    // Network bridge check
    if config.mode.includes_agent() {
        let status = check_network_bridge(&config.bridge_name);
        checks.push(
            CheckItem::new("Network Bridge", format!("{} is UP", config.bridge_name))
                .with_status(status)
                .with_message(config.bridge_name.clone()),
        );

        // KVM check
        let status = check_kvm_access();
        checks.push(
            CheckItem::new("KVM Access", "/dev/kvm is accessible")
                .with_status(status)
                .with_message("/dev/kvm"),
        );
    }

    // Directory permissions
    let status = check_directory_permissions(&config.data_dir);
    checks.push(
        CheckItem::new("Data Directory", "Permissions and ownership")
            .with_status(status)
            .with_message(config.data_dir.display().to_string()),
    );

    checks
}

/// Check if a file is executable
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = path.metadata() {
        return metadata.permissions().mode() & 0o111 != 0;
    }
    false
}

/// Get service state (active, enabled)
fn get_service_state(name: &str) -> (bool, bool) {
    let active = run_command("systemctl", &["is-active", name])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);

    let enabled = run_command("systemctl", &["is-enabled", name])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
        .unwrap_or(false);

    (active, enabled)
}

/// Check health endpoint with retries
fn check_health_endpoint(url: &str, max_attempts: u32) -> Status {
    for attempt in 1..=max_attempts {
        if let Ok(output) = run_command(
            "curl",
            &["-sf", "-o", "/dev/null", "-w", "%{http_code}", url],
        ) {
            let code = String::from_utf8_lossy(&output.stdout);
            if code.trim() == "200" || output.status.success() {
                return Status::Success;
            }
        }

        if attempt < max_attempts {
            thread::sleep(Duration::from_secs(2));
        }
    }

    Status::Error
}

/// Check database connection
fn check_database_connection(db_name: &str, _db_user: &str) -> Status {
    let output = run_command(
        "sudo",
        &["-u", "postgres", "psql", "-d", db_name, "-c", "SELECT 1;"],
    );

    if let Ok(out) = output {
        if out.status.success() {
            return Status::Success;
        }
    }

    Status::Warning
}

/// Check network bridge
fn check_network_bridge(name: &str) -> Status {
    let bridge_path = format!("/sys/class/net/{}/bridge", name);
    if !Path::new(&bridge_path).exists() {
        return Status::Error;
    }

    let operstate_path = format!("/sys/class/net/{}/operstate", name);
    if let Ok(state) = std::fs::read_to_string(operstate_path) {
        if state.trim() == "up" {
            return Status::Success;
        }
    }

    Status::Warning
}

/// Check KVM access
fn check_kvm_access() -> Status {
    if !Path::new("/dev/kvm").exists() {
        return Status::Error;
    }

    // Try to read /dev/kvm
    if let Ok(output) = run_command("test", &["-r", "/dev/kvm"]) {
        if output.status.success() {
            return Status::Success;
        }
    }

    Status::Warning
}

/// Check directory permissions
fn check_directory_permissions(path: &Path) -> Status {
    if !path.exists() {
        return Status::Error;
    }

    // Check if writable
    if let Ok(output) = run_command("test", &["-w", &path.display().to_string()]) {
        if output.status.success() {
            return Status::Success;
        }
    }

    // Check ownership
    if let Ok(output) = run_command("stat", &["-c", "%U", &path.display().to_string()]) {
        let owner = String::from_utf8_lossy(&output.stdout);
        if owner.trim() == "nqrust" || owner.trim() == "root" {
            return Status::Success;
        }
    }

    Status::Warning
}
