//! NQR-MicroVM Brand Theme
//!
//! Color scheme based on the NQR-MicroVM UI application by Nexus.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

/// Primary brand color - NQR Orange
pub const PRIMARY: Color = Color::Rgb(255, 80, 1); // #FF5001

/// Success color - Green
pub const SUCCESS: Color = Color::Rgb(34, 197, 94); // #22C55E

/// Warning color - Yellow
pub const WARNING: Color = Color::Rgb(234, 179, 8); // #EAB308

/// Error color - Red
pub const ERROR: Color = Color::Rgb(239, 68, 68); // #EF4444

/// Info/Accent color - Blue
pub const INFO: Color = Color::Rgb(59, 130, 246); // #3B82F6

/// Background color - Dark
pub const BACKGROUND: Color = Color::Rgb(26, 26, 26); // #1A1A1A

/// Foreground/Text color - Light
pub const FOREGROUND: Color = Color::Rgb(252, 252, 252); // #FCFCFC

/// Card/Panel background
pub const CARD: Color = Color::Rgb(53, 53, 53); // #353535

/// Border color
pub const BORDER: Color = Color::Rgb(74, 74, 74); // #4A4A4A

/// Muted text color
pub const MUTED: Color = Color::Rgb(107, 114, 128); // #6B7280

/// Secondary text color
pub const SECONDARY: Color = Color::Rgb(156, 163, 175); // #9CA3AF

/// Purple accent (for special highlights)
pub const PURPLE: Color = Color::Rgb(168, 85, 247); // #A855F7

/// Cyan accent (for volume/storage indicators)
pub const CYAN: Color = Color::Rgb(6, 182, 212); // #06B6D4

/// Status check symbols
pub mod symbols {
    pub const CHECK: &str = "✓";
    pub const CROSS: &str = "✗";
    pub const PENDING: &str = "○";
    pub const IN_PROGRESS: &str = "◐";
    pub const ARROW_RIGHT: &str = "▶";
    pub const ARROW_DOWN: &str = "▼";
    pub const BULLET: &str = "•";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// Pre-built styles for common UI elements
pub mod styles {
    use super::*;

    /// Default text style
    pub fn text() -> Style {
        Style::default().fg(FOREGROUND)
    }

    /// Primary brand style (orange)
    pub fn primary() -> Style {
        Style::default().fg(PRIMARY)
    }

    /// Primary text on primary background
    pub fn primary_bold() -> Style {
        Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
    }

    /// Success style (green)
    pub fn success() -> Style {
        Style::default().fg(SUCCESS)
    }

    /// Warning style (yellow)
    pub fn warning() -> Style {
        Style::default().fg(WARNING)
    }

    /// Error style (red)
    pub fn error() -> Style {
        Style::default().fg(ERROR)
    }

    /// Info style (blue)
    pub fn info() -> Style {
        Style::default().fg(INFO)
    }

    /// Muted/dimmed text
    pub fn muted() -> Style {
        Style::default().fg(MUTED)
    }

    /// Secondary text
    pub fn secondary() -> Style {
        Style::default().fg(SECONDARY)
    }

    /// Title style
    pub fn title() -> Style {
        Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
    }

    /// Highlighted/selected item
    pub fn highlight() -> Style {
        Style::default()
            .fg(FOREGROUND)
            .bg(PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    /// Border style
    pub fn border() -> Style {
        Style::default().fg(BORDER)
    }

    /// Active border style
    pub fn border_active() -> Style {
        Style::default().fg(PRIMARY)
    }

    /// Header style
    pub fn header() -> Style {
        Style::default().fg(FOREGROUND).add_modifier(Modifier::BOLD)
    }

    /// Key hint style (for keyboard shortcuts)
    pub fn key_hint() -> Style {
        Style::default().fg(INFO)
    }

    /// Status styles based on check result
    pub fn status_success() -> Style {
        Style::default().fg(SUCCESS)
    }

    pub fn status_warning() -> Style {
        Style::default().fg(WARNING)
    }

    pub fn status_error() -> Style {
        Style::default().fg(ERROR)
    }

    pub fn status_pending() -> Style {
        Style::default().fg(MUTED)
    }

    pub fn status_in_progress() -> Style {
        Style::default().fg(PRIMARY)
    }
}

/// ASCII art logo for NQR-MicroVM
pub const LOGO: &str = r#"
  ███╗   ██╗ ██████╗ ██████╗       ███╗   ███╗██╗ ██████╗██████╗  ██████╗ ██╗   ██╗███╗   ███╗
  ████╗  ██║██╔═══██╗██╔══██╗      ████╗ ████║██║██╔════╝██╔══██╗██╔═══██╗██║   ██║████╗ ████║
  ██╔██╗ ██║██║   ██║██████╔╝█████╗██╔████╔██║██║██║     ██████╔╝██║   ██║██║   ██║██╔████╔██║
  ██║╚██╗██║██║▄▄ ██║██╔══██╗╚════╝██║╚██╔╝██║██║██║     ██╔══██╗██║   ██║╚██╗ ██╔╝██║╚██╔╝██║
  ██║ ╚████║╚██████╔╝██║  ██║      ██║ ╚═╝ ██║██║╚██████╗██║  ██║╚██████╔╝ ╚████╔╝ ██║ ╚═╝ ██║
  ╚═╝  ╚═══╝ ╚══▀▀═╝ ╚═╝  ╚═╝      ╚═╝     ╚═╝╚═╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝   ╚═══╝  ╚═╝     ╚═╝
"#;

/// Compact logo for smaller terminals
pub const LOGO_COMPACT: &str = r#"
  ╔═╗╔═╗ ═══════════════════════════════════════════════════════════════ ╔═╗╔═╗
  ║ ╚╝ ║   NQR-MicroVM  •  Rust Firecracker MicroVM Platform  •  Nexus   ║ ╚╝ ║
  ╚════╝ ═══════════════════════════════════════════════════════════════ ╚════╝
"#;

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name
pub const PRODUCT_NAME: &str = "NQR-MicroVM";

/// Company name
pub const COMPANY_NAME: &str = "Nexus";

/// Full product description
pub const PRODUCT_DESCRIPTION: &str = "Rust Firecracker MicroVM Platform";
