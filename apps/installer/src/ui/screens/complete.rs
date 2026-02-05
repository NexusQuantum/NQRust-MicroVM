//! Installation complete screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, NetworkMode},
    theme::{styles, COMPANY_NAME, PRODUCT_NAME},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::success())
        .title(" Installation Complete ")
        .title_alignment(Alignment::Center)
        .title_style(styles::success());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Success header
            Constraint::Length(1),  // Spacing
            Constraint::Length(10), // Service URLs
            Constraint::Length(1),  // Spacing
            Constraint::Min(8),     // Next steps
            Constraint::Length(3),  // Key hints
        ])
        .split(inner);

    // Success header
    let header_text = Text::from(vec![
        Line::from(Span::styled(
            "✓ Installation completed successfully!",
            styles::success(),
        )),
        Line::from(Span::styled(
            format!("{} is now installed and running.", PRODUCT_NAME),
            styles::text(),
        )),
    ]);
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Service URLs box
    let urls_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border())
        .title(" Access URLs ")
        .title_style(styles::info());

    let host = app.display_host();

    let mut url_lines = Vec::new();
    url_lines.push(Line::from(""));

    if app.config.mode.includes_ui() {
        url_lines.push(Line::from(vec![
            Span::styled("  Web UI:      ", styles::muted()),
            Span::styled(format!("http://{}:3000", host), styles::info()),
        ]));
    }
    if app.config.mode.includes_manager() {
        url_lines.push(Line::from(vec![
            Span::styled("  Manager API: ", styles::muted()),
            Span::styled(format!("http://{}:18080", host), styles::info()),
        ]));
        url_lines.push(Line::from(vec![
            Span::styled("  API Docs:    ", styles::muted()),
            Span::styled(format!("http://{}:18080/swagger-ui/", host), styles::info()),
        ]));
    }
    if app.config.mode.includes_agent() {
        url_lines.push(Line::from(vec![
            Span::styled("  Agent API:   ", styles::muted()),
            Span::styled(format!("http://{}:9090", host), styles::info()),
        ]));
    }
    url_lines.push(Line::from(""));

    let urls = Paragraph::new(url_lines).block(urls_block);
    frame.render_widget(urls, chunks[2]);

    // Next steps box
    let steps_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border())
        .title(" Next Steps ")
        .title_style(styles::primary());

    let steps_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", styles::primary()),
            Span::styled("Check service status: ", styles::text()),
            Span::styled("sudo systemctl status nqrust-*", styles::muted()),
        ]),
        Line::from(vec![
            Span::styled("  2. ", styles::primary()),
            Span::styled("View logs: ", styles::text()),
            Span::styled("journalctl -u nqrust-manager -f", styles::muted()),
        ]),
        Line::from(vec![
            Span::styled("  3. ", styles::primary()),
            Span::styled("Create your first VM via the Web UI or API", styles::text()),
        ]),
        Line::from(""),
    ];

    let steps = Paragraph::new(steps_lines).block(steps_block);
    frame.render_widget(steps, chunks[4]);

    // Key hints - show reboot option for bridged mode (network changes need reboot)
    let needs_reboot = app.config.network_mode == NetworkMode::Bridged;

    let hints = if needs_reboot {
        Text::from(vec![
            Line::from(vec![
                Span::styled("⚠ ", styles::warning()),
                Span::styled(
                    "A reboot is recommended for network changes to take effect",
                    styles::warning(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Press ", styles::muted()),
                Span::styled("r", styles::key_hint()),
                Span::styled(" to reboot now  •  ", styles::muted()),
                Span::styled("q", styles::key_hint()),
                Span::styled(" to exit  •  ", styles::muted()),
                Span::styled(
                    format!("Thank you for using {} by {}", PRODUCT_NAME, COMPANY_NAME),
                    styles::muted(),
                ),
            ]),
        ])
    } else {
        Text::from(vec![Line::from(vec![
            Span::styled("Press ", styles::muted()),
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" or ", styles::muted()),
            Span::styled("q", styles::key_hint()),
            Span::styled(" to exit  •  ", styles::muted()),
            Span::styled(
                format!("Thank you for using {} by {}", PRODUCT_NAME, COMPANY_NAME),
                styles::muted(),
            ),
        ])])
    };
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[5]);
}
