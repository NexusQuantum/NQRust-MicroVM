//! Build and binary installation module.

use std::fs;
use std::path::Path;
use std::process::Output;

use anyhow::{anyhow, Result};

use crate::app::{InstallConfig, LogEntry};
use crate::installer::{run_command, run_sudo};

/// Firecracker version to install
pub const FIRECRACKER_VERSION: &str = "1.13.1";

/// Write build failure debug log
fn write_debug_log(
    component: &str,
    command: &str,
    directory: &Path,
    output: &Output,
) -> Result<String> {
    let debug_log = format!("/tmp/nqr-installer-{}.log", component);
    let output_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let debug_content = format!(
        "=== {} Build Failed ===\n\
         Time: {}\n\
         Command: {}\n\
         Directory: {}\n\
         Exit Code: {:?}\n\n\
         === STDOUT ({} bytes) ===\n{}\n\n\
         === STDERR ({} bytes) ===\n{}\n\n",
        component,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        command,
        directory.display(),
        output.status.code(),
        output.stdout.len(),
        output_str,
        output.stderr.len(),
        stderr_str
    );

    std::fs::write(&debug_log, debug_content)?;
    Ok(debug_log)
}

/// Build binaries from source
pub fn build_from_source(config: &InstallConfig, source_dir: &Path) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    logs.push(LogEntry::info("Building NQR-MicroVM from source..."));

    // Ensure we're in the source directory
    if !source_dir.exists() {
        logs.push(LogEntry::error(format!(
            "Source directory not found: {}",
            source_dir.display()
        )));
        return Err(anyhow!("Source directory not found"));
    }

    // Build manager if needed
    if config.mode.includes_manager() {
        logs.push(LogEntry::info("Building manager..."));

        let output = run_command(
            "sh",
            &[
                "-c",
                &format!(
                    "cd {} && . $HOME/.cargo/env && cargo build --release -p manager 2>&1",
                    source_dir.display()
                ),
            ],
        )?;

        if output.status.success() {
            logs.push(LogEntry::success("Manager built successfully"));
        } else {
            // Write full debug log
            match write_debug_log(
                "manager",
                "cargo build --release -p manager",
                source_dir,
                &output,
            ) {
                Ok(debug_log) => {
                    logs.push(LogEntry::error(format!(
                        "Manager build failed. Full output saved to: {}",
                        debug_log
                    )));
                    logs.push(LogEntry::info(
                        "You can view the log with: cat /tmp/nqr-installer-manager.log".to_string(),
                    ));
                }
                Err(e) => {
                    logs.push(LogEntry::warning(format!(
                        "Failed to write debug log: {}",
                        e
                    )));
                }
            }

            // Log the last 30 lines of output for debugging
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str
                .lines()
                .rev()
                .take(30)
                .collect::<Vec<_>>()
                .iter()
                .rev()
            {
                if !line.trim().is_empty() {
                    logs.push(LogEntry::error(line.to_string()));
                }
            }

            return Err(anyhow!(
                "Manager build failed - see /tmp/nqr-installer-manager.log for full output"
            ));
        }
    }

    // Build agent if needed
    if config.mode.includes_agent() {
        logs.push(LogEntry::info("Building agent..."));

        let output = run_command(
            "sh",
            &[
                "-c",
                &format!(
                    "cd {} && . $HOME/.cargo/env && cargo build --release -p agent 2>&1 | grep -E '(Compiling|Finished)'",
                    source_dir.display()
                ),
            ],
        )?;

        if output.status.success() {
            logs.push(LogEntry::success("Agent built successfully"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::error(format!("Agent build failed: {}", stderr)));
            return Err(anyhow!("Agent build failed"));
        }

        // Build guest-agent with musl for static linking
        logs.push(LogEntry::info("Building guest-agent (static)..."));

        let output = run_command(
            "sh",
            &[
                "-c",
                &format!(
                    "cd {} && . $HOME/.cargo/env && cargo build --release -p guest-agent --target x86_64-unknown-linux-musl 2>&1 | grep -E '(Compiling|Finished)'",
                    source_dir.display()
                ),
            ],
        )?;

        if output.status.success() {
            logs.push(LogEntry::success("Guest-agent built successfully"));
        } else {
            // Try without musl target
            logs.push(LogEntry::warning("Static build failed, trying dynamic..."));

            let output = run_command(
                "sh",
                &[
                    "-c",
                    &format!(
                        "cd {} && . $HOME/.cargo/env && cargo build --release -p guest-agent 2>&1 | grep -E '(Compiling|Finished)'",
                        source_dir.display()
                    ),
                ],
            )?;

            if output.status.success() {
                logs.push(LogEntry::success("Guest-agent built (dynamic)"));
            } else {
                logs.push(LogEntry::error("Guest-agent build failed"));
            }
        }
    }

    // Build UI if needed
    if config.mode.includes_ui() {
        logs.push(LogEntry::info("Building Web UI..."));

        let ui_dir = source_dir.join("apps/ui");
        if ui_dir.exists() {
            // Install dependencies
            logs.push(LogEntry::info("Installing UI dependencies..."));
            let _ = run_command(
                "sh",
                &["-c", &format!("cd {} && pnpm install", ui_dir.display())],
            );

            // Build
            let output = run_command(
                "sh",
                &["-c", &format!("cd {} && pnpm build", ui_dir.display())],
            )?;

            if output.status.success() {
                logs.push(LogEntry::success("Web UI built successfully"));
            } else {
                logs.push(LogEntry::warning("Web UI build may have issues"));
            }
        } else {
            logs.push(LogEntry::warning(
                "UI directory not found, skipping UI build",
            ));
        }
    }

    logs.push(LogEntry::success("Build complete"));

    Ok(logs)
}

