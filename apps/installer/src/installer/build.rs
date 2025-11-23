//! Build and binary installation module.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use crate::app::{InstallConfig, LogEntry};
use crate::installer::{run_command, run_sudo};

/// Firecracker version to install
pub const FIRECRACKER_VERSION: &str = "1.13.1";

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
            let output_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);

            // Log the last 20 lines of output for debugging
            for line in output_str.lines().rev().take(20).collect::<Vec<_>>().iter().rev() {
                if !line.trim().is_empty() {
                    logs.push(LogEntry::error(line.to_string()));
                }
            }

            if !stderr_str.is_empty() {
                logs.push(LogEntry::error(format!("Stderr: {}", stderr_str)));
            }

            return Err(anyhow!("Manager build failed - check logs above"));
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

    logs.push(LogEntry::info(format!(
        "Downloading NQR-MicroVM binaries v{}...",
        version
    )));

    let base_url = format!(
        "https://github.com/your-org/nqrust-microvm/releases/download/v{}/",
        version
    );

    let download_dir = "/tmp/nqrust-download";
    let _ = fs::create_dir_all(download_dir);

    // Download manager if needed
    if config.mode.includes_manager() {
        logs.push(LogEntry::info("Downloading manager binary..."));

        let url = format!("{}manager-linux-x86_64", base_url);
        let output_path = format!("{}/manager", download_dir);

        let output = run_command("curl", &["-sSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("Manager downloaded"));
        } else {
            logs.push(LogEntry::error("Failed to download manager"));
        }
    }

    // Download agent if needed
    if config.mode.includes_agent() {
        logs.push(LogEntry::info("Downloading agent binary..."));

        let url = format!("{}agent-linux-x86_64", base_url);
        let output_path = format!("{}/agent", download_dir);

        let output = run_command("curl", &["-sSL", "-o", &output_path, &url])?;

        if output.status.success() {
            logs.push(LogEntry::success("Agent downloaded"));
        } else {
            logs.push(LogEntry::error("Failed to download agent"));
        }

        // Download guest-agent
        logs.push(LogEntry::info("Downloading guest-agent binary..."));

        let url = format!("{}guest-agent-linux-x86_64", base_url);
        let output_path = format!("{}/guest-agent", download_dir);

        let _ = run_command("curl", &["-sSL", "-o", &output_path, &url]);
    }

    logs.push(LogEntry::success("Download complete"));

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

    // Set ownership
    let _ = run_sudo(
        "chown",
        &["-R", "root:root", &install_dir.display().to_string()],
    );

    logs.push(LogEntry::success("Binary installation complete"));

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
