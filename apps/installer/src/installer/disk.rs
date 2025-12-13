//! Disk partitioning and base OS installation module.
//!
//! This module handles:
//! - Disk detection and selection
//! - Partitioning (GPT with EFI, root, and optional swap)
//! - Base Debian installation via debootstrap
//! - System configuration (fstab, hostname, users)
//! - Bootloader installation (GRUB)

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, thread};

use anyhow::{anyhow, Result};

use crate::app::LogEntry;
use crate::installer::{run_command, run_sudo};

/// Information about a disk
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,  // e.g., "sda", "nvme0n1"
    pub path: PathBuf, // e.g., "/dev/sda"
    pub size_bytes: u64,
    pub size_human: String, // e.g., "500G"
    pub model: String,
    pub is_removable: bool,
}

/// Partition layout for installation
#[derive(Debug, Clone)]
pub struct PartitionLayout {
    pub efi_part: Option<String>,  // e.g., "/dev/sda1"
    pub root_part: String,         // e.g., "/dev/sda2"
    pub swap_part: Option<String>, // e.g., "/dev/sda3"
}

/// Target mount point for installation
pub const TARGET_MOUNT: &str = "/mnt/target";

/// List available disks for installation
pub fn list_disks() -> Result<Vec<DiskInfo>> {
    let output = run_command(
        "lsblk",
        &["-d", "-b", "-o", "NAME,SIZE,MODEL,RM", "-n", "-p"],
    )?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in output_str.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[0].trim_start_matches("/dev/");
            let path = PathBuf::from(parts[0]);
            let size_bytes: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

            // Skip very small disks (< 8GB) and the live USB/CD
            if size_bytes < 8 * 1024 * 1024 * 1024 {
                continue;
            }

            let model = parts.get(2..).map(|p| p.join(" ")).unwrap_or_default();
            let is_removable = parts.last().map(|s| *s == "1").unwrap_or(false);

            disks.push(DiskInfo {
                name: name.to_string(),
                path,
                size_bytes,
                size_human: format_size(size_bytes),
                model,
                is_removable,
            });
        }
    }

    Ok(disks)
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else {
        format!("{:.1}G", bytes as f64 / GB as f64)
    }
}

/// Check if system is booted in UEFI mode
pub fn is_uefi() -> bool {
    Path::new("/sys/firmware/efi").exists()
}

