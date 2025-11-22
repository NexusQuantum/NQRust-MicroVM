//! Pre-flight check results screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, Status},
    theme::styles,
    ui::widgets::checklist,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Pre-flight Checks ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with summary
            Constraint::Min(10),   // Check list
            Constraint::Length(4), // Status summary
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Count statuses
    let total = app.preflight_checks.len();
    let passed = app
        .preflight_checks
        .iter()
        .filter(|c| c.status == Status::Success)
        .count();
    let warnings = app
        .preflight_checks
        .iter()
        .filter(|c| c.status == Status::Warning)
        .count();
    let failed = app
        .preflight_checks
        .iter()
        .filter(|c| c.status == Status::Error)
        .count();
    let pending = app
        .preflight_checks
        .iter()
        .filter(|c| c.status == Status::Pending || c.status == Status::InProgress)
        .count();

    // Header with progress
    let header_text = if pending > 0 {
        Text::from(vec![
            Line::from(vec![
                Span::styled(app.spinner(), styles::primary()),
                Span::styled(" Running pre-flight checks...", styles::text()),
            ]),
            Line::from(vec![Span::styled(
                format!("Completed: {}/{}", total - pending, total),
                styles::muted(),
            )]),
        ])
    } else {
        Text::from(vec![
            Line::from(Span::styled("Pre-flight checks complete", styles::text())),
            Line::from(vec![
                Span::styled(format!("{} passed", passed), styles::success()),
                Span::styled(" │ ", styles::muted()),
                Span::styled(format!("{} warnings", warnings), styles::warning()),
                Span::styled(" │ ", styles::muted()),
                Span::styled(format!("{} failed", failed), styles::error()),
            ]),
        ])
    };
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Checklist
    checklist::render(frame, &app.preflight_checks, chunks[1]);

    // Status summary box
    let all_passed = failed == 0;
    let status_style = if all_passed {
        styles::success()
    } else {
        styles::error()
    };

    let status_text = if failed > 0 {
        Text::from(vec![
            Line::from(Span::styled("✗ Some checks failed", styles::error())),
            Line::from(Span::styled(
                "Please resolve the issues above before continuing.",
                styles::muted(),
            )),
        ])
    } else if warnings > 0 {
        Text::from(vec![
            Line::from(Span::styled(
                "⚠ Checks passed with warnings",
                styles::warning(),
            )),
            Line::from(Span::styled(
                "You can continue, but review warnings above.",
                styles::muted(),
            )),
        ])
    } else if pending > 0 {
        Text::from(vec![Line::from(Span::styled(
            "Running checks...",
            styles::primary(),
        ))])
    } else {
        Text::from(vec![
            Line::from(Span::styled("✓ All checks passed", styles::success())),
            Line::from(Span::styled(
                "Ready to proceed with installation.",
                styles::muted(),
            )),
        ])
    };

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(status_style);

    let status = Paragraph::new(status_text)
        .alignment(Alignment::Center)
        .block(status_block);
    frame.render_widget(status, chunks[2]);

    // Key hints
    let can_continue = failed == 0 && pending == 0;
    let hints = if can_continue {
        Text::from(vec![Line::from(vec![
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" Continue  ", styles::muted()),
            Span::styled("Esc", styles::key_hint()),
            Span::styled(" Back  ", styles::muted()),
            Span::styled("r", styles::key_hint()),
            Span::styled(" Re-run checks  ", styles::muted()),
            Span::styled("q", styles::key_hint()),
            Span::styled(" Quit", styles::muted()),
        ])])
    } else {
        Text::from(vec![Line::from(vec![
            Span::styled("Esc", styles::key_hint()),
            Span::styled(" Back  ", styles::muted()),
            Span::styled("r", styles::key_hint()),
            Span::styled(" Re-run checks  ", styles::muted()),
            Span::styled("q", styles::key_hint()),
            Span::styled(" Quit", styles::muted()),
        ])])
    };
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[3]);
}
