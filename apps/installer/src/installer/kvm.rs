//! KVM setup module.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use crate::app::LogEntry;
use crate::installer::{current_user, run_command, run_sudo};

/// Setup KVM for virtualization
pub fn setup_kvm() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Detect CPU type
    let cpu_flags = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let is_intel = cpu_flags.contains("vmx");
    let is_amd = cpu_flags.contains("svm");

    if !is_intel && !is_amd {
        logs.push(LogEntry::error(
            "No virtualization support detected (vmx/svm)",
        ));
        return Err(anyhow!("CPU does not support virtualization"));
    }

    let cpu_type = if is_intel {
        "Intel (VT-x)"
    } else {
        "AMD (AMD-V)"
    };
    let kvm_module = if is_intel { "kvm_intel" } else { "kvm_amd" };

    logs.push(LogEntry::info(format!("Detected {} CPU", cpu_type)));

    // Load KVM modules
    logs.push(LogEntry::info("Loading KVM modules..."));
    let _ = run_sudo("modprobe", &["kvm"]);
    let output = run_sudo("modprobe", &[kvm_module])?;

    if !output.status.success() {
        logs.push(LogEntry::warning(format!(
            "Failed to load {} module",
            kvm_module
        )));
    } else {
        logs.push(LogEntry::success(format!("{} module loaded", kvm_module)));
    }

    // Make modules persistent
    logs.push(LogEntry::info("Making KVM modules persistent..."));
    let modules_content = format!("kvm\n{}\n", kvm_module);
    let modules_file = "/etc/modules-load.d/kvm.conf";

    let write_cmd = format!(
        "echo '{}' | sudo tee {} > /dev/null",
        modules_content, modules_file
    );
    let _ = run_command("sh", &["-c", &write_cmd]);

    // Create kvm group if needed
    let groups_output = run_command("getent", &["group", "kvm"])?;
    if !groups_output.status.success() {
        logs.push(LogEntry::info("Creating kvm group..."));
        let _ = run_sudo("groupadd", &["kvm"]);
    }

    // Add current user to kvm group
    let user = current_user();
    logs.push(LogEntry::info(format!(
        "Adding user '{}' to kvm group...",
        user
    )));
    let output = run_sudo("usermod", &["-aG", "kvm", &user])?;

    if output.status.success() {
        logs.push(LogEntry::success(format!(
            "User '{}' added to kvm group",
            user
        )));
        logs.push(LogEntry::warning(
            "You may need to log out and back in for group changes to take effect",
        ));
    }

    // Set /dev/kvm permissions
    logs.push(LogEntry::info("Configuring /dev/kvm permissions..."));

    if Path::new("/dev/kvm").exists() {
        let _ = run_sudo("chown", &["root:kvm", "/dev/kvm"]);
        let _ = run_sudo("chmod", &["660", "/dev/kvm"]);
        logs.push(LogEntry::success("/dev/kvm permissions configured"));
    } else {
        logs.push(LogEntry::warning(
            "/dev/kvm not found - KVM may not be fully loaded",
        ));
    }

    // Create udev rule for persistent permissions
    logs.push(LogEntry::info("Creating udev rule for KVM..."));
    let udev_rule = r#"KERNEL=="kvm", GROUP="kvm", MODE="0660""#;
    let udev_file = "/etc/udev/rules.d/99-kvm.rules";

    let write_cmd = format!("echo '{}' | sudo tee {} > /dev/null", udev_rule, udev_file);
    let _ = run_command("sh", &["-c", &write_cmd]);

    // Reload udev rules
    let _ = run_sudo("udevadm", &["control", "--reload-rules"]);
    let _ = run_sudo("udevadm", &["trigger"]);

    logs.push(LogEntry::success("KVM setup complete"));

    // Verify KVM is working
    if let Ok(output) = run_command("ls", &["-la", "/dev/kvm"]) {
        if output.status.success() {
            let info = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!("/dev/kvm: {}", info.trim())));
        }
    }

    Ok(logs)
}

/// Verify KVM is working
pub fn verify_kvm() -> Result<bool> {
    // Check /dev/kvm exists
    if !Path::new("/dev/kvm").exists() {
        return Ok(false);
    }

    // Check KVM module is loaded
    let modules = fs::read_to_string("/proc/modules").unwrap_or_default();
    if !modules.contains("kvm") {
        return Ok(false);
    }

    // Try to access /dev/kvm
    if let Ok(output) = run_command("test", &["-r", "/dev/kvm"]) {
        if !output.status.success() {
            return Ok(false);
        }
    }

    Ok(true)
}
