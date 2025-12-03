//! Application state machine for the NQR-MicroVM installer.

#![allow(dead_code)]

use std::path::PathBuf;

/// Installation source - where to get binaries and images from
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InstallSource {
    /// Download from internet (default)
    #[default]
    Download,
    /// Build from source
    BuildFromSource,
    /// Use pre-bundled files from local path (for ISO/air-gapped installation)
    LocalBundle(PathBuf),
}

impl InstallSource {
    pub fn is_offline(&self) -> bool {
        matches!(self, InstallSource::LocalBundle(_))
    }

    pub fn bundle_path(&self) -> Option<&PathBuf> {
        match self {
            InstallSource::LocalBundle(path) => Some(path),
            _ => None,
        }
    }
}

/// Installation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallMode {
    /// Full production installation (Manager + Agent + UI)
    #[default]
    Production,
    /// Development build from source
    Development,
    /// Manager only (orchestration service)
    ManagerOnly,
    /// Agent only (worker node)
    AgentOnly,
    /// Minimal installation (Manager + Agent, no UI)
    Minimal,
}

impl InstallMode {
    pub const ALL: [InstallMode; 5] = [
        InstallMode::Production,
        InstallMode::Development,
        InstallMode::ManagerOnly,
        InstallMode::AgentOnly,
        InstallMode::Minimal,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            InstallMode::Production => "Production",
            InstallMode::Development => "Development",
            InstallMode::ManagerOnly => "Manager Only",
            InstallMode::AgentOnly => "Agent Only",
            InstallMode::Minimal => "Minimal",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            InstallMode::Production => "Full installation with Manager, Agent, and Web UI",
            InstallMode::Development => "Build from source for development and testing",
            InstallMode::ManagerOnly => "Central orchestration service only",
            InstallMode::AgentOnly => "Host agent for running VMs (worker node)",
            InstallMode::Minimal => "Manager + Agent without the Web UI",
        }
    }

    pub fn includes_manager(&self) -> bool {
        matches!(
            self,
            InstallMode::Production
                | InstallMode::Development
                | InstallMode::ManagerOnly
                | InstallMode::Minimal
        )
    }

    pub fn includes_agent(&self) -> bool {
        matches!(
            self,
            InstallMode::Production
                | InstallMode::Development
                | InstallMode::AgentOnly
                | InstallMode::Minimal
        )
    }

    pub fn includes_ui(&self) -> bool {
        matches!(self, InstallMode::Production | InstallMode::Development)
    }
}

/// Network configuration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    /// NAT mode with internal bridge (10.0.0.0/24)
    Nat,
    /// Bridged mode connecting to external network
    #[default]
    Bridged,
}

impl NetworkMode {
    pub fn name(&self) -> &'static str {
        match self {
            NetworkMode::Nat => "NAT",
            NetworkMode::Bridged => "Bridged",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NetworkMode::Nat => "Isolated network with NAT (10.0.0.0/24)",
            NetworkMode::Bridged => "VMs get IPs from router (DHCP from external network)",
        }
    }
}

/// Installation configuration
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Installation mode
    pub mode: InstallMode,
    /// Installation source (download, build, or local bundle)
    pub install_source: InstallSource,
    /// Installation directory for binaries
    pub install_dir: PathBuf,
    /// Data directory for VMs and images
    pub data_dir: PathBuf,
    /// Configuration directory
    pub config_dir: PathBuf,
    /// Log directory
    pub log_dir: PathBuf,
    /// Network mode
    pub network_mode: NetworkMode,
    /// Bridge name
    pub bridge_name: String,
    /// Database host (for manager)
    pub db_host: String,
    /// Database port
    pub db_port: u16,
    /// Database name
    pub db_name: String,
    /// Database user
    pub db_user: String,
    /// Database password
    pub db_password: String,
    /// Install UI
    pub with_ui: bool,
    /// Install container runtime
    pub with_container_runtime: bool,
    /// Install Docker (for container features and DockerHub image pulling)
    pub with_docker: bool,
    /// Non-interactive mode
    pub non_interactive: bool,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            mode: InstallMode::default(),
            install_source: InstallSource::default(),
            install_dir: PathBuf::from("/opt/nqrust-microvm"),
            data_dir: PathBuf::from("/srv/fc"),
            config_dir: PathBuf::from("/etc/nqrust-microvm"),
            log_dir: PathBuf::from("/var/log/nqrust-microvm"),
            network_mode: NetworkMode::default(),
            bridge_name: "fcbr0".to_string(),
            db_host: "localhost".to_string(),
            db_port: 5432,
            db_name: "nqrust".to_string(),
            db_user: "nqrust".to_string(),
            db_password: String::new(), // Will be generated
            with_ui: true,
            with_container_runtime: true, // Enable by default for container features
            with_docker: true,            // Enable by default for DockerHub image pulling
            non_interactive: false,
        }
    }
}

/// Current screen in the installer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// Welcome screen with logo
    #[default]
    Welcome,
    /// Mode selection
    ModeSelect,
    /// Configuration input
    Config,
    /// Pre-flight check results
    Preflight,
    /// Installation progress
    Progress,
    /// Verification results
    Verify,
    /// Installation complete
    Complete,
    /// Error screen
    Error,
}

/// Installation phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    Preflight = 1,
    Dependencies = 2,
    Kvm = 3,
    Network = 4,
    Database = 5,
    Binaries = 6,
    Install = 7,
    Images = 8,
    Configuration = 9,
    Sudo = 10,
    Services = 11,
    Verification = 12,
}

