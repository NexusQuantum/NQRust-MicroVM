//! Status bar widget.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, Screen},
    theme::{styles, PRODUCT_NAME, VERSION},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let screen_name = match app.screen {
        Screen::Welcome => "Welcome",
        Screen::InstallTypeSelect => "Install Type",
        Screen::DiskSelect => "Disk Selection",
        Screen::DiskConfig => "Disk Configuration",
        Screen::ModeSelect => "Mode Selection",
        Screen::NetworkConfig => "Network Config",
        Screen::Config => "Configuration",
        Screen::Preflight => "Pre-flight Checks",
        Screen::Progress => "Installation",
        Screen::DiskProgress => "Disk Installation",
        Screen::Verify => "Verification",
        Screen::Complete => "Complete",
        Screen::Error => "Error",
    };

    let step_num = match app.screen {
        Screen::Welcome => 1,
        Screen::InstallTypeSelect => 2,
        Screen::DiskSelect => 3,
        Screen::DiskConfig => 4,
        Screen::ModeSelect => 2,
        Screen::NetworkConfig => 3,
        Screen::Config => 4,
        Screen::Preflight => 5,
        Screen::Progress => 6,
        Screen::DiskProgress => 5,
        Screen::Verify => 7,
        Screen::Complete => 8,
        Screen::Error => 0,
    };

    let total_steps = 8;

    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", PRODUCT_NAME), styles::primary()),
        Span::styled("│", styles::border()),
        Span::styled(format!(" {} ", screen_name), styles::text()),
        Span::styled("│", styles::border()),
        Span::styled(
            format!(" Step {}/{} ", step_num, total_steps),
            styles::muted(),
        ),
        Span::styled("│", styles::border()),
        Span::styled(format!(" v{} ", VERSION), styles::muted()),
    ]);

    let status_bar = Paragraph::new(status_line);
    frame.render_widget(status_bar, area);
}