/// Partition and format a disk for installation
pub fn partition_disk(disk: &DiskInfo, logs: &mut Vec<LogEntry>) -> Result<PartitionLayout> {
    let disk_path = disk.path.to_str().unwrap();
    let is_nvme = disk.name.starts_with("nvme");
    let part_prefix = if is_nvme { "p" } else { "" };

    logs.push(LogEntry::warning(format!(
        "WARNING: All data on {} will be destroyed!",
        disk_path
    )));

    // Unmount any existing partitions on this disk
    logs.push(LogEntry::info("Unmounting existing partitions..."));
    let _ = run_sudo("umount", &["-R", TARGET_MOUNT]);
    for i in 1..=10 {
        let part = format!("{}{}{}", disk_path, part_prefix, i);
        let _ = run_sudo("umount", &[&part]);
    }
    let _ = run_sudo("swapoff", &["-a"]);

    // Wipe existing partition table
    logs.push(LogEntry::info("Wiping existing partition table..."));
    run_sudo("wipefs", &["-a", disk_path])?;

    // Create new GPT partition table
    logs.push(LogEntry::info("Creating GPT partition table..."));
    run_sudo("parted", &["-s", disk_path, "mklabel", "gpt"])?;

    let layout = if is_uefi() {
        logs.push(LogEntry::info(
            "UEFI mode detected, creating EFI partition...",
        ));

        // Create EFI System Partition (512MB)
        run_sudo(
            "parted",
            &["-s", disk_path, "mkpart", "EFI", "fat32", "1MiB", "513MiB"],
        )?;
        run_sudo("parted", &["-s", disk_path, "set", "1", "esp", "on"])?;

        // Create swap partition (4GB)
        run_sudo(
            "parted",
            &[
                "-s",
                disk_path,
                "mkpart",
                "swap",
                "linux-swap",
                "513MiB",
                "4609MiB",
            ],
        )?;

        // Create root partition (rest of disk)
        run_sudo(
            "parted",
            &["-s", disk_path, "mkpart", "root", "ext4", "4609MiB", "100%"],
        )?;

        let efi_part = format!("{}{}{}", disk_path, part_prefix, 1);
        let swap_part = format!("{}{}{}", disk_path, part_prefix, 2);
        let root_part = format!("{}{}{}", disk_path, part_prefix, 3);

        PartitionLayout {
            efi_part: Some(efi_part),
            root_part,
            swap_part: Some(swap_part),
        }
    } else {
        logs.push(LogEntry::info("BIOS mode detected..."));

        // Create BIOS boot partition (1MB)
        run_sudo(
            "parted",
            &["-s", disk_path, "mkpart", "bios", "1MiB", "2MiB"],
        )?;
        run_sudo("parted", &["-s", disk_path, "set", "1", "bios_grub", "on"])?;

        // Create swap partition (4GB)
        run_sudo(
            "parted",
            &[
                "-s",
                disk_path,
                "mkpart",
                "swap",
                "linux-swap",
                "2MiB",
                "4098MiB",
            ],
        )?;

        // Create root partition (rest of disk)
        run_sudo(
            "parted",
            &["-s", disk_path, "mkpart", "root", "ext4", "4098MiB", "100%"],
        )?;

        let swap_part = format!("{}{}{}", disk_path, part_prefix, 2);
        let root_part = format!("{}{}{}", disk_path, part_prefix, 3);

        PartitionLayout {
            efi_part: None,
            root_part,
            swap_part: Some(swap_part),
        }
    };

    // Wait for kernel to recognize new partitions
    logs.push(LogEntry::info("Waiting for partitions to be recognized..."));
    thread::sleep(Duration::from_secs(2));
    let _ = run_sudo("partprobe", &[disk_path]);
    thread::sleep(Duration::from_secs(1));

    // Format partitions
    logs.push(LogEntry::info("Formatting partitions..."));

    if let Some(ref efi) = layout.efi_part {
        logs.push(LogEntry::info(format!(
            "Formatting {} as FAT32 (EFI)...",
            efi
        )));
        run_sudo("mkfs.fat", &["-F32", "-n", "EFI", efi])?;
    }

    if let Some(ref swap) = layout.swap_part {
        logs.push(LogEntry::info(format!("Formatting {} as swap...", swap)));
        run_sudo("mkswap", &["-L", "swap", swap])?;
    }

    logs.push(LogEntry::info(format!(
        "Formatting {} as ext4...",
        layout.root_part
    )));
    run_sudo("mkfs.ext4", &["-F", "-L", "nqrust-root", &layout.root_part])?;

    logs.push(LogEntry::success("Disk partitioning complete"));

    Ok(layout)
}

/// Mount partitions for installation
pub fn mount_partitions(layout: &PartitionLayout, logs: &mut Vec<LogEntry>) -> Result<()> {
    // Create and mount root
    logs.push(LogEntry::info(format!(
        "Mounting {} to {}...",
        layout.root_part, TARGET_MOUNT
    )));
    fs::create_dir_all(TARGET_MOUNT)?;
    run_sudo("mount", &[&layout.root_part, TARGET_MOUNT])?;

    // Mount EFI partition if present
    if let Some(ref efi) = layout.efi_part {
        let efi_mount = format!("{}/boot/efi", TARGET_MOUNT);
        fs::create_dir_all(&efi_mount)?;
        logs.push(LogEntry::info(format!(
            "Mounting {} to {}...",
            efi, efi_mount
        )));
        run_sudo("mount", &[efi, &efi_mount])?;
    }

    // Enable swap
    if let Some(ref swap) = layout.swap_part {
        logs.push(LogEntry::info(format!("Enabling swap on {}...", swap)));
        let _ = run_sudo("swapon", &[swap]);
    }

    logs.push(LogEntry::success("Partitions mounted"));

    Ok(())
}

