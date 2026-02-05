//! NQR-MicroVM Installer
//!
//! A TUI installer for NQR-MicroVM - Rust Firecracker MicroVM Platform by Nexus.

mod app;
mod installer;
mod theme;
mod ui;

use std::{io, path::PathBuf, time::Duration};

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::{App, InstallConfig, InstallMode, InstallSource, NetworkMode, Screen};

/// NQR-MicroVM Installer - Rust Firecracker MicroVM Platform by Nexus
#[derive(Parser)]
#[command(name = "nqr-installer")]
#[command(author = "Nexus")]
#[command(version)]
#[command(about = "NQR-MicroVM Installer - Rust Firecracker MicroVM Platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install NQR-MicroVM
    Install {
        /// Installation mode
        #[arg(long, value_enum, default_value = "production")]
        mode: CliInstallMode,

        /// Installation directory for binaries
        #[arg(long, default_value = "/opt/nqrust-microvm")]
        install_dir: PathBuf,

        /// Data directory for VMs and images
        #[arg(long, default_value = "/srv/fc")]
        data_dir: PathBuf,

        /// Configuration directory
        #[arg(long, default_value = "/etc/nqrust-microvm")]
        config_dir: PathBuf,

        /// Network mode (nat or bridged)
        #[arg(long, value_enum, default_value = "nat")]
        network_mode: CliNetworkMode,

        /// Bridge name
        #[arg(long, default_value = "fcbr0")]
        bridge_name: String,

        /// Database host
        #[arg(long, default_value = "localhost")]
        db_host: String,

        /// Database port
        #[arg(long, default_value = "5432")]
        db_port: u16,

        /// Database password (will be generated if not provided)
        #[arg(long)]
        db_password: Option<String>,

        /// Include Web UI
        #[arg(long, default_value = "true")]
        with_ui: bool,

        /// Include container runtime (Docker-in-VM support, ~500MB download)
        #[arg(long, default_value = "true")]
        with_container_runtime: bool,

        /// Install Docker (for DockerHub image pulling and container features)
        #[arg(long, default_value = "true")]
        with_docker: bool,

        /// Non-interactive mode
        #[arg(long)]
        non_interactive: bool,

        /// Configuration file (YAML)
        #[arg(long)]
        config: Option<PathBuf>,

        /// Enable debug output
        #[arg(long)]
        debug: bool,

        /// Air-gapped mode: use pre-bundled files from local path (for offline installation)
        /// When enabled, skips all downloads and uses files from the bundle directory.
        /// Alias: --iso-mode (for backward compatibility)
        #[arg(long, alias = "iso-mode")]
        airgap: bool,

        /// Path to the pre-bundled files for ISO mode (default: /opt/nqrust-bundle)
        #[arg(long, default_value = "/opt/nqrust-bundle")]
        bundle_path: PathBuf,
    },
    /// Uninstall NQR-MicroVM
    Uninstall {
        /// Keep VM data
        #[arg(long)]
        keep_data: bool,

        /// Keep database
        #[arg(long)]
        keep_database: bool,

        /// Keep configuration files
        #[arg(long)]
        keep_config: bool,

        /// Remove everything without prompts
        #[arg(long)]
        force: bool,

        /// Non-interactive mode
        #[arg(long)]
        non_interactive: bool,
    },
    /// Full disk installation (for air-gapped ISO boot)
    /// Installs complete OS + NQRust to a target disk
    DiskInstall {
        /// Target disk (e.g., /dev/sda)
        #[arg(long)]
        target_disk: Option<String>,

        /// Hostname for the installed system
        #[arg(long, default_value = "nqrust-node")]
        hostname: String,

        /// Root password for the installed system
        #[arg(long, default_value = "nqrust")]
        root_password: String,

        /// Path to the pre-bundled files (default: /opt/nqrust-bundle)
        #[arg(long, default_value = "/opt/nqrust-bundle")]
        bundle_path: PathBuf,

        /// Non-interactive mode (requires --target-disk)
        #[arg(long)]
        non_interactive: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CliInstallMode {
    Production,
    Dev,
    Manager,
    Agent,
    Minimal,
}

impl From<CliInstallMode> for InstallMode {
    fn from(mode: CliInstallMode) -> Self {
        match mode {
            CliInstallMode::Production => InstallMode::Production,
            CliInstallMode::Dev => InstallMode::Development,
            CliInstallMode::Manager => InstallMode::ManagerOnly,
            CliInstallMode::Agent => InstallMode::AgentOnly,
            CliInstallMode::Minimal => InstallMode::Minimal,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CliNetworkMode {
    Nat,
    Bridged,
    Isolated,
}

impl From<CliNetworkMode> for NetworkMode {
    fn from(mode: CliNetworkMode) -> Self {
        match mode {
            CliNetworkMode::Nat => NetworkMode::Nat,
            CliNetworkMode::Bridged => NetworkMode::Bridged,
            CliNetworkMode::Isolated => NetworkMode::Isolated,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Install {
            mode,
            install_dir,
            data_dir,
            config_dir,
            network_mode,
            bridge_name,
            db_host,
            db_port,
            db_password,
            with_ui,
            with_container_runtime,
            with_docker,
            non_interactive,
            config: _config_file,
            debug: _debug,
            airgap,
            bundle_path,
        }) => {
            // Determine installation source
            let install_source = if airgap {
                InstallSource::LocalBundle(bundle_path)
            } else if mode == CliInstallMode::Dev {
                InstallSource::BuildFromSource
            } else {
                InstallSource::Download
            };

            let config = InstallConfig {
                mode: mode.into(),
                install_source,
                install_dir,
                data_dir,
                config_dir,
                log_dir: PathBuf::from("/var/log/nqrust-microvm"),
                network_mode: network_mode.into(),
                bridge_name,
                db_host,
                db_port,
                db_name: "nqrust".to_string(),
                db_user: "nqrust".to_string(),
                db_password: db_password.unwrap_or_default(),
                with_ui,
                with_container_runtime,
                with_docker,
                non_interactive,
            };

            if non_interactive {
                run_non_interactive(config)
            } else {
                run_tui(config)
            }
        }
        Some(Commands::Uninstall {
            keep_data,
            keep_database,
            keep_config,
            force,
            non_interactive,
        }) => {
            if non_interactive || force {
                run_uninstall_non_interactive(keep_data, keep_database, keep_config)
            } else {
                run_uninstall_tui(keep_data, keep_database, keep_config)
            }
        }
        Some(Commands::DiskInstall {
            target_disk,
            hostname,
            root_password,
            bundle_path,
            non_interactive,
        }) => {
            if non_interactive {
                if target_disk.is_none() {
                    eprintln!("Error: --target-disk is required in non-interactive mode");
                    std::process::exit(1);
                }
                run_disk_install_non_interactive(
                    target_disk.unwrap(),
                    hostname,
                    root_password,
                    bundle_path,
                )
            } else {
                // Use the proper TUI for disk install
                run_disk_install_tui_proper(target_disk, hostname, root_password, bundle_path)
            }
        }
        None => {
            // Default: run interactive TUI installer
            run_tui(InstallConfig::default())
        }
    }
}

/// Run the TUI installer
fn run_tui(config: InstallConfig) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the terminal to prevent overlapping
    terminal.clear()?;

    // Create app state
    let mut app = App::new().with_config(config);

    // Set initial terminal size
    if let Ok((cols, rows)) = terminal::size() {
        app.update_terminal_size(cols, rows);
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Main application loop
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;

        // Handle input with timeout for spinner animation
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Resize(cols, rows) => {
                    app.update_terminal_size(cols, rows);
                }
                Event::Key(key) => {
                    // Global quit always works
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        app.should_quit = true;
                    }

                    // Skip input when terminal is too small (except quit)
                    if !app.terminal_too_small {
                        // Handle input based on current screen
                        match app.screen {
                            Screen::Welcome => handle_welcome_input(app, key.code),
                            Screen::InstallTypeSelect => {
                                handle_install_type_select_input(app, key.code)
                            }
                            Screen::DiskSelect => handle_disk_select_input(app, key.code),
                            Screen::DiskConfig => handle_disk_config_input(app, key.code),
                            Screen::ModeSelect => handle_mode_select_input(app, key.code),
                            Screen::NetworkConfig => handle_network_config_input(app, key.code),
                            Screen::Config => handle_config_input(app, key.code),
                            Screen::Preflight => handle_preflight_input(app, key.code),
                            Screen::Progress => handle_progress_input(app, key.code),
                            Screen::DiskProgress => handle_disk_progress_input(app, key.code),
                            Screen::Verify => handle_verify_input(app, key.code),
                            Screen::Complete => handle_complete_input(app, key.code),
                            Screen::Error => handle_error_input(app, key.code),
                        }
                    }
                }
                _ => {}
            }
        } else {
            // Tick spinner animation
            app.tick_spinner();
        }

        // Handle messages from installation thread
        // Collect all pending messages first to avoid borrow checker issues
        let mut messages = Vec::new();
        if let Some(rx) = &app.install_rx {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }

        // Process collected messages
        for msg in messages {
            use installer::executor::InstallMessage;
            match msg {
                InstallMessage::PhaseStart(phase) => {
                    app.current_phase = Some(phase);
                    app.set_phase_status(phase, app::Status::InProgress);
                }
                InstallMessage::PhaseProgress(_phase, message) => {
                    app.log(app::LogEntry::info(message));
                }
                InstallMessage::PhaseComplete(phase, status) => {
                    app.set_phase_status(phase, status);
                    app.current_phase = None;
                }
                InstallMessage::Log(entry) => {
                    app.log(entry);
                }
                InstallMessage::PreflightResult(checks) => {
                    app.preflight_checks = checks;
                }
                InstallMessage::Error(msg) => {
                    app.error_message = Some(msg);
                    app.screen = app::Screen::Error;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_welcome_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.next_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_install_type_select_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.install_type_selection > 0 {
                app.install_type_selection -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.install_type_selection < app::InstallType::ALL.len() - 1 {
                app.install_type_selection += 1;
            }
        }
        KeyCode::Enter => {
            app.install_type = app::InstallType::ALL[app.install_type_selection];
            // If disk install, load available disks
            if app.install_type == app::InstallType::DiskInstall {
                if let Ok(disks) = installer::disk::list_disks() {
                    app.available_disks = disks;
                }
            }
            app.next_screen();
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_disk_select_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.disk_selection > 0 {
                app.disk_selection -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.available_disks.is_empty() && app.disk_selection < app.available_disks.len() - 1
            {
                app.disk_selection += 1;
            }
        }
        KeyCode::Enter => {
            if !app.available_disks.is_empty() {
                app.detect_network_info();
                // Go to disk config screen (installation starts from there)
                app.next_screen();
            }
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

/// Handle disk config screen input
fn handle_disk_config_input(app: &mut App, key: KeyCode) {
    const DISK_CONFIG_FIELD_COUNT: usize = 6;

    if app.editing {
        match key {
            KeyCode::Enter => {
                ui::screens::disk_config::apply_disk_config_field(app);
                app.editing = false;
                app.input_buffer.clear();
            }
            KeyCode::Esc => {
                app.editing = false;
                app.input_buffer.clear();
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
            }
            _ => {}
        }
    } else {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.disk_config_field > 0 {
                    app.disk_config_field -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.disk_config_field < DISK_CONFIG_FIELD_COUNT - 1 {
                    app.disk_config_field += 1;
                }
            }
            KeyCode::Char('e') | KeyCode::Char(' ') => {
                // Start editing the current field
                app.editing = true;
                app.input_buffer = get_disk_config_field_value(app);
            }
            KeyCode::Enter => {
                // Continue to disk progress and start installation
                start_disk_installation(app);
                app.next_screen();
            }
            KeyCode::Esc => app.prev_screen(),
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        }
    }
}

/// Get current disk config field value for editing
fn get_disk_config_field_value(app: &App) -> String {
    match app.disk_config_field {
        0 => app.disk_hostname.clone(),
        1 => app.disk_root_password.clone(),
        2 => app.config.network_mode.name().to_string(),
        3 => app.config.bridge_name.clone(),
        4 => if app.config.with_docker { "Yes" } else { "No" }.to_string(),
        5 => if app.config.with_container_runtime {
            "Yes"
        } else {
            "No"
        }
        .to_string(),
        _ => String::new(),
    }
}

fn handle_disk_progress_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // If installation complete, go to complete screen
            let success_count = app
                .logs
                .iter()
                .filter(|l| l.level == app::LogLevel::Success)
                .count();
            if success_count >= 10 {
                app.next_screen();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.log_scroll > 0 {
                app.log_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.log_scroll < app.logs.len().saturating_sub(1) {
                app.log_scroll += 1;
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

/// Start disk installation in background
fn start_disk_installation(app: &mut App) {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();

    // Store the receiver so the main loop can receive messages
    app.install_rx = Some(rx);

    let disk = app.available_disks[app.disk_selection].clone();
    let hostname = app.disk_hostname.clone();
    let root_password = app.disk_root_password.clone();
    let bundle_path = app
        .config
        .install_source
        .bundle_path()
        .cloned()
        .unwrap_or_else(|| std::path::PathBuf::from("/opt/nqrust-bundle"));

    // Create a sender that wraps LogEntry into InstallMessage::Log
    let log_tx = tx.clone();

    thread::spawn(move || {
        // Create a channel for LogEntry that converts to InstallMessage
        let (log_sender, log_receiver) = mpsc::channel::<app::LogEntry>();

        // Spawn a thread to forward LogEntry to InstallMessage
        let forward_tx = log_tx.clone();
        thread::spawn(move || {
            while let Ok(log) = log_receiver.recv() {
                let _ = forward_tx.send(installer::executor::InstallMessage::Log(log));
            }
        });

        // Run disk install with real-time logging
        let result = installer::disk::run_disk_install_with_sender(
            &disk,
            &hostname,
            &root_password,
            &bundle_path,
            log_sender,
        );

        if let Err(e) = result {
            let _ = log_tx.send(installer::executor::InstallMessage::Error(e.to_string()));
        }
    });

    // Clear logs for fresh start
    app.logs.clear();
    app.log_scroll = 0;
}

fn handle_mode_select_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.mode_selection > 0 {
                app.mode_selection -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.mode_selection < InstallMode::ALL.len() - 1 {
                app.mode_selection += 1;
            }
        }
        KeyCode::Enter => {
            app.config.mode = InstallMode::ALL[app.mode_selection];
            // Detect interfaces for network config screen
            app.available_interfaces = installer::network::list_interfaces();
            app.interface_selection = 0;
            app.detect_network_info();
            app.next_screen();
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

/// Track which panel is focused on the network config screen
/// false = mode list (left), true = interface list (right)
static mut NETWORK_FOCUS_INTERFACES: bool = false;

fn handle_network_config_input(app: &mut App, key: KeyCode) {
    let selected_mode = app::NetworkMode::ALL[app.network_mode_selection];
    let show_interfaces =
        selected_mode == app::NetworkMode::Bridged && !app.available_interfaces.is_empty();

    // Safety: single-threaded TUI, no concurrent access
    let focus_interfaces = unsafe { NETWORK_FOCUS_INTERFACES } && show_interfaces;

    match key {
        KeyCode::Tab | KeyCode::BackTab => {
            // Toggle between mode list and interface list (only when Bridged)
            if show_interfaces {
                unsafe {
                    NETWORK_FOCUS_INTERFACES = !NETWORK_FOCUS_INTERFACES;
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if focus_interfaces {
                if app.interface_selection > 0 {
                    app.interface_selection -= 1;
                }
            } else if app.network_mode_selection > 0 {
                app.network_mode_selection -= 1;
                // Reset interface focus when mode changes
                unsafe {
                    NETWORK_FOCUS_INTERFACES = false;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if focus_interfaces {
                if app.interface_selection < app.available_interfaces.len().saturating_sub(1) {
                    app.interface_selection += 1;
                }
            } else if app.network_mode_selection < app::NetworkMode::ALL.len() - 1 {
                app.network_mode_selection += 1;
                // Reset interface focus when mode changes
                unsafe {
                    NETWORK_FOCUS_INTERFACES = false;
                }
            }
        }
        KeyCode::Enter => {
            // Apply network mode selection
            app.config.network_mode = app::NetworkMode::ALL[app.network_mode_selection];

            // If bridged mode and interfaces available, record the selected interface
            if app.config.network_mode == app::NetworkMode::Bridged
                && !app.available_interfaces.is_empty()
            {
                let iface = &app.available_interfaces[app.interface_selection];
                app.detected_interface = Some(iface.name.clone());
                app.detected_ip = iface.ip.clone();
                // Re-detect gateway for the selected interface
                app.detected_gateway = installer::network::get_default_gateway();
            } else {
                // For NAT/Isolated, detect network info normally
                app.detect_network_info();
            }

            // Reset focus for next visit
            unsafe {
                NETWORK_FOCUS_INTERFACES = false;
            }

            app.next_screen();
        }
        KeyCode::Esc => {
            unsafe {
                NETWORK_FOCUS_INTERFACES = false;
            }
            app.prev_screen();
        }
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_config_input(app: &mut App, key: KeyCode) {
    if app.editing {
        match key {
            KeyCode::Enter => {
                apply_config_field(app);
                app.editing = false;
                app.input_buffer.clear();
            }
            KeyCode::Esc => {
                app.editing = false;
                app.input_buffer.clear();
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
            }
            _ => {}
        }
    } else {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.config_field > 0 {
                    app.config_field -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.config_field < 8 {
                    // CONFIG_FIELDS.len() - 1 (9 fields, indices 0..=8)
                    app.config_field += 1;
                }
            }
            KeyCode::Char('e') | KeyCode::Char(' ') => {
                // Start editing the selected field
                app.editing = true;
                app.input_buffer = get_current_field_value(app);
            }
            KeyCode::Enter => {
                // Continue to next screen and run preflight checks
                run_preflight_checks(app);
                app.next_screen();
            }
            KeyCode::Esc => app.prev_screen(),
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        }
    }
}

fn handle_preflight_input(app: &mut App, key: KeyCode) {
    let can_continue = app
        .preflight_checks
        .iter()
        .all(|c| c.status != app::Status::Error && c.status != app::Status::InProgress);

    match key {
        KeyCode::Enter if can_continue => {
            // Create message channel
            let (tx, rx) = std::sync::mpsc::channel();
            app.install_rx = Some(rx);

            // Clone config for thread
            let config = app.config.clone();

            // Spawn installation thread
            std::thread::spawn(move || {
                use installer::executor;
                if let Err(e) = executor::run_installation(config, tx.clone()) {
                    let _ = tx.send(executor::InstallMessage::Error(format!(
                        "Installation failed: {}",
                        e
                    )));
                }
            });

            // Transition to Progress screen
            app.next_screen();
        }
        KeyCode::Char('r') => {
            // Run actual preflight checks
            run_preflight_checks(app);
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_progress_input(app: &mut App, key: KeyCode) {
    let all_complete = app.phases.iter().all(|(_, s)| s.is_complete());

    match key {
        KeyCode::Enter if all_complete => app.next_screen(),
        KeyCode::Up | KeyCode::Char('k') => {
            if app.log_scroll > 0 {
                app.log_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.log_scroll < app.logs.len().saturating_sub(1) {
                app.log_scroll += 1;
            }
        }
        _ => {}
    }
}

fn handle_verify_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.next_screen(),
        KeyCode::Char('r') => {
            // Re-run verification
        }
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_complete_input(app: &mut App, key: KeyCode) {
    let needs_reboot = app.config.network_mode == NetworkMode::Bridged;

    match key {
        KeyCode::Char('r') if needs_reboot => {
            // Initiate system reboot
            // Note: The app will quit before the reboot completes
            let _ = std::process::Command::new("sudo").args(["reboot"]).spawn();
            app.should_quit = true;
        }
        KeyCode::Enter | KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_error_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('r') => {
            // Retry - go back to progress
            app.screen = Screen::Progress;
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn get_current_field_value(app: &App) -> String {
    match app.config_field {
        0 => app.config.install_dir.display().to_string(),
        1 => app.config.data_dir.display().to_string(),
        2 => app.config.config_dir.display().to_string(),
        3 => app.config.db_host.clone(),
        4 => app.config.db_port.to_string(),
        5 => app.config.db_name.clone(),
        6 => app.config.db_user.clone(),
        7 => {
            if app.config.with_docker {
                "Yes".to_string()
            } else {
                "No".to_string()
            }
        }
        8 => {
            if app.config.with_container_runtime {
                "Yes".to_string()
            } else {
                "No".to_string()
            }
        }
        _ => String::new(),
    }
}

fn apply_config_field(app: &mut App) {
    let value = app.input_buffer.clone();
    match app.config_field {
        0 => app.config.install_dir = PathBuf::from(&value),
        1 => app.config.data_dir = PathBuf::from(&value),
        2 => app.config.config_dir = PathBuf::from(&value),
        3 => app.config.db_host = value,
        4 => {
            if let Ok(port) = value.parse() {
                app.config.db_port = port;
            }
        }
        5 => app.config.db_name = value,
        6 => app.config.db_user = value,
        7 => {
            // Toggle Docker installation (yes/no/y/n)
            let lower = value.to_lowercase();
            app.config.with_docker =
                lower == "yes" || lower == "y" || lower == "true" || lower == "1";
        }
        8 => {
            // Toggle container runtime installation (yes/no/y/n)
            let lower = value.to_lowercase();
            app.config.with_container_runtime =
                lower == "yes" || lower == "y" || lower == "true" || lower == "1";
        }
        _ => {}
    }
}

fn run_preflight_checks(app: &mut App) {
    use installer::preflight;

    // Run the same preflight checks for both online and air-gapped mode
    app.preflight_checks = preflight::run_preflight_checks();
}

fn run_non_interactive(_config: InstallConfig) -> Result<()> {
    println!("Non-interactive installation not yet implemented");
    println!("Use the TUI installer for now.");
    Ok(())
}

fn run_uninstall_tui(_keep_data: bool, _keep_database: bool, _keep_config: bool) -> Result<()> {
    println!("Uninstall TUI not yet implemented");
    Ok(())
}

fn run_uninstall_non_interactive(
    _keep_data: bool,
    _keep_database: bool,
    _keep_config: bool,
) -> Result<()> {
    println!("Non-interactive uninstall not yet implemented");
    Ok(())
}

/// Run the disk install with proper TUI (for disk-install command)
fn run_disk_install_tui_proper(
    target_disk: Option<String>,
    hostname: String,
    root_password: String,
    bundle_path: PathBuf,
) -> Result<()> {
    use crate::installer::disk;

    // Create config for disk install (treated as ISO mode for air-gapped install)
    let config = InstallConfig {
        install_source: InstallSource::LocalBundle(bundle_path),
        ..InstallConfig::default()
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the terminal to prevent overlapping
    terminal.clear()?;

    // Create app state for disk install
    let mut app = App::new().with_config(config);
    app.disk_hostname = hostname;
    app.disk_root_password = root_password;
    app.install_type = app::InstallType::DiskInstall;

    // Set initial terminal size
    if let Ok((cols, rows)) = terminal::size() {
        app.update_terminal_size(cols, rows);
    }

    // Load available disks
    if let Ok(disks) = disk::list_disks() {
        app.available_disks = disks;
    }

    // If target disk is specified, pre-select it and go directly to config
    if let Some(target) = target_disk {
        if let Some(idx) = app
            .available_disks
            .iter()
            .position(|d| d.path.to_string_lossy() == target || d.name == target)
        {
            app.disk_selection = idx;
            app.screen = Screen::DiskConfig; // Skip disk select, go to config
        } else {
            // Disk not found, start from disk select
            app.screen = Screen::DiskSelect;
        }
    } else {
        // Start from disk select screen
        app.screen = Screen::DiskSelect;
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Run disk install in non-interactive mode
fn run_disk_install_non_interactive(
    target_disk: String,
    hostname: String,
    root_password: String,
    bundle_path: PathBuf,
) -> Result<()> {
    use crate::installer::disk;

    println!("NQRust-MicroVM Full Disk Installation (Non-Interactive)");
    println!("Target disk: {}", target_disk);
    println!("Hostname: {}", hostname);
    println!("Bundle path: {}\n", bundle_path.display());

    // Get disk info
    let disks = disk::list_disks()?;
    let selected_disk = disks
        .iter()
        .find(|d| d.path.to_string_lossy() == target_disk || d.name == target_disk)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Disk {} not found", target_disk))?;

    // Run disk installation
    let mut logs = Vec::new();
    disk::run_disk_install(
        &selected_disk,
        &hostname,
        &root_password,
        &bundle_path,
        &mut logs,
    )?;

    // Print logs
    for log in &logs {
        println!("{}", log);
    }

    Ok(())
}
