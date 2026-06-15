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

/// Let swtpm write the per-VM TPM state under the agent's run dir. Ubuntu ships
/// an AppArmor profile for swtpm that confines its state/socket paths and denies
/// the platform's run dir (e.g. `/srv/fc`), so a TPM-enabled QEMU VM (Windows 11
/// / measured boot) fails to start with "Permission denied". We add the
/// profile's documented local include (enforce mode is preserved — this grants
/// one path, it does not disable the profile) and reload it. No-op where the
/// profile isn't present (non-AppArmor distros, or swtpm without a profile).
pub fn configure_swtpm_apparmor(run_dir: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();
    let profile = "/etc/apparmor.d/usr.bin.swtpm";
    if !Path::new(profile).exists() {
        logs.push(LogEntry::info(
            "swtpm AppArmor profile not present — skipping (unconfined or non-AppArmor host)",
        ));
        return Ok(logs);
    }
    logs.push(LogEntry::info(
        "Allowing swtpm to write TPM state under the run dir (AppArmor)...",
    ));
    let run_dir = run_dir.trim_end_matches('/');
    let local = "/etc/apparmor.d/local/usr.bin.swtpm";
    let content = format!(
        "# NQRust-MicroVM: per-VM swtpm state lives under the run dir\n  {run_dir}/** rwk,"
    );
    let write_cmd = format!("echo '{content}' | sudo tee {local} > /dev/null");
    let _ = run_command("sh", &["-c", &write_cmd]);
    let output = run_sudo("apparmor_parser", &["-r", profile])?;
    if output.status.success() {
        logs.push(LogEntry::success(
            "swtpm AppArmor updated — TPM state allowed under the run dir (enforce kept)",
        ));
    } else {
        logs.push(LogEntry::warning(
            "Failed to reload swtpm AppArmor profile — TPM-enabled VMs may not start",
        ));
    }
    Ok(logs)
}

/// libguestfs (virt-v2v's engine) builds its appliance with `supermin`, which
/// must read the host kernel at `/boot/vmlinuz-*`. On Debian/Ubuntu those are
/// mode 0600 (root-only), so the non-root manager's V2V import fails with
/// "supermin exited with error status 1". Make the installed kernels
/// world-readable so virt-v2v works for the manager service. Best-effort; a
/// no-op where /boot/vmlinuz-* isn't present (e.g. some distros / containers).
pub fn configure_libguestfs_kernel_readable() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();
    // chmod every installed kernel image; a glob via sh keeps it simple and
    // covers future kernels present at install time.
    let out = run_command("sh", &["-c", "ls /boot/vmlinuz-* 2>/dev/null | wc -l"]);
    let count = out
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if count == "0" || count.is_empty() {
        logs.push(LogEntry::info(
            "no /boot/vmlinuz-* present — skipping libguestfs kernel-readable fix",
        ));
        return Ok(logs);
    }
    logs.push(LogEntry::info(
        "Making host kernel readable so virt-v2v (libguestfs) works for the non-root manager...",
    ));
    let _ = run_command("sh", &["-c", "sudo chmod 0644 /boot/vmlinuz-* 2>/dev/null"]);
    logs.push(LogEntry::success(
        "Kernel made readable for libguestfs/virt-v2v (V2V import)",
    ));
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
