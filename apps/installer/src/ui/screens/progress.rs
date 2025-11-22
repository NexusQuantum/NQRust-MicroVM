//! Installation progress screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, Phase, Status},
    theme::styles,
    ui::widgets::{log_viewer, phase_progress},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Installation Progress ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Current phase header
            Constraint::Length(15), // Phase progress sidebar
            Constraint::Min(10),    // Log viewer
            Constraint::Length(3),  // Key hints
        ])
        .split(inner);

    // Current phase header
    let current_phase = app.current_phase;
    let (phase_name, phase_status) = if let Some(phase) = current_phase {
        (phase.name(), app.phase_status(phase))
    } else {
        ("Preparing...", Status::Pending)
    };

    let spinner = if phase_status == Status::InProgress {
        format!("{} ", app.spinner())
    } else {
        String::new()
    };

    let header_text = Text::from(vec![
        Line::from(vec![
            Span::styled(spinner, styles::primary()),
            Span::styled(phase_name, styles::header()),
        ]),
        Line::from(Span::styled(
            get_phase_description(current_phase),
            styles::muted(),
        )),
    ]);
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Phase progress
    phase_progress::render(frame, &app.phases, current_phase, chunks[1]);

    // Log viewer
    log_viewer::render(frame, &app.logs, app.log_scroll, chunks[2]);

    // Key hints
    let all_complete = app.phases.iter().all(|(_, s)| s.is_complete());
    let hints = if all_complete {
        Text::from(vec![Line::from(vec![
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" Continue to verification", styles::muted()),
        ])])
    } else {
        Text::from(vec![Line::from(vec![
            Span::styled("↑/↓", styles::key_hint()),
            Span::styled(" Scroll logs  ", styles::muted()),
            Span::styled("Ctrl+C", styles::key_hint()),
            Span::styled(" Abort installation", styles::muted()),
        ])])
    };
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[3]);
}

fn get_phase_description(phase: Option<Phase>) -> &'static str {
    match phase {
        Some(Phase::Preflight) => "Verifying system requirements...",
        Some(Phase::Dependencies) => "Installing required packages and tools...",
        Some(Phase::Kvm) => "Configuring KVM virtualization...",
        Some(Phase::Network) => "Setting up network bridge...",
        Some(Phase::Database) => "Configuring PostgreSQL database...",
        Some(Phase::Binaries) => "Building or downloading binaries...",
        Some(Phase::Install) => "Installing binaries to system...",
        Some(Phase::Configuration) => "Generating configuration files...",
        Some(Phase::Sudo) => "Configuring sudo permissions...",
        Some(Phase::Services) => "Installing and starting systemd services...",
        Some(Phase::Verification) => "Verifying installation...",
        None => "Initializing installation...",
    }
}
