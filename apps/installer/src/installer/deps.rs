//! Dependency installation module.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use crate::app::LogEntry;
use crate::installer::{command_exists, run_command, run_sudo};

/// Package manager type
#[derive(Debug, Clone, Copy)]
pub enum PackageManager {
    Apt,
    Dnf,
    Yum,
}

impl PackageManager {
    /// Detect the system package manager
    pub fn detect() -> Option<Self> {
        if command_exists("apt-get") {
            Some(PackageManager::Apt)
        } else if command_exists("dnf") {
            Some(PackageManager::Dnf)
        } else if command_exists("yum") {
            Some(PackageManager::Yum)
        } else {
            None
        }
    }

    /// Update package lists
    pub fn update(&self) -> Result<()> {
        match self {
            PackageManager::Apt => {
                run_sudo("apt-get", &["update", "-qq"])?;
            }
            PackageManager::Dnf | PackageManager::Yum => {
                // DNF/YUM don't need explicit update
            }
        }
        Ok(())
    }

    /// Install packages
    pub fn install(&self, packages: &[&str]) -> Result<()> {
        let mut args = match self {
            PackageManager::Apt => vec!["apt-get", "install", "-y", "-qq"],
            PackageManager::Dnf => vec!["dnf", "install", "-y", "-q"],
            PackageManager::Yum => vec!["yum", "install", "-y", "-q"],
        };
        args.extend(packages);

        let output = run_sudo(args[0], &args[1..])?;
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to install packages: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}

/// Required system packages (no build tools needed - we download pre-built binaries)
pub fn get_required_packages(pm: PackageManager) -> Vec<&'static str> {
    match pm {
        PackageManager::Apt => vec![
            "curl",
            "screen",
            "iproute2",
            "iptables",
            "bridge-utils",
            "dnsmasq",
            "net-tools",
            "lsof",
        ],
        PackageManager::Dnf | PackageManager::Yum => vec![
            "curl",
            "screen",
            "iproute",
            "iptables",
            "bridge-utils",
            "dnsmasq",
            "net-tools",
            "lsof",
        ],
    }
}

/// PostgreSQL packages (no dev libraries needed)
pub fn get_postgres_packages(pm: PackageManager) -> Vec<&'static str> {
    match pm {
        PackageManager::Apt => vec!["postgresql", "postgresql-contrib"],
        PackageManager::Dnf | PackageManager::Yum => {
            vec!["postgresql-server", "postgresql-contrib"]
        }
    }
}

/// Install Rust toolchain
pub fn install_rust() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if Rust is already installed
    if let Ok(output) = run_command("rustc", &["--version"]) {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!(
                "Rust already installed: {}",
                version.trim()
            )));

            // Check minimum version
            if version.contains("1.70") || version.contains("1.7") || version.contains("1.8") {
                return Ok(logs);
            }
        }
    }

    logs.push(LogEntry::info("Installing Rust toolchain..."));

    // Download and run rustup
    let output = run_command(
        "sh",
        &[
            "-c",
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
        ],
    )?;

    if !output.status.success() {
        logs.push(LogEntry::error("Failed to install Rust"));
        return Err(anyhow!("Rust installation failed"));
    }

    logs.push(LogEntry::success("Rust toolchain installed"));

    // Add musl target
    logs.push(LogEntry::info("Adding musl target for static builds..."));
    let _ = run_command(
        "sh",
        &[
            "-c",
            ". $HOME/.cargo/env && rustup target add x86_64-unknown-linux-musl",
        ],
    );

    Ok(logs)
}