impl Phase {
    pub const ALL: [Phase; 12] = [
        Phase::Preflight,
        Phase::Dependencies,
        Phase::Kvm,
        Phase::Network,
        Phase::Database,
        Phase::Binaries,
        Phase::Install,
        Phase::Images,
        Phase::Configuration,
        Phase::Sudo,
        Phase::Services,
        Phase::Verification,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Phase::Preflight => "Pre-flight Checks",
            Phase::Dependencies => "Dependencies",
            Phase::Kvm => "KVM Setup",
            Phase::Network => "Network",
            Phase::Database => "Database",
            Phase::Binaries => "Build/Download",
            Phase::Install => "Installation",
            Phase::Images => "Base Images",
            Phase::Configuration => "Configuration",
            Phase::Sudo => "Sudo Setup",
            Phase::Services => "Services",
            Phase::Verification => "Verification",
        }
    }

    pub fn number(&self) -> u8 {
        *self as u8
    }
}

/// Status of a check or phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Pending,
    InProgress,
    Success,
    Warning,
    Error,
    Skipped,
}

impl Status {
    pub fn symbol(&self) -> &'static str {
        match self {
            Status::Pending => "○",
            Status::InProgress => "◐",
            Status::Success => "✓",
            Status::Warning => "⚠",
            Status::Error => "✗",
            Status::Skipped => "⊘",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self,
            Status::Success | Status::Warning | Status::Error | Status::Skipped
        )
    }
}

/// A check item with status
#[derive(Debug, Clone)]
pub struct CheckItem {
    pub name: String,
    pub description: String,
    pub status: Status,
    pub message: Option<String>,
}

impl CheckItem {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: Status::Pending,
            message: None,
        }
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Log entry level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Success,
    Warning,
    Error,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Success, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }

    pub fn debug(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, message)
    }
}

/// Application state
pub struct App {
    /// Current screen
    pub screen: Screen,
    /// Installation configuration
    pub config: InstallConfig,
    /// Selected mode index (for mode selection screen)
    pub mode_selection: usize,
    /// Pre-flight check items
    pub preflight_checks: Vec<CheckItem>,
    /// Installation phases
    pub phases: Vec<(Phase, Status)>,
    /// Current phase
    pub current_phase: Option<Phase>,
    /// Log entries
    pub logs: Vec<LogEntry>,
    /// Should quit
    pub should_quit: bool,
    /// Error message (for error screen)
    pub error_message: Option<String>,
    /// Scroll offset for log viewer
    pub log_scroll: usize,
    /// Config field index (for config screen)
    pub config_field: usize,
    /// Input buffer (for text input)
    pub input_buffer: String,
    /// Editing mode
    pub editing: bool,
    /// Spinner frame
    pub spinner_frame: usize,
    /// Installation message receiver (from background thread)
    pub install_rx: Option<std::sync::mpsc::Receiver<crate::installer::executor::InstallMessage>>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Welcome,
            config: InstallConfig::default(),
            mode_selection: 0,
            preflight_checks: Vec::new(),
            phases: Phase::ALL.iter().map(|p| (*p, Status::Pending)).collect(),
            current_phase: None,
            logs: Vec::new(),
            should_quit: false,
            error_message: None,
            log_scroll: 0,
            config_field: 0,
            input_buffer: String::new(),
            editing: false,
            spinner_frame: 0,
            install_rx: None,
        }
    }

    pub fn with_config(mut self, config: InstallConfig) -> Self {
        self.config = config;
        self
    }

    /// Navigate to next screen
    pub fn next_screen(&mut self) {
        self.screen = match self.screen {
            Screen::Welcome => Screen::ModeSelect,
            Screen::ModeSelect => Screen::Config,
            Screen::Config => Screen::Preflight,
            Screen::Preflight => Screen::Progress,
            Screen::Progress => Screen::Verify,
            Screen::Verify => Screen::Complete,
            Screen::Complete => Screen::Complete,
            Screen::Error => Screen::Error,
        };
    }

    /// Navigate to previous screen
    pub fn prev_screen(&mut self) {
        self.screen = match self.screen {
            Screen::Welcome => Screen::Welcome,
            Screen::ModeSelect => Screen::Welcome,
            Screen::Config => Screen::ModeSelect,
            Screen::Preflight => Screen::Config,
            Screen::Progress => Screen::Preflight,
            Screen::Verify => Screen::Progress,
            Screen::Complete => Screen::Verify,
            Screen::Error => Screen::Welcome,
        };
    }

    /// Add a log entry
    pub fn log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
        // Auto-scroll to bottom
        if self.logs.len() > 1 {
            self.log_scroll = self.logs.len().saturating_sub(1);
        }
    }

    /// Update spinner frame
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 10;
    }

    /// Get current spinner character
    pub fn spinner(&self) -> &'static str {
        crate::theme::symbols::SPINNER[self.spinner_frame]
    }

    /// Set phase status
    pub fn set_phase_status(&mut self, phase: Phase, status: Status) {
        if let Some(entry) = self.phases.iter_mut().find(|(p, _)| *p == phase) {
            entry.1 = status;
        }
        if status == Status::InProgress {
            self.current_phase = Some(phase);
        }
    }

    /// Get phase status
    pub fn phase_status(&self, phase: Phase) -> Status {
        self.phases
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, s)| *s)
            .unwrap_or(Status::Pending)
    }
}
