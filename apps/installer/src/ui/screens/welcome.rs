//! Welcome screen with NQR-MicroVM logo.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::App,
    theme::{styles, COMPANY_NAME, LOGO, PRODUCT_DESCRIPTION, PRODUCT_NAME, VERSION},
};

pub fn render(frame: &mut Frame, _app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into logo area and info area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Top padding
            Constraint::Length(8), // Logo
            Constraint::Length(2), // Spacing
            Constraint::Length(3), // Title and description
            Constraint::Min(1),    // Spacer
            Constraint::Length(3), // Version info
            Constraint::Length(3), // Key hints
            Constraint::Length(1), // Bottom padding
        ])
        .split(inner);

    // Render logo
    let logo_lines: Vec<Line> = LOGO
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| Line::from(Span::styled(line, styles::primary())))
        .collect();

    let logo = Paragraph::new(logo_lines).alignment(Alignment::Center);
    frame.render_widget(logo, chunks[1]);

    // Render title and description
    let title_text = Text::from(vec![
        Line::from(vec![
            Span::styled(PRODUCT_NAME, styles::title()),
            Span::styled(" Installer", styles::header()),
        ]),
        Line::from(Span::styled(PRODUCT_DESCRIPTION, styles::secondary())),
        Line::from(Span::styled(
            format!("by {}", COMPANY_NAME),
            styles::muted(),
        )),
    ]);
    let title = Paragraph::new(title_text).alignment(Alignment::Center);
    frame.render_widget(title, chunks[3]);

    // Render version info
    let version_text = Text::from(vec![
        Line::from(vec![
            Span::styled("Version ", styles::muted()),
            Span::styled(VERSION, styles::info()),
        ]),
        Line::from(Span::styled(
            "Rust Firecracker MicroVM Management Platform",
            styles::muted(),
        )),
    ]);
    let version = Paragraph::new(version_text).alignment(Alignment::Center);
    frame.render_widget(version, chunks[5]);

    // Render key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("Press ", styles::muted()),
        Span::styled("Enter", styles::key_hint()),
        Span::styled(" to continue  •  ", styles::muted()),
        Span::styled("q", styles::key_hint()),
        Span::styled(" to quit", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[6]);
}