/// Download pre-built binaries from GitHub releases
pub fn download_binaries(config: &InstallConfig, version: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let repo = "NexusQuantum/NQRust-MicroVM";
    let base_url = if version == "latest" {
        format!("https://github.com/{}/releases/latest/download", repo)
    } else {
        format!("https://github.com/{}/releases/download/v{}", repo, version)
    };

    logs.push(LogEntry::info(format!(
        "Downloading NQR-MicroVM binaries from {}...",
        if version == "latest" {
            "latest release"
        } else {
            version
        }
    )));

    let download_dir = "/tmp/nqrust-download";
    let _ = fs::create_dir_all(download_dir);

    // Download manager if needed
    if config.mode.includes_manager() {
        logs.push(LogEntry::info("Downloading manager binary..."));

        let url = format!("{}/nqrust-manager-x86_64-unknown-linux-gnu", base_url);
        let output_path = format!("{}/manager", download_dir);

        let output = run_command("curl", &["-fsSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("Manager downloaded"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::error(format!(
                "Failed to download manager: {}",
                stderr
            )));
            return Err(anyhow!("Failed to download manager binary"));
        }
    }

    // Download agent if needed
    if config.mode.includes_agent() {
        logs.push(LogEntry::info("Downloading agent binary..."));

        let url = format!("{}/nqrust-agent-x86_64-unknown-linux-gnu", base_url);
        let output_path = format!("{}/agent", download_dir);

        let output = run_command("curl", &["-fsSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("Agent downloaded"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::error(format!(
                "Failed to download agent: {}",
                stderr
            )));
            return Err(anyhow!("Failed to download agent binary"));
        }

        // Download guest-agent
        logs.push(LogEntry::info("Downloading guest-agent binary..."));

        let url = format!("{}/nqrust-guest-agent-x86_64-linux-musl", base_url);
        let output_path = format!("{}/guest-agent", download_dir);

        let output = run_command("curl", &["-fsSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("Guest-agent downloaded"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::error(format!(
                "Failed to download guest-agent: {}",
                stderr
            )));
            return Err(anyhow!("Failed to download guest-agent binary"));
        }
    }

    // Download UI if needed
    if config.with_ui {
        logs.push(LogEntry::info("Downloading UI package..."));

        let url = format!("{}/nqrust-ui.tar.gz", base_url);
        let output_path = format!("{}/nqrust-ui.tar.gz", download_dir);

        let output = run_command("curl", &["-fsSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("UI package downloaded"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logs.push(LogEntry::warning(format!(
                "Failed to download UI: {}",
                stderr
            )));
            // Don't fail - UI is optional
        }
    }

    logs.push(LogEntry::success("All binaries downloaded successfully"));

    Ok(logs)
}