/// Install base Debian system using debootstrap
pub fn install_base_system(logs: &mut Vec<LogEntry>) -> Result<()> {
    logs.push(LogEntry::info(
        "Installing base Debian system (this may take several minutes)...",
    ));

    // Check if debootstrap is available
    if run_command("which", &["debootstrap"]).is_err() {
        return Err(anyhow!(
            "debootstrap not found - ensure it's included in the ISO"
        ));
    }

    // Run debootstrap
    let result = run_sudo(
        "debootstrap",
        &[
            "--arch=amd64",
            "--include=linux-image-amd64,grub-efi-amd64,grub-pc,locales,sudo,openssh-server,curl,wget,ca-certificates,gnupg,apt-transport-https,systemd,systemd-sysv,dbus,network-manager,iproute2,iputils-ping,vim,less,bash-completion",
            "bookworm",
            TARGET_MOUNT,
            "http://deb.debian.org/debian",
        ],
    );

    match result {
        Ok(output) if output.status.success() => {
            logs.push(LogEntry::success("Base system installed"));
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::error(format!("debootstrap failed: {}", stderr)));
            Err(anyhow!("debootstrap failed"))
        }
        Err(e) => {
            logs.push(LogEntry::error(format!("Failed to run debootstrap: {}", e)));
            Err(e)
        }
    }
}

/// Configure the installed system
pub fn configure_system(
    layout: &PartitionLayout,
    hostname: &str,
    logs: &mut Vec<LogEntry>,
) -> Result<()> {
    logs.push(LogEntry::info("Configuring installed system..."));

    // Mount virtual filesystems for chroot
    logs.push(LogEntry::info("Mounting virtual filesystems..."));
    run_sudo(
        "mount",
        &["--bind", "/dev", &format!("{}/dev", TARGET_MOUNT)],
    )?;
    run_sudo(
        "mount",
        &["--bind", "/dev/pts", &format!("{}/dev/pts", TARGET_MOUNT)],
    )?;
    run_sudo(
        "mount",
        &["-t", "proc", "proc", &format!("{}/proc", TARGET_MOUNT)],
    )?;
    run_sudo(
        "mount",
        &["-t", "sysfs", "sys", &format!("{}/sys", TARGET_MOUNT)],
    )?;

    if is_uefi() {
        run_sudo(
            "mount",
            &[
                "--bind",
                "/sys/firmware/efi/efivars",
                &format!("{}/sys/firmware/efi/efivars", TARGET_MOUNT),
            ],
        )?;
    }

    // Generate fstab
    logs.push(LogEntry::info("Generating /etc/fstab..."));
    let root_uuid = get_uuid(&layout.root_part)?;
    let mut fstab = "# /etc/fstab - generated by NQRust installer\n".to_string();
    fstab.push_str(&format!(
        "UUID={}  /  ext4  errors=remount-ro  0  1\n",
        root_uuid
    ));

    if let Some(ref efi) = layout.efi_part {
        let efi_uuid = get_uuid(efi)?;
        fstab.push_str(&format!(
            "UUID={}  /boot/efi  vfat  umask=0077  0  1\n",
            efi_uuid
        ));
    }

    if let Some(ref swap) = layout.swap_part {
        let swap_uuid = get_uuid(swap)?;
        fstab.push_str(&format!("UUID={}  none  swap  sw  0  0\n", swap_uuid));
    }

    fs::write(format!("{}/etc/fstab", TARGET_MOUNT), fstab)?;

    // Set hostname
    logs.push(LogEntry::info(format!(
        "Setting hostname to '{}'...",
        hostname
    )));
    fs::write(
        format!("{}/etc/hostname", TARGET_MOUNT),
        format!("{}\n", hostname),
    )?;
    fs::write(
        format!("{}/etc/hosts", TARGET_MOUNT),
        format!("127.0.0.1  localhost\n127.0.1.1  {}\n", hostname),
    )?;

    // Configure locale
    logs.push(LogEntry::info("Configuring locale..."));
    fs::write(
        format!("{}/etc/locale.gen", TARGET_MOUNT),
        "en_US.UTF-8 UTF-8\n",
    )?;
    chroot_run("locale-gen")?;
    fs::write(
        format!("{}/etc/default/locale", TARGET_MOUNT),
        "LANG=en_US.UTF-8\n",
    )?;

    // Set timezone
    logs.push(LogEntry::info("Setting timezone..."));
    let _ = std::fs::remove_file(format!("{}/etc/localtime", TARGET_MOUNT));
    chroot_run("ln -sf /usr/share/zoneinfo/UTC /etc/localtime")?;

    // Enable services
    logs.push(LogEntry::info("Enabling services..."));
    chroot_run("systemctl enable ssh")?;
    chroot_run("systemctl enable NetworkManager")?;

    logs.push(LogEntry::success("System configured"));

    Ok(())
}

