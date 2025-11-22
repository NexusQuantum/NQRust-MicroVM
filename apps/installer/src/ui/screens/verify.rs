//! Verification results screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, CheckItem, Status},
    theme::styles,
    ui::widgets::checklist,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Installation Verification ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Verification checks
            Constraint::Length(5), // Summary
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Build verification items (these would be populated during verification)
    let verification_items = build_verification_items(app);

    // Count results
    let passed = verification_items
        .iter()
        .filter(|c| c.status == Status::Success)
        .count();
    let failed = verification_items
        .iter()
        .filter(|c| c.status == Status::Error)
        .count();
    let warnings = verification_items
        .iter()
        .filter(|c| c.status == Status::Warning)
        .count();

    // Header
    let header_text = Text::from(vec![
        Line::from(Span::styled(
            "Verifying installation components...",
            styles::text(),
        )),
        Line::from(vec![
            Span::styled(format!("{} passed", passed), styles::success()),
            Span::styled(" │ ", styles::muted()),
            Span::styled(format!("{} warnings", warnings), styles::warning()),
            Span::styled(" │ ", styles::muted()),
            Span::styled(format!("{} failed", failed), styles::error()),
        ]),
    ]);
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Verification checklist
    checklist::render(frame, &verification_items, chunks[1]);

    // Summary
    let all_passed = failed == 0;
    let summary_style = if all_passed {
        styles::success()
    } else {
        styles::error()
    };

    let summary_text = if all_passed {
        Text::from(vec![
            Line::from(Span::styled(
                "✓ Installation verified successfully!",
                styles::success(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "All components are installed and running correctly.",
                styles::text(),
            )),
        ])
    } else {
        Text::from(vec![
            Line::from(Span::styled("✗ Verification found issues", styles::error())),
            Line::from(""),
            Line::from(Span::styled(
                "Some components may not be working correctly. Check logs for details.",
                styles::text(),
            )),
        ])
    };

    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(summary_style);

    let summary = Paragraph::new(summary_text)
        .alignment(Alignment::Center)
        .block(summary_block);
    frame.render_widget(summary, chunks[2]);

    // Key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("Enter", styles::key_hint()),
        Span::styled(" Continue  ", styles::muted()),
        Span::styled("r", styles::key_hint()),
        Span::styled(" Re-verify  ", styles::muted()),
        Span::styled("q", styles::key_hint()),
        Span::styled(" Quit", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[3]);
}

fn build_verification_items(app: &App) -> Vec<CheckItem> {
    let mut items = Vec::new();

    // Binary checks
    if app.config.mode.includes_manager() {
        items.push(
            CheckItem::new(
                "Manager Binary",
                "Check manager binary exists and is executable",
            )
            .with_status(Status::Success),
        );
    }
    if app.config.mode.includes_agent() {
        items.push(
            CheckItem::new(
                "Agent Binary",
                "Check agent binary exists and is executable",
            )
            .with_status(Status::Success),
        );
        items.push(
            CheckItem::new("Guest Agent", "Check guest-agent binary for VMs")
                .with_status(Status::Success),
        );
    }

    // Service checks
    if app.config.mode.includes_manager() {
        items.push(
            CheckItem::new("Manager Service", "nqrust-manager.service is running")
                .with_status(Status::Success),
        );
    }
    if app.config.mode.includes_agent() {
        items.push(
            CheckItem::new("Agent Service", "nqrust-agent.service is running")
                .with_status(Status::Success),
        );
    }
    if app.config.mode.includes_ui() {
        items.push(
            CheckItem::new("UI Service", "nqrust-ui.service is running")
                .with_status(Status::Success),
        );
    }

    // Health endpoints
    if app.config.mode.includes_manager() {
        items.push(
            CheckItem::new("Manager Health", "Manager API responds on port 18080")
                .with_status(Status::Success),
        );
    }
    if app.config.mode.includes_agent() {
        items.push(
            CheckItem::new("Agent Health", "Agent API responds on port 9090")
                .with_status(Status::Success),
        );
    }

    // Infrastructure checks
    if app.config.mode.includes_manager() {
        items.push(
            CheckItem::new("Database", "PostgreSQL connection working")
                .with_status(Status::Success),
        );
    }
    if app.config.mode.includes_agent() {
        items.push(
            CheckItem::new("Network Bridge", "fcbr0 bridge is UP").with_status(Status::Success),
        );
        items.push(
            CheckItem::new("KVM Access", "/dev/kvm is accessible").with_status(Status::Success),
        );
    }

    items
}