/// Install Firecracker
pub fn install_firecracker(version: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if already installed
    if let Ok(output) = run_command("firecracker", &["--version"]) {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!(
                "Firecracker already installed: {}",
                ver.trim()
            )));
            return Ok(logs);
        }
    }

    logs.push(LogEntry::info(format!(
        "Downloading Firecracker {}...",
        version
    )));

    let arch = "x86_64";
    let url = format!(
        "https://github.com/firecracker-microvm/firecracker/releases/download/v{}/firecracker-v{}-{}.tgz",
        version, version, arch
    );

    // Download
    let tmp_dir = "/tmp/firecracker-install";
    let _ = fs::create_dir_all(tmp_dir);

    let download_cmd = format!("curl -sSL '{}' | tar -xz -C {}", url, tmp_dir);
    let output = run_command("sh", &["-c", &download_cmd])?;

    if !output.status.success() {
        logs.push(LogEntry::error("Failed to download Firecracker"));
        return Err(anyhow!("Firecracker download failed"));
    }

    logs.push(LogEntry::info(
        "Installing Firecracker to /usr/local/bin...",
    ));

    // Find and install binary
    let binary_name = format!(
        "release-v{}-{}/firecracker-v{}-{}",
        version, arch, version, arch
    );
    let install_cmd = format!(
        "sudo cp {}/{} /usr/local/bin/firecracker && sudo chmod +x /usr/local/bin/firecracker",
        tmp_dir, binary_name
    );
    let output = run_command("sh", &["-c", &install_cmd])?;

    if !output.status.success() {
        logs.push(LogEntry::error("Failed to install Firecracker binary"));
        return Err(anyhow!("Firecracker installation failed"));
    }

    // Install jailer too
    let jailer_name = format!("release-v{}-{}/jailer-v{}-{}", version, arch, version, arch);
    let _ = run_command(
        "sh",
        &[
            "-c",
            &format!(
                "sudo cp {}/{} /usr/local/bin/jailer && sudo chmod +x /usr/local/bin/jailer",
                tmp_dir, jailer_name
            ),
        ],
    );

    logs.push(LogEntry::success(format!(
        "Firecracker {} installed",
        version
    )));

    // Cleanup
    let _ = fs::remove_dir_all(tmp_dir);

    Ok(logs)
}

/// Install Node.js
pub fn install_nodejs() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if already installed
    if let Ok(output) = run_command("node", &["--version"]) {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!(
                "Node.js already installed: {}",
                version.trim()
            )));

            // Check if pnpm is installed
            if !command_exists("pnpm") {
                logs.push(LogEntry::info("Installing pnpm..."));
                let _ = run_command("npm", &["install", "-g", "pnpm"]);
                logs.push(LogEntry::success("pnpm installed"));
            }

            return Ok(logs);
        }
    }

    logs.push(LogEntry::info("Installing Node.js 20.x LTS..."));

    // Detect package manager
    let pm = PackageManager::detect().ok_or_else(|| anyhow!("No package manager found"))?;

    match pm {
        PackageManager::Apt => {
            // Use NodeSource repository
            let output = run_command(
                "sh",
                &[
                    "-c",
                    "curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -",
                ],
            )?;

            if !output.status.success() {
                logs.push(LogEntry::error("Failed to add NodeSource repository"));
                return Err(anyhow!("NodeSource setup failed"));
            }

            pm.install(&["nodejs"])?;
        }
        PackageManager::Dnf | PackageManager::Yum => {
            let output = run_command(
                "sh",
                &[
                    "-c",
                    "curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -",
                ],
            )?;

            if !output.status.success() {
                logs.push(LogEntry::error("Failed to add NodeSource repository"));
                return Err(anyhow!("NodeSource setup failed"));
            }

            pm.install(&["nodejs"])?;
        }
    }

    logs.push(LogEntry::success("Node.js installed"));

    // Install pnpm
    logs.push(LogEntry::info("Installing pnpm..."));
    let _ = run_command("npm", &["install", "-g", "pnpm"]);
    logs.push(LogEntry::success("pnpm installed"));

    Ok(logs)
}

/// Install Docker
pub fn install_docker() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if already installed
    if let Ok(output) = run_command("docker", &["--version"]) {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!(
                "Docker already installed: {}",
                version.trim()
            )));

            // Ensure Docker service is enabled and running
            let _ = run_sudo("systemctl", &["enable", "docker"]);
            let _ = run_sudo("systemctl", &["start", "docker"]);

            return Ok(logs);
        }
    }

    logs.push(LogEntry::info("Installing Docker..."));

    // Use official Docker install script
    let output = run_command("sh", &["-c", "curl -fsSL https://get.docker.com | sudo sh"])?;

    if !output.status.success() {
        logs.push(LogEntry::error("Failed to install Docker"));
        return Err(anyhow!("Docker installation failed"));
    }

    logs.push(LogEntry::success("Docker installed"));

    // Enable and start Docker service
    logs.push(LogEntry::info("Enabling Docker service..."));
    let _ = run_sudo("systemctl", &["enable", "docker"]);
    let _ = run_sudo("systemctl", &["start", "docker"]);
    logs.push(LogEntry::success("Docker service enabled and started"));

    // Add current user to docker group (optional, for non-root usage)
    if let Ok(user) = std::env::var("SUDO_USER").or_else(|_| std::env::var("USER")) {
        let _ = run_sudo("usermod", &["-aG", "docker", &user]);
        logs.push(LogEntry::info(format!(
            "Added user '{}' to docker group (re-login required)",
            user
        )));
    }

    Ok(logs)
}

