//! Error screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{app::App, theme::styles};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::error())
        .title(" Installation Error ")
        .title_alignment(Alignment::Center)
        .title_style(styles::error());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Error header
            Constraint::Min(10),   // Error message
            Constraint::Length(6), // Recovery options
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Error header
    let header_text = Text::from(vec![
        Line::from(Span::styled("✗ Installation failed", styles::error())),
        Line::from(Span::styled(
            "An error occurred during installation.",
            styles::text(),
        )),
    ]);
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Error message box
    let error_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::error())
        .title(" Error Details ")
        .title_style(styles::error());

    let error_msg = app
        .error_message
        .as_deref()
        .unwrap_or("Unknown error occurred");

    let error_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(error_msg, styles::text())),
        Line::from(""),
    ]);

    let error = Paragraph::new(error_text)
        .block(error_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(error, chunks[1]);

    // Recovery options
    let recovery_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border())
        .title(" Recovery Options ")
        .title_style(styles::warning());

    let recovery_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  • ", styles::warning()),
            Span::styled(
                "Check the installation logs at /var/log/nqrust-install/",
                styles::text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • ", styles::warning()),
            Span::styled(
                "Run with --debug flag for more detailed output",
                styles::text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • ", styles::warning()),
            Span::styled(
                "Try running the uninstaller and reinstalling",
                styles::text(),
            ),
        ]),
        Line::from(""),
    ];

    let recovery = Paragraph::new(recovery_lines).block(recovery_block);
    frame.render_widget(recovery, chunks[2]);

    // Key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("r", styles::key_hint()),
        Span::styled(" Retry  ", styles::muted()),
        Span::styled("Esc", styles::key_hint()),
        Span::styled(" Go back  ", styles::muted()),
        Span::styled("q", styles::key_hint()),
        Span::styled(" Quit", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[3]);
}