/// Install bootloader (GRUB)
pub fn install_bootloader(disk: &DiskInfo, logs: &mut Vec<LogEntry>) -> Result<()> {
    let disk_path = disk.path.to_str().unwrap();

    logs.push(LogEntry::info("Installing GRUB bootloader..."));

    if is_uefi() {
        logs.push(LogEntry::info("Installing GRUB for UEFI..."));
        chroot_run(&format!(
            "grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=nqrust --recheck {}",
            disk_path
        ))?;
    } else {
        logs.push(LogEntry::info("Installing GRUB for BIOS..."));
        chroot_run(&format!(
            "grub-install --target=i386-pc --recheck {}",
            disk_path
        ))?;
    }

    // Generate GRUB config
    logs.push(LogEntry::info("Generating GRUB configuration..."));
    chroot_run("update-grub")?;

    logs.push(LogEntry::success("Bootloader installed"));

    Ok(())
}

/// Create root user and set password
pub fn setup_users(root_password: &str, logs: &mut Vec<LogEntry>) -> Result<()> {
    logs.push(LogEntry::info("Setting up root user..."));

    // Set root password
    let cmd = format!("echo 'root:{}' | chpasswd", root_password);
    chroot_run(&cmd)?;

    // Create nqrust user for services
    chroot_run("useradd --system --no-create-home --shell /usr/sbin/nologin nqrust || true")?;

    logs.push(LogEntry::success("Users configured"));

    Ok(())
}

/// Run a command inside chroot
fn chroot_run(cmd: &str) -> Result<()> {
    let result = run_sudo("chroot", &[TARGET_MOUNT, "sh", "-c", cmd])?;

    if result.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err(anyhow!("chroot command failed: {} - {}", cmd, stderr))
    }
}

/// Get UUID of a partition
fn get_uuid(partition: &str) -> Result<String> {
    let output = run_command("blkid", &["-s", "UUID", "-o", "value", partition])?;
    let uuid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uuid.is_empty() {
        Err(anyhow!("Could not get UUID for {}", partition))
    } else {
        Ok(uuid)
    }
}

/// Unmount all partitions after installation
pub fn unmount_all(logs: &mut Vec<LogEntry>) -> Result<()> {
    logs.push(LogEntry::info("Unmounting filesystems..."));

    // Unmount in reverse order
    let mounts = [
        "/sys/firmware/efi/efivars",
        "/sys",
        "/proc",
        "/dev/pts",
        "/dev",
        "/boot/efi",
        "",
    ];

    for mount in &mounts {
        let path = format!("{}{}", TARGET_MOUNT, mount);
        let _ = run_sudo("umount", &["-l", &path]);
    }

    logs.push(LogEntry::success("Filesystems unmounted"));

    Ok(())
}

/// Full disk installation workflow
pub fn run_disk_installation(
    disk: &DiskInfo,
    hostname: &str,
    root_password: &str,
    logs: &mut Vec<LogEntry>,
) -> Result<PartitionLayout> {
    // Partition disk
    let layout = partition_disk(disk, logs)?;

    // Mount partitions
    mount_partitions(&layout, logs)?;

    // Install base system
    install_base_system(logs)?;

    // Configure system
    configure_system(&layout, hostname, logs)?;

    // Setup users
    setup_users(root_password, logs)?;

    // Install bootloader
    install_bootloader(disk, logs)?;

    Ok(layout)
}