/// Install Docker from bundled .deb packages (for air-gapped/offline mode)
///
/// Docker debs (docker-ce, docker-ce-cli, containerd.io, etc.) are included
/// in the same debs/ directory as system packages. Since install_bundled_packages()
/// already installed them via dpkg, this function just enables and starts the service.
pub fn install_docker_from_bundle() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if Docker was installed by the bundled debs
    if let Ok(output) = run_command("docker", &["--version"]) {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::info(format!(
                "Docker installed from bundle: {}",
                version.trim()
            )));
        } else {
            logs.push(LogEntry::warning(
                "Docker binary found but version check failed",
            ));
        }
    } else {
        logs.push(LogEntry::warning(
            "Docker not found after installing bundled packages - container features may not work",
        ));
        return Ok(logs);
    }

    // Enable and start Docker service
    logs.push(LogEntry::info("Enabling Docker service..."));
    let _ = run_sudo("systemctl", &["enable", "docker"]);
    let _ = run_sudo("systemctl", &["start", "docker"]);
    logs.push(LogEntry::success("Docker service enabled and started"));

    // Enable and start containerd
    let _ = run_sudo("systemctl", &["enable", "containerd"]);
    let _ = run_sudo("systemctl", &["start", "containerd"]);

    // Add nqrust user to docker group
    let _ = run_sudo("usermod", &["-aG", "docker", "nqrust"]);
    logs.push(LogEntry::info(
        "Added 'nqrust' user to docker group",
    ));

    // Also add the installing user if running with sudo
    if let Ok(user) = std::env::var("SUDO_USER").or_else(|_| std::env::var("USER")) {
        if user != "root" && user != "nqrust" {
            let _ = run_sudo("usermod", &["-aG", "docker", &user]);
            logs.push(LogEntry::info(format!(
                "Added user '{}' to docker group (re-login required)",
                user
            )));
        }
    }

    Ok(logs)
}

/// Install SQLx CLI
pub fn install_sqlx_cli() -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    if command_exists("sqlx") {
        logs.push(LogEntry::info("SQLx CLI already installed"));
        return Ok(logs);
    }

    logs.push(LogEntry::info("Installing SQLx CLI..."));

    let output = run_command(
        "sh",
        &[
            "-c",
            ". $HOME/.cargo/env && cargo install sqlx-cli --no-default-features --features postgres",
        ],
    )?;

    if !output.status.success() {
        logs.push(LogEntry::warning("SQLx CLI installation may have failed"));
    } else {
        logs.push(LogEntry::success("SQLx CLI installed"));
    }

    Ok(logs)
}

/// Install packages from bundled .deb files (for air-gapped/offline mode)
///
/// Detects the Ubuntu version and selects the correct subdirectory of .deb packages.
/// Uses a two-pass approach: dpkg -i (may fail on deps), then apt-get install -f --no-download.
pub fn install_bundled_packages(bundle_path: &Path) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Detect Ubuntu version for correct deb selection
    let ubuntu_codename = super::preflight::detect_ubuntu_codename();
    let versioned_dir = bundle_path.join("debs").join(&ubuntu_codename);
    let flat_dir = bundle_path.join("debs");

    let debs_dir = if versioned_dir.exists() {
        logs.push(LogEntry::info(format!(
            "Using version-specific packages for {}",
            ubuntu_codename
        )));
        versioned_dir
    } else if flat_dir.exists() {
        logs.push(LogEntry::warning(format!(
            "No packages for {}, using generic debs/",
            ubuntu_codename
        )));
        flat_dir
    } else {
        logs.push(LogEntry::warning("No bundled packages found"));
        return Ok(logs);
    };

    // Check that there are actually .deb files
    let has_debs = fs::read_dir(&debs_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().map_or(false, |ext| ext == "deb"))
        })
        .unwrap_or(false);

    if !has_debs {
        logs.push(LogEntry::warning(format!(
            "No .deb files found in {:?}",
            debs_dir
        )));
        return Ok(logs);
    }

    logs.push(LogEntry::info(format!(
        "Installing bundled packages from {:?}...",
        debs_dir
    )));

    // Pass 1: dpkg -i (install all debs, may fail on dependency ordering)
    let _ = run_sudo(
        "sh",
        &[
            "-c",
            &format!("dpkg -i {}/*.deb 2>&1 || true", debs_dir.display()),
        ],
    );

    // Pass 2: fix broken dependencies using only local packages (no download)
    let output = run_sudo(
        "sh",
        &["-c", "apt-get install -f -y --no-download 2>&1 || true"],
    )?;

    if output.status.success() {
        logs.push(LogEntry::success("Bundled packages installed"));
    } else {
        logs.push(LogEntry::warning(
            "Some bundled packages may have failed to install",
        ));
    }

    Ok(logs)
}