/// Install binaries to system
pub fn install_binaries(
    config: &InstallConfig,
    source_dir: Option<&Path>,
) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let install_dir = &config.install_dir;
    let bin_dir = install_dir.join("bin");

    logs.push(LogEntry::info(format!(
        "Installing binaries to {}...",
        bin_dir.display()
    )));

    // Create directories
    let _ = run_sudo("mkdir", &["-p", &bin_dir.display().to_string()]);

    // Determine source
    let release_dir = if let Some(src) = source_dir {
        src.join("target/release")
    } else {
        Path::new("/tmp/nqrust-download").to_path_buf()
    };

    // Install manager
    if config.mode.includes_manager() {
        let src = release_dir.join("manager");
        let dst = bin_dir.join("nqrust-manager");

        if src.exists() {
            let _ = run_sudo(
                "cp",
                &[&src.display().to_string(), &dst.display().to_string()],
            );
            let _ = run_sudo("chmod", &["+x", &dst.display().to_string()]);
            logs.push(LogEntry::success("Manager binary installed"));
        } else {
            logs.push(LogEntry::warning("Manager binary not found"));
        }
    }

    // Install agent
    if config.mode.includes_agent() {
        let src = release_dir.join("agent");
        let dst = bin_dir.join("nqrust-agent");

        if src.exists() {
            let _ = run_sudo(
                "cp",
                &[&src.display().to_string(), &dst.display().to_string()],
            );
            let _ = run_sudo("chmod", &["+x", &dst.display().to_string()]);
            logs.push(LogEntry::success("Agent binary installed"));
        } else {
            logs.push(LogEntry::warning("Agent binary not found"));
        }

        // Install guest-agent
        let musl_path = source_dir
            .map(|s| s.join("target/x86_64-unknown-linux-musl/release/guest-agent"))
            .unwrap_or_else(|| release_dir.join("guest-agent"));

        let src = if musl_path.exists() {
            musl_path
        } else {
            release_dir.join("guest-agent")
        };

        let dst = bin_dir.join("guest-agent");

        if src.exists() {
            let _ = run_sudo(
                "cp",
                &[&src.display().to_string(), &dst.display().to_string()],
            );
            let _ = run_sudo("chmod", &["+x", &dst.display().to_string()]);
            logs.push(LogEntry::success("Guest-agent binary installed"));
        } else {
            logs.push(LogEntry::warning("Guest-agent binary not found"));
        }
    }

    // Install UI if needed
    if config.with_ui {
        let ui_tarball = release_dir.join("nqrust-ui.tar.gz");
        let ui_dir = install_dir.join("ui");

        if ui_tarball.exists() {
            logs.push(LogEntry::info("Installing UI..."));

            // Create UI directory
            let _ = run_sudo("mkdir", &["-p", &ui_dir.display().to_string()]);

            // Extract tarball
            let output = run_sudo(
                "tar",
                &[
                    "-xzf",
                    &ui_tarball.display().to_string(),
                    "-C",
                    &ui_dir.display().to_string(),
                ],
            )?;

            if output.status.success() {
                logs.push(LogEntry::success("UI installed"));

                // Install Node.js dependencies if needed
                logs.push(LogEntry::info("Installing UI dependencies..."));
                let _ = run_command(
                    "sh",
                    &[
                        "-c",
                        &format!(
                            "cd {} && npm install --production 2>&1 || pnpm install --prod 2>&1 || true",
                            ui_dir.display()
                        ),
                    ],
                );
            } else {
                logs.push(LogEntry::warning("Failed to extract UI"));
            }
        } else {
            logs.push(LogEntry::warning(
                "UI tarball not found - skipping UI installation",
            ));
        }
    }

    // Set ownership
    let _ = run_sudo(
        "chown",
        &["-R", "root:root", &install_dir.display().to_string()],
    );

    logs.push(LogEntry::success("Binary installation complete"));

    Ok(logs)
}

/// Base images to download
/// Format: (filename, description, is_compressed)
/// Note: container-runtime is compressed (.gz) due to GitHub's 2GB file size limit
pub const BASE_IMAGES: &[(&str, &str, bool)] = &[
    // Kernels
    ("vmlinux-5.10.fc.bin", "Firecracker kernel 5.10", false),
    // Rootfs images
    (
        "alpine-3.18-minimal.ext4",
        "Alpine Linux 3.18 minimal",
        false,
    ),
    ("busybox-1.35.ext4", "BusyBox 1.35", false),
    ("ubuntu-24.04-minimal.ext4", "Ubuntu 24.04 minimal", false),
    // Function runtimes
    ("node-runtime.ext4", "Node.js function runtime", false),
    ("python-runtime.ext4", "Python function runtime", false),
    // Container runtime (optional, large - compressed due to GitHub 2GB limit)
    (
        "container-runtime.ext4",
        "Container runtime (Docker-in-VM)",
        true,
    ),
];

