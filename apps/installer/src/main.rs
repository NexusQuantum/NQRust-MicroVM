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
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::{App, InstallConfig, InstallMode, NetworkMode, Screen};

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

        /// Include container runtime
        #[arg(long)]
        with_container_runtime: bool,

        /// Non-interactive mode
        #[arg(long)]
        non_interactive: bool,

        /// Configuration file (YAML)
        #[arg(long)]
        config: Option<PathBuf>,

        /// Enable debug output
        #[arg(long)]
        debug: bool,
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
}

impl From<CliNetworkMode> for NetworkMode {
    fn from(mode: CliNetworkMode) -> Self {
        match mode {
            CliNetworkMode::Nat => NetworkMode::Nat,
            CliNetworkMode::Bridged => NetworkMode::Bridged,
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
            non_interactive,
            config: _config_file,
            debug: _debug,
        }) => {
            let config = InstallConfig {
                mode: mode.into(),
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new().with_config(config);

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
            if let Event::Key(key) = event::read()? {
                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.should_quit = true;
                }

                // Handle input based on current screen
                match app.screen {
                    Screen::Welcome => handle_welcome_input(app, key.code),
                    Screen::ModeSelect => handle_mode_select_input(app, key.code),
                    Screen::Config => handle_config_input(app, key.code),
                    Screen::Preflight => handle_preflight_input(app, key.code),
                    Screen::Progress => handle_progress_input(app, key.code),
                    Screen::Verify => handle_verify_input(app, key.code),
                    Screen::Complete => handle_complete_input(app, key.code),
                    Screen::Error => handle_error_input(app, key.code),
                }
            }
        } else {
            // Tick spinner animation
            app.tick_spinner();
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
            app.next_screen();
        }
        KeyCode::Esc => app.prev_screen(),
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
                    // CONFIG_FIELDS.len() - 1
                    app.config_field += 1;
                }
            }
            KeyCode::Enter => {
                // Start editing
                app.editing = true;
                app.input_buffer = get_current_field_value(app);
            }
            KeyCode::Tab => {
                // Continue to next screen
                init_preflight_checks(app);
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
        KeyCode::Enter if can_continue => app.next_screen(),
        KeyCode::Char('r') => {
            init_preflight_checks(app);
            // TODO: Actually run checks
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
    match key {
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
        3 => app.config.network_mode.name().to_string(),
        4 => app.config.bridge_name.clone(),
        5 => app.config.db_host.clone(),
        6 => app.config.db_port.to_string(),
        7 => app.config.db_name.clone(),
        8 => app.config.db_user.clone(),
        _ => String::new(),
    }
}

fn apply_config_field(app: &mut App) {
    let value = app.input_buffer.clone();
    match app.config_field {
        0 => app.config.install_dir = PathBuf::from(&value),
        1 => app.config.data_dir = PathBuf::from(&value),
        2 => app.config.config_dir = PathBuf::from(&value),
        3 => {
            app.config.network_mode = if value.to_lowercase() == "bridged" {
                NetworkMode::Bridged
            } else {
                NetworkMode::Nat
            };
        }
        4 => app.config.bridge_name = value,
        5 => app.config.db_host = value,
        6 => {
            if let Ok(port) = value.parse() {
                app.config.db_port = port;
            }
        }
        7 => app.config.db_name = value,
        8 => app.config.db_user = value,
        _ => {}
    }
}

fn init_preflight_checks(app: &mut App) {
    use app::{CheckItem, Status};

    app.preflight_checks = vec![
        CheckItem::new("Architecture", "x86_64 required").with_status(Status::Success),
        CheckItem::new("Operating System", "Ubuntu 22.04+ / Debian 11+ / RHEL 8+")
            .with_status(Status::Success),
        CheckItem::new("Kernel Version", "4.14 or newer").with_status(Status::Success),
        CheckItem::new("Systemd", "systemd init required").with_status(Status::Success),
        CheckItem::new("KVM Support", "CPU virtualization enabled").with_status(Status::Success),
        CheckItem::new("Memory", "Minimum 2GB RAM").with_status(Status::Success),
        CheckItem::new("Disk Space", "Minimum 20GB available").with_status(Status::Success),
        CheckItem::new("Required Commands", "curl, git, sudo, systemctl, ip")
            .with_status(Status::Success),
        CheckItem::new("Port 18080", "Manager API port").with_status(Status::Success),
        CheckItem::new("Port 9090", "Agent API port").with_status(Status::Success),
        CheckItem::new("Port 3000", "Web UI port").with_status(Status::Success),
        CheckItem::new("Port 5432", "PostgreSQL port").with_status(Status::Success),
    ];
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
