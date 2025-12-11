//! Pre-flight system checks.

use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use anyhow::Result;

use crate::app::{CheckItem, Status};
use crate::installer::{command_exists, run_command};

/// Run all pre-flight checks
pub fn run_preflight_checks() -> Vec<CheckItem> {
    vec![
        check_architecture(),
        check_os(),
        check_kernel(),
        check_systemd(),
        check_kvm_support(),
        check_memory(),
        check_disk_space(),
        check_required_commands(),
        check_port_available(18080, "Manager API"),
        check_port_available(9090, "Agent API"),
        check_port_available(3000, "Web UI"),
        check_port_available(5432, "PostgreSQL"),
    ]
}

/// Run pre-flight checks for offline/ISO mode (skip network-related checks, lenient on systemd/disk)
pub fn run_preflight_checks_offline() -> Vec<CheckItem> {
    vec![
        check_architecture(),
        check_os(),
        check_kernel(),
        check_systemd_offline(), // Lenient - live ISO uses sysvinit
        check_kvm_support(),
        check_memory(),
        check_disk_space_offline(), // Lenient - live ISO has limited RAM disk
        check_required_commands_offline(),
        check_port_available(18080, "Manager API"),
        check_port_available(9090, "Agent API"),
        check_port_available(3000, "Web UI"),
        check_port_available(5432, "PostgreSQL"),
    ]
}

/// Check for systemd in offline mode (warning only, not error)
/// Live ISO uses sysvinit, but target installation will have systemd
fn check_systemd_offline() -> CheckItem {
    if Path::new("/run/systemd/system").exists() {
        CheckItem::new("Systemd", "systemd init (target system)")
            .with_status(Status::Success)
            .with_message("systemd detected")
    } else {
        // In live ISO, sysvinit is expected - this is just a warning
        CheckItem::new("Systemd", "systemd init (target system)")
            .with_status(Status::Warning)
            .with_message("Live ISO uses sysvinit (OK for installation)")
    }
}

/// Check disk space in offline mode (more lenient, skip if on tmpfs)
fn check_disk_space_offline() -> CheckItem {
    // In live ISO mode, we're usually running from RAM (tmpfs/overlay)
    // The actual disk space check should happen when installing to target
    // For now, just show a warning that user needs to select a target disk
    
    // Try to find available block devices for installation
    if let Ok(output) = run_command("lsblk", &["-d", "-n", "-o", "NAME,SIZE,TYPE"]) {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let disks: Vec<&str> = output_str
            .lines()
            .filter(|l| l.contains("disk"))
            .collect();
        
        if !disks.is_empty() {
            return CheckItem::new("Disk Space", "Target disk available")
                .with_status(Status::Success)
                .with_message(format!("{} disk(s) found for installation", disks.len()));
        }
    }
    
    CheckItem::new("Disk Space", "Target disk available")
        .with_status(Status::Warning)
        .with_message("Select target disk during installation")
}

/// Check required commands for offline mode (no systemctl/curl/git needed in live ISO)
fn check_required_commands_offline() -> CheckItem {
    // In live ISO mode, we don't need systemctl (sysvinit), curl (offline), or git (offline)
    let required = ["sudo", "ip", "cp", "mount"];
    let missing: Vec<&str> = required
        .iter()
        .filter(|cmd| !command_exists(cmd))
        .copied()
        .collect();

    if missing.is_empty() {
        CheckItem::new("Required Commands", "sudo, ip, cp, mount")
            .with_status(Status::Success)
            .with_message("All commands available")
    } else {
        CheckItem::new("Required Commands", "sudo, ip, cp, mount")
            .with_status(Status::Error)
            .with_message(format!("Missing: {}", missing.join(", ")))
    }
}

/// Check CPU architecture
fn check_architecture() -> CheckItem {
    let arch = std::env::consts::ARCH;
    if arch == "x86_64" {
        CheckItem::new("Architecture", "x86_64 required")
            .with_status(Status::Success)
            .with_message(format!("Found: {}", arch))
    } else {
        CheckItem::new("Architecture", "x86_64 required")
            .with_status(Status::Error)
            .with_message(format!("Found: {} (unsupported)", arch))
    }
}

/// Check operating system
fn check_os() -> CheckItem {
    let os_info = get_os_info();

    let (supported, version_ok) = match os_info.id.as_str() {
        "ubuntu" => (true, parse_version(&os_info.version) >= (22, 4)),
        "debian" => (true, parse_version(&os_info.version) >= (11, 0)),
        "rhel" | "centos" | "rocky" | "almalinux" | "fedora" => {
            (true, parse_version(&os_info.version) >= (8, 0))
        }
        _ => (false, false),
    };

    if supported && version_ok {
        CheckItem::new("Operating System", "Ubuntu 22.04+ / Debian 11+ / RHEL 8+")
            .with_status(Status::Success)
            .with_message(format!("{} {}", os_info.name, os_info.version))
    } else if supported {
        CheckItem::new("Operating System", "Ubuntu 22.04+ / Debian 11+ / RHEL 8+")
            .with_status(Status::Warning)
            .with_message(format!(
                "{} {} (older version, may work)",
                os_info.name, os_info.version
            ))
    } else {
        CheckItem::new("Operating System", "Ubuntu 22.04+ / Debian 11+ / RHEL 8+")
            .with_status(Status::Error)
            .with_message(format!(
                "{} {} (unsupported)",
                os_info.name, os_info.version
            ))
    }
}

