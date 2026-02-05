//! Network configuration screen.
//!
//! Allows the user to select a network mode (Bridged, NAT, Isolated)
//! and, when Bridged mode is selected, pick a physical network interface.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::{App, NetworkMode},
    theme::{styles, symbols},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Network Configuration ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected_mode = NetworkMode::ALL[app.network_mode_selection];
    let show_interfaces = selected_mode == NetworkMode::Bridged;

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(1),  // Spacing
            Constraint::Min(8),    // Network mode list + optional interface list
            Constraint::Length(6), // Description box
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Header
    let header = Paragraph::new(Text::from(vec![Line::from(Span::styled(
        "Choose the network mode for your MicroVM bridge:",
        styles::text(),
    ))]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Main content area: mode list + optional interface list
    if show_interfaces && !app.available_interfaces.is_empty() {
        // Split horizontally: mode list on left, interface list on right
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[2]);

        render_mode_list(frame, app, content_chunks[0]);
        render_interface_list(frame, app, content_chunks[1]);
    } else {
        // Just the mode list, full width
        render_mode_list(frame, app, chunks[2]);
    }

    // Description box
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::primary())
        .title(format!(" {} Mode ", selected_mode.name()))
        .title_style(styles::primary());

    let mut desc_lines = vec![Line::from(Span::styled(
        selected_mode.description(),
        styles::text(),
    ))];

    // Add bridge name info
    desc_lines.push(Line::from(""));
    desc_lines.push(Line::from(vec![
        Span::styled("Bridge: ", styles::muted()),
        Span::styled(&app.config.bridge_name, styles::info()),
    ]));

    // If Bridged mode and we have an interface selected, show it
    if show_interfaces && !app.available_interfaces.is_empty() {
        let iface = &app.available_interfaces[app.interface_selection];
        desc_lines.push(Line::from(vec![
            Span::styled("Uplink: ", styles::muted()),
            Span::styled(&iface.name, styles::info()),
            Span::styled(
                if iface.is_up { " (UP)" } else { " (DOWN)" },
                if iface.is_up {
                    styles::success()
                } else {
                    styles::warning()
                },
            ),
        ]));
    }

    let description = Paragraph::new(desc_lines).block(desc_block);
    frame.render_widget(description, chunks[3]);

    // Key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("↑/↓", styles::key_hint()),
        Span::styled(" Navigate  ", styles::muted()),
        Span::styled("Tab", styles::key_hint()),
        Span::styled(" Switch Panel  ", styles::muted()),
        Span::styled("Enter", styles::key_hint()),
        Span::styled(" Continue  ", styles::muted()),
        Span::styled("Esc", styles::key_hint()),
        Span::styled(" Back  ", styles::muted()),
        Span::styled("q", styles::key_hint()),
        Span::styled(" Quit", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[4]);
}

fn render_mode_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = NetworkMode::ALL
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let selected = i == app.network_mode_selection;
            let prefix = if selected {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            let style = if selected {
                styles::highlight()
            } else {
                styles::text()
            };

            let icon = match mode {
                NetworkMode::Bridged => "🌐",
                NetworkMode::Nat => "🔒",
                NetworkMode::Isolated => "🔌",
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(format!("{} ", icon)),
                Span::styled(mode.name(), style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Network Mode ")
                .title_style(styles::secondary()),
        )
        .highlight_style(styles::highlight());

    let mut state = ListState::default();
    state.select(Some(app.network_mode_selection));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_interface_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .available_interfaces
        .iter()
        .enumerate()
        .map(|(i, iface)| {
            let selected = i == app.interface_selection;
            let prefix = if selected {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            let style = if selected {
                styles::highlight()
            } else {
                styles::text()
            };

            let status_style = if iface.is_up {
                styles::success()
            } else {
                styles::warning()
            };

            let type_label = if iface.is_wireless { "wifi" } else { "eth" };

            let ip_str = iface
                .ip
                .as_deref()
                .unwrap_or("no IP");

            let speed_str = iface
                .speed
                .as_deref()
                .unwrap_or("");

            let default_marker = if iface.is_default { " *" } else { "" };

            let mut spans = vec![
                Span::styled(prefix, style),
                Span::styled(&iface.name, style),
                Span::styled(
                    format!(" [{}]", type_label),
                    if selected { style } else { styles::muted() },
                ),
                Span::styled(
                    if iface.is_up { " UP" } else { " DOWN" },
                    status_style,
                ),
            ];

            if !ip_str.is_empty() && ip_str != "no IP" {
                spans.push(Span::styled(
                    format!(" {}", ip_str),
                    if selected { style } else { styles::info() },
                ));
            }

            if !speed_str.is_empty() {
                spans.push(Span::styled(
                    format!(" {}", speed_str),
                    if selected { style } else { styles::muted() },
                ));
            }

            if !default_marker.is_empty() {
                spans.push(Span::styled(
                    default_marker.to_string(),
                    styles::primary(),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Uplink Interface (* = default) ")
                .title_style(styles::secondary()),
        )
        .highlight_style(styles::highlight());

    let mut state = ListState::default();
    state.select(Some(app.interface_selection));
    frame.render_stateful_widget(list, area, &mut state);
}