/// Download base images (kernels, rootfs, runtimes)
pub fn download_base_images(config: &InstallConfig, version: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    let repo = "NexusQuantum/NQRust-MicroVM";
    let base_url = if version == "latest" {
        format!("https://github.com/{}/releases/latest/download", repo)
    } else {
        format!("https://github.com/{}/releases/download/v{}", repo, version)
    };

    // Image directory - use data_dir/images to match manager config
    let image_dir = config.data_dir.join("images");
    let image_dir_str = image_dir.display().to_string();

    logs.push(LogEntry::info(format!(
        "Downloading base images to {}...",
        image_dir_str
    )));

    // Create image directory
    let _ = run_sudo("mkdir", &["-p", &image_dir_str]);

    // Download each image
    for (filename, description, is_compressed) in BASE_IMAGES {
        // Skip container runtime if not requested (it's large ~2GB)
        if *filename == "container-runtime.ext4" && !config.with_container_runtime {
            logs.push(LogEntry::info(format!(
                "Skipping {} (container runtime not selected)",
                description
            )));
            continue;
        }

        let dst_path = format!("{}/{}", image_dir_str, filename);

        // Skip if already exists
        if Path::new(&dst_path).exists() {
            logs.push(LogEntry::info(format!(
                "{} already exists, skipping",
                filename
            )));
            continue;
        }

        logs.push(LogEntry::info(format!("Downloading {}...", description)));

        // For compressed images, download .gz and decompress
        if *is_compressed {
            let gz_filename = format!("{}.gz", filename);
            let gz_path = format!("{}/{}", image_dir_str, gz_filename);
            let url = format!("{}/{}", base_url, gz_filename);

            logs.push(LogEntry::info(format!(
                "Downloading compressed {} (~500MB)...",
                gz_filename
            )));

            let output = run_command("curl", &["-fsSL", "-o", &gz_path, &url])?;

            if output.status.success() {
                logs.push(LogEntry::info(format!(
                    "Decompressing {} (this may take a minute)...",
                    gz_filename
                )));

                // Decompress using gunzip
                let output = run_command("gunzip", &["-f", &gz_path])?;

                if output.status.success() {
                    logs.push(LogEntry::success(format!(
                        "{} downloaded and decompressed",
                        description
                    )));
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    logs.push(LogEntry::warning(format!(
                        "Failed to decompress {}: {}",
                        gz_filename, stderr
                    )));
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logs.push(LogEntry::warning(format!(
                    "Failed to download {}: {}",
                    gz_filename, stderr
                )));
            }
        } else {
            // Regular uncompressed download
            let url = format!("{}/{}", base_url, filename);
            let output = run_command("curl", &["-fsSL", "-o", &dst_path, &url])?;

            if output.status.success() {
                logs.push(LogEntry::success(format!("{} downloaded", description)));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logs.push(LogEntry::warning(format!(
                    "Failed to download {}: {}",
                    filename, stderr
                )));
                // Don't fail - images are optional, user can add them later
            }
        }
    }

    // Set permissions
    let _ = run_sudo("chmod", &["-R", "755", &image_dir_str]);

    logs.push(LogEntry::success("Base images download complete"));

    Ok(logs)
}

/// Register downloaded images with the manager database
pub fn register_images_with_manager(config: &InstallConfig) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Only register if manager is installed
    if !config.mode.includes_manager() {
        return Ok(logs);
    }

    logs.push(LogEntry::info(
        "Images will be auto-registered when manager starts",
    ));

    // Note: We could call the manager API here, but it's cleaner to let
    // the manager scan and register images on startup via MANAGER_IMAGE_ROOT

    Ok(logs)
}

/// Verify binaries are installed
pub fn verify_binaries(config: &InstallConfig) -> Result<bool> {
    let bin_dir = config.install_dir.join("bin");

    if config.mode.includes_manager() {
        let manager = bin_dir.join("nqrust-manager");
        if !manager.exists() {
            return Ok(false);
        }
    }

    if config.mode.includes_agent() {
        let agent = bin_dir.join("nqrust-agent");
        if !agent.exists() {
            return Ok(false);
        }

        let guest_agent = bin_dir.join("guest-agent");
        if !guest_agent.exists() {
            return Ok(false);
        }
    }

    Ok(true)
}