/// Check kernel version
fn check_kernel() -> CheckItem {
    let kernel = get_kernel_version();
    let (major, minor) = parse_version(&kernel);

    if major > 4 || (major == 4 && minor >= 14) {
        CheckItem::new("Kernel Version", "4.14 or newer")
            .with_status(Status::Success)
            .with_message(format!("Found: {}", kernel))
    } else {
        CheckItem::new("Kernel Version", "4.14 or newer")
            .with_status(Status::Error)
            .with_message(format!("Found: {} (too old)", kernel))
    }
}

/// Check for systemd
fn check_systemd() -> CheckItem {
    if Path::new("/run/systemd/system").exists() {
        CheckItem::new("Systemd", "systemd init required")
            .with_status(Status::Success)
            .with_message("systemd detected")
    } else {
        CheckItem::new("Systemd", "systemd init required")
            .with_status(Status::Error)
            .with_message("systemd not found")
    }
}

/// Check KVM support
fn check_kvm_support() -> CheckItem {
    // Check CPU flags
    let cpu_flags = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let has_vmx = cpu_flags.contains("vmx");
    let has_svm = cpu_flags.contains("svm");

    if !has_vmx && !has_svm {
        return CheckItem::new("KVM Support", "CPU virtualization enabled")
            .with_status(Status::Error)
            .with_message("No vmx/svm CPU flags found");
    }

    // Check /dev/kvm
    if !Path::new("/dev/kvm").exists() {
        return CheckItem::new("KVM Support", "CPU virtualization enabled")
            .with_status(Status::Warning)
            .with_message("CPU supports KVM but /dev/kvm not found (KVM module not loaded?)");
    }

    // Check KVM module
    let kvm_module = if has_vmx { "kvm_intel" } else { "kvm_amd" };
    let modules = fs::read_to_string("/proc/modules").unwrap_or_default();

    if !modules.contains(kvm_module) {
        return CheckItem::new("KVM Support", "CPU virtualization enabled")
            .with_status(Status::Warning)
            .with_message(format!("{} module not loaded", kvm_module));
    }

    CheckItem::new("KVM Support", "CPU virtualization enabled")
        .with_status(Status::Success)
        .with_message(format!(
            "{} ({})",
            if has_vmx { "Intel VT-x" } else { "AMD-V" },
            kvm_module
        ))
}

/// Check available memory
fn check_memory() -> CheckItem {
    let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let total_kb: u64 = meminfo
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let total_mb = total_kb / 1024;
    let total_gb = total_mb as f64 / 1024.0;

    if total_mb >= 2048 {
        CheckItem::new("Memory", "Minimum 2GB RAM")
            .with_status(Status::Success)
            .with_message(format!("{:.1} GB available", total_gb))
    } else {
        CheckItem::new("Memory", "Minimum 2GB RAM")
            .with_status(Status::Error)
            .with_message(format!("{:.1} GB available (need 2GB+)", total_gb))
    }
}

/// Check available disk space
fn check_disk_space() -> CheckItem {
    // Check /srv or / for available space
    let check_path = if Path::new("/srv").exists() {
        "/srv"
    } else {
        "/"
    };

    if let Ok(output) = run_command("df", &["-BG", check_path]) {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1) {
            if let Some(available) = line.split_whitespace().nth(3) {
                let available_gb: u64 = available.trim_end_matches('G').parse().unwrap_or(0);

                if available_gb >= 20 {
                    return CheckItem::new("Disk Space", "Minimum 20GB available")
                        .with_status(Status::Success)
                        .with_message(format!("{}GB available on {}", available_gb, check_path));
                } else {
                    return CheckItem::new("Disk Space", "Minimum 20GB available")
                        .with_status(Status::Error)
                        .with_message(format!("{}GB available (need 20GB+)", available_gb));
                }
            }
        }
    }

    CheckItem::new("Disk Space", "Minimum 20GB available")
        .with_status(Status::Warning)
        .with_message("Could not determine available space")
}

/// Check required commands
fn check_required_commands() -> CheckItem {
    let required = ["curl", "git", "sudo", "systemctl", "ip"];
    let missing: Vec<&str> = required
        .iter()
        .filter(|cmd| !command_exists(cmd))
        .copied()
        .collect();

    if missing.is_empty() {
        CheckItem::new("Required Commands", "curl, git, sudo, systemctl, ip")
            .with_status(Status::Success)
            .with_message("All commands available")
    } else {
        CheckItem::new("Required Commands", "curl, git, sudo, systemctl, ip")
            .with_status(Status::Error)
            .with_message(format!("Missing: {}", missing.join(", ")))
    }
}

/// Check if a port is available
fn check_port_available(port: u16, name: &str) -> CheckItem {
    if let Ok(output) = run_command("ss", &["-tlnp"]) {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let port_str = format!(":{}", port);

        if output_str.lines().any(|l| l.contains(&port_str)) {
            return CheckItem::new(format!("Port {}", port), format!("{} port", name))
                .with_status(Status::Warning)
                .with_message("Port already in use");
        }
    }

    CheckItem::new(format!("Port {}", port), format!("{} port", name))
        .with_status(Status::Success)
        .with_message("Available")
}

// Helper structs and functions

struct OsInfo {
    id: String,
    name: String,
    version: String,
}

fn get_os_info() -> OsInfo {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();

    let mut id = String::new();
    let mut name = String::new();
    let mut version = String::new();

    for line in os_release.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"');
            match key {
                "ID" => id = value.to_lowercase(),
                "NAME" => name = value.to_string(),
                "VERSION_ID" => version = value.to_string(),
                _ => {}
            }
        }
    }

    OsInfo { id, name, version }
}

fn get_kernel_version() -> String {
    fs::read_to_string("/proc/version")
        .unwrap_or_default()
        .split_whitespace()
        .nth(2)
        .unwrap_or("0.0.0")
        .to_string()
}

fn parse_version(version: &str) -> (u32, u32) {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor)
}