/// Install Node.js from bundled binary tarball (for air-gapped/offline mode)
///
/// Extracts the official Node.js binary distribution to /usr/local and
/// installs the bundled pnpm standalone binary.
pub fn install_nodejs_from_bundle(bundle_path: &Path) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();
    let node_dir = bundle_path.join("node");

    if !node_dir.exists() {
        return Err(anyhow!(
            "Node.js bundle directory not found at {:?}",
            node_dir
        ));
    }

    // Find node tarball (node-v*-linux-x64.tar.xz)
    let tarball = fs::read_dir(&node_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name().map_or(false, |n| {
                let name = n.to_string_lossy();
                name.starts_with("node-v") && name.ends_with("-linux-x64.tar.xz")
            })
        });

    let tarball = match tarball {
        Some(t) => t,
        None => return Err(anyhow!("No Node.js tarball found in {:?}", node_dir)),
    };

    logs.push(LogEntry::info(format!(
        "Installing Node.js from {:?}...",
        tarball.file_name().unwrap_or_default()
    )));

    // Extract to /usr/local (adds bin/node, bin/npm, bin/npx, include/, lib/, share/)
    let output = run_sudo(
        "tar",
        &[
            "-xJf",
            &tarball.display().to_string(),
            "-C",
            "/usr/local",
            "--strip-components=1",
        ],
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to extract Node.js: {}", stderr));
    }

    // Verify node works
    if let Ok(output) = run_command("node", &["--version"]) {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            logs.push(LogEntry::success(format!(
                "Node.js {} installed",
                version.trim()
            )));
        }
    }

    // Install pnpm from bundle
    let pnpm_src = node_dir.join("pnpm");
    if pnpm_src.exists() {
        let _ = run_sudo(
            "cp",
            &[&pnpm_src.display().to_string(), "/usr/local/bin/pnpm"],
        );
        let _ = run_sudo("chmod", &["+x", "/usr/local/bin/pnpm"]);
        logs.push(LogEntry::success("pnpm installed from bundle"));
    } else {
        // Try installing pnpm via npm (which we just installed)
        logs.push(LogEntry::info(
            "pnpm binary not in bundle, installing via npm...",
        ));
        match run_command("npm", &["install", "-g", "pnpm"]) {
            Ok(o) if o.status.success() => {
                logs.push(LogEntry::success("pnpm installed via npm"));
            }
            _ => {
                logs.push(LogEntry::warning(
                    "Failed to install pnpm - UI service may not work",
                ));
            }
        }
    }

    Ok(logs)
}

/// Install Firecracker from bundled binary (for ISO/offline mode)
pub fn install_firecracker_from_bundle(bundle_path: &Path) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let firecracker_src = bundle_path.join("bin").join("firecracker");
    let jailer_src = bundle_path.join("bin").join("jailer");

    if !firecracker_src.exists() {
        return Err(anyhow!(
            "Firecracker binary not found in bundle at {:?}",
            firecracker_src
        ));
    }

    logs.push(LogEntry::info("Installing Firecracker from bundle..."));

    // Copy firecracker binary
    let output = run_sudo(
        "cp",
        &[
            firecracker_src.to_str().unwrap(),
            "/usr/local/bin/firecracker",
        ],
    )?;

    if !output.status.success() {
        return Err(anyhow!("Failed to copy Firecracker binary"));
    }

    run_sudo("chmod", &["+x", "/usr/local/bin/firecracker"])?;

    // Copy jailer if exists
    if jailer_src.exists() {
        let _ = run_sudo(
            "cp",
            &[jailer_src.to_str().unwrap(), "/usr/local/bin/jailer"],
        );
        let _ = run_sudo("chmod", &["+x", "/usr/local/bin/jailer"]);
        logs.push(LogEntry::info("Jailer installed"));
    }

    logs.push(LogEntry::success("Firecracker installed from bundle"));

    Ok(logs)
}
