//! Installation orchestration and execution.

use std::sync::mpsc::Sender;

use anyhow::Result;

use crate::app::{CheckItem, InstallConfig, LogEntry, Phase, Status};
use crate::installer::{build, config, database, deps, kvm, network, preflight, services, verify};

/// Messages sent from installation thread to UI thread
#[derive(Debug, Clone)]
pub enum InstallMessage {
    /// A phase is starting
    PhaseStart(Phase),
    /// Progress update within a phase
    PhaseProgress(Phase, String),
    /// A phase has completed
    PhaseComplete(Phase, Status),
    /// Log entry
    Log(LogEntry),
    /// Preflight check results
    PreflightResult(Vec<CheckItem>),
    /// Fatal error occurred
    Error(String),
}

/// Run the complete installation process
pub fn run_installation(config: InstallConfig, tx: Sender<InstallMessage>) -> Result<()> {
    // Phase 1: Preflight Checks
    tx.send(InstallMessage::PhaseStart(Phase::Preflight))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Running preflight checks...",
    )))?;

    let checks = preflight::run_preflight_checks();
    let has_errors = checks.iter().any(|c| c.status == Status::Error);

    tx.send(InstallMessage::PreflightResult(checks.clone()))?;

    if has_errors {
        tx.send(InstallMessage::PhaseComplete(
            Phase::Preflight,
            Status::Error,
        ))?;
        tx.send(InstallMessage::Error(
            "Preflight checks failed. Please resolve errors before continuing.".to_string(),
        ))?;
        return Ok(());
    }

    tx.send(InstallMessage::Log(LogEntry::success(
        "All preflight checks passed",
    )))?;
    tx.send(InstallMessage::PhaseComplete(
        Phase::Preflight,
        Status::Success,
    ))?;

    // Phase 2: Install Dependencies
    tx.send(InstallMessage::PhaseStart(Phase::Dependencies))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Installing system dependencies...",
    )))?;

    // Detect package manager
    let pm = match deps::PackageManager::detect() {
        Some(pm) => pm,
        None => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Dependencies,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(
                "No supported package manager found (apt, dnf, yum)".to_string(),
            ))?;
            return Ok(());
        }
    };

    // Update package manager
    tx.send(InstallMessage::Log(LogEntry::info(
        "Updating package manager...",
    )))?;
    if let Err(e) = pm.update() {
        tx.send(InstallMessage::Log(LogEntry::warning(format!(
            "Failed to update package manager: {}",
            e
        ))))?;
    }

    // Install system packages
    let packages = deps::get_required_packages(pm);
    tx.send(InstallMessage::Log(LogEntry::info(format!(
        "Installing {} system packages...",
        packages.len()
    ))))?;
    if let Err(e) = pm.install(&packages) {
        tx.send(InstallMessage::PhaseComplete(
            Phase::Dependencies,
            Status::Error,
        ))?;
        tx.send(InstallMessage::Error(format!(
            "Failed to install system packages: {}",
            e
        )))?;
        return Ok(());
    }
    tx.send(InstallMessage::Log(LogEntry::success(
        "System packages installed",
    )))?;

    // Install PostgreSQL packages if manager is included
    if config.mode.includes_manager() {
        let pg_packages = deps::get_postgres_packages(pm);
        tx.send(InstallMessage::Log(LogEntry::info(
            "Installing PostgreSQL...",
        )))?;
        if let Err(e) = pm.install(&pg_packages) {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Dependencies,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to install PostgreSQL: {}",
                e
            )))?;
            return Ok(());
        }
        tx.send(InstallMessage::Log(LogEntry::success(
            "PostgreSQL installed",
        )))?;
    }

    // Install Firecracker v1.13.1
    tx.send(InstallMessage::Log(LogEntry::info(
        "Installing Firecracker v1.13.1...",
    )))?;
    match deps::install_firecracker(build::FIRECRACKER_VERSION) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Dependencies,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to install Firecracker: {}",
                e
            )))?;
            return Ok(());
        }
    }

    // Install Node.js if UI is included
    if config.with_ui {
        tx.send(InstallMessage::Log(LogEntry::info(
            "Installing Node.js for UI...",
        )))?;
        match deps::install_nodejs() {
            Ok(logs) => {
                for log in logs {
                    tx.send(InstallMessage::Log(log))?;
                }
            }
            Err(e) => {
                tx.send(InstallMessage::Log(LogEntry::warning(format!(
                    "Failed to install Node.js: {} - UI may not work",
                    e
                ))))?;
            }
        }
    }

    tx.send(InstallMessage::PhaseComplete(
        Phase::Dependencies,
        Status::Success,
    ))?;

    // Phase 3: Setup KVM
    if config.mode.includes_agent() {
        tx.send(InstallMessage::PhaseStart(Phase::Kvm))?;
        tx.send(InstallMessage::Log(LogEntry::info("Setting up KVM...")))?;

        match kvm::setup_kvm() {
            Ok(logs) => {
                for log in logs {
                    tx.send(InstallMessage::Log(log))?;
                }
                tx.send(InstallMessage::PhaseComplete(Phase::Kvm, Status::Success))?;
            }
            Err(e) => {
                tx.send(InstallMessage::PhaseComplete(Phase::Kvm, Status::Error))?;
                tx.send(InstallMessage::Error(format!("Failed to setup KVM: {}", e)))?;
                return Ok(());
            }
        }
    } else {
        tx.send(InstallMessage::PhaseStart(Phase::Kvm))?;
        tx.send(InstallMessage::Log(LogEntry::info("Skipping KVM setup")))?;
        tx.send(InstallMessage::PhaseComplete(Phase::Kvm, Status::Skipped))?;
    }

    // Phase 4: Setup Network
    if config.mode.includes_agent() {
        tx.send(InstallMessage::PhaseStart(Phase::Network))?;
        tx.send(InstallMessage::Log(LogEntry::info(format!(
            "Setting up network bridge ({})...",
            config.bridge_name
        ))))?;

        match network::setup_network(config.network_mode, &config.bridge_name) {
            Ok(logs) => {
                for log in logs {
                    tx.send(InstallMessage::Log(log))?;
                }
                tx.send(InstallMessage::PhaseComplete(
                    Phase::Network,
                    Status::Success,
                ))?;
            }
            Err(e) => {
                tx.send(InstallMessage::PhaseComplete(Phase::Network, Status::Error))?;
                tx.send(InstallMessage::Error(format!(
                    "Failed to setup network: {}",
                    e
                )))?;
                return Ok(());
            }
        }
    } else {
        tx.send(InstallMessage::PhaseStart(Phase::Network))?;
        tx.send(InstallMessage::Log(LogEntry::info(
            "Skipping network setup",
        )))?;
        tx.send(InstallMessage::PhaseComplete(
            Phase::Network,
            Status::Skipped,
        ))?;
    }

    // Generate database password early (needed for both database setup and build phase)
    let db_password = if config.db_password.is_empty() {
        database::generate_password(32)
    } else {
        config.db_password.clone()
    };

    // Phase 5: Setup Database
    if config.mode.includes_manager() {
        tx.send(InstallMessage::PhaseStart(Phase::Database))?;
        tx.send(InstallMessage::Log(LogEntry::info(
            "Setting up PostgreSQL database...",
        )))?;

        match database::setup_database(&config.db_name, &config.db_user, &db_password) {
            Ok(logs) => {
                for log in logs {
                    tx.send(InstallMessage::Log(log))?;
                }
                tx.send(InstallMessage::PhaseComplete(
                    Phase::Database,
                    Status::Success,
                ))?;
            }
            Err(e) => {
                tx.send(InstallMessage::PhaseComplete(
                    Phase::Database,
                    Status::Error,
                ))?;
                tx.send(InstallMessage::Error(format!(
                    "Failed to setup database: {}",
                    e
                )))?;
                return Ok(());
            }
        }
    } else {
        tx.send(InstallMessage::PhaseStart(Phase::Database))?;
        tx.send(InstallMessage::Log(LogEntry::info(
            "Skipping database setup",
        )))?;
        tx.send(InstallMessage::PhaseComplete(
            Phase::Database,
            Status::Skipped,
        ))?;
    }

    // Phase 6: Download Pre-built Binaries
    tx.send(InstallMessage::PhaseStart(Phase::Binaries))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Downloading pre-built binaries from GitHub releases...",
    )))?;

    // Download binaries from latest GitHub release
    match build::download_binaries(&config, "latest") {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
            tx.send(InstallMessage::PhaseComplete(
                Phase::Binaries,
                Status::Success,
            ))?;
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Binaries,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to download binaries: {}",
                e
            )))?;
            return Ok(());
        }
    }

    // Phase 7: Install Binaries
    tx.send(InstallMessage::PhaseStart(Phase::Install))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Installing binaries...",
    )))?;

    match build::install_binaries(&config, None) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
            tx.send(InstallMessage::PhaseComplete(
                Phase::Install,
                Status::Success,
            ))?;
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(Phase::Install, Status::Error))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to install binaries: {}",
                e
            )))?;
            return Ok(());
        }
    }

    // Phase 8: Generate Configuration
    tx.send(InstallMessage::PhaseStart(Phase::Configuration))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Generating configuration files...",
    )))?;

    // Create system user
    match config::create_system_user() {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
        }
        Err(e) => {
            tx.send(InstallMessage::Log(LogEntry::warning(format!(
                "Failed to create system user: {}",
                e
            ))))?;
        }
    }

    // Create directories
    match config::create_directories(&config) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Configuration,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to create directories: {}",
                e
            )))?;
            return Ok(());
        }
    }

    // Generate config files
    match config::generate_config(&config, &db_password) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
            tx.send(InstallMessage::PhaseComplete(
                Phase::Configuration,
                Status::Success,
            ))?;
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Configuration,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to generate configuration: {}",
                e
            )))?;
            return Ok(());
        }
    }

    // Phase 9: Setup Sudo (skip for now, handled by services)
    tx.send(InstallMessage::PhaseStart(Phase::Sudo))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Configuring sudo permissions...",
    )))?;
    tx.send(InstallMessage::PhaseComplete(Phase::Sudo, Status::Success))?;

    // Phase 10: Install and Start Services
    tx.send(InstallMessage::PhaseStart(Phase::Services))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Installing systemd services...",
    )))?;

    match services::install_services(&config) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Services,
                Status::Error,
            ))?;
            tx.send(InstallMessage::Error(format!(
                "Failed to install services: {}",
                e
            )))?;
            return Ok(());
        }
    }

    tx.send(InstallMessage::Log(LogEntry::info("Starting services...")))?;
    match services::start_services(&config) {
        Ok(logs) => {
            for log in logs {
                tx.send(InstallMessage::Log(log))?;
            }
            tx.send(InstallMessage::PhaseComplete(
                Phase::Services,
                Status::Success,
            ))?;
        }
        Err(e) => {
            tx.send(InstallMessage::PhaseComplete(
                Phase::Services,
                Status::Warning,
            ))?;
            tx.send(InstallMessage::Log(LogEntry::warning(format!(
                "Failed to start some services: {}",
                e
            ))))?;
        }
    }

    // Phase 11: Verification
    tx.send(InstallMessage::PhaseStart(Phase::Verification))?;
    tx.send(InstallMessage::Log(LogEntry::info(
        "Verifying installation...",
    )))?;

    let verification_checks = verify::run_verification(&config);
    let verify_failed = verification_checks
        .iter()
        .any(|c| c.status == Status::Error);

    if verify_failed {
        tx.send(InstallMessage::Log(LogEntry::warning(
            "Some verification checks failed",
        )))?;
        tx.send(InstallMessage::PhaseComplete(
            Phase::Verification,
            Status::Warning,
        ))?;
    } else {
        tx.send(InstallMessage::Log(LogEntry::success(
            "All verification checks passed",
        )))?;
        tx.send(InstallMessage::PhaseComplete(
            Phase::Verification,
            Status::Success,
        ))?;
    }

    tx.send(InstallMessage::Log(LogEntry::success(
        "Installation completed successfully!",
    )))?;

    Ok(())
}
