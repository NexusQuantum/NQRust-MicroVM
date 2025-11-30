//! Configuration input screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::App, theme::styles};

/// Configuration fields
const CONFIG_FIELDS: &[(&str, &str)] = &[
    ("Install Directory", "/opt/nqrust-microvm"),
    ("Data Directory", "/srv/fc"),
    ("Config Directory", "/etc/nqrust-microvm"),
    ("Network Mode", "NAT / Bridged"),
    ("Bridge Name", "fcbr0"),
    ("Database Host", "localhost"),
    ("Database Port", "5432"),
    ("Database Name", "nqrust"),
    ("Database User", "nqrust"),
    ("Install Docker", "Yes/No (for DockerHub features)"),
    ("Container Runtime", "Yes/No (Docker-in-VM support)"),
];

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Configuration ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header
            Constraint::Min(15),   // Config fields
            Constraint::Length(3), // Summary
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Header
    let header = Paragraph::new(Text::from(vec![Line::from(Span::styled(
        "Configure installation paths and settings:",
        styles::text(),
    ))]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Config fields layout
    let field_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(chunks[1])[1];

    // Split field area into rows
    let field_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            CONFIG_FIELDS
                .iter()
                .map(|_| Constraint::Length(3))
                .chain(std::iter::once(Constraint::Min(0)))
                .collect::<Vec<_>>(),
        )
        .split(field_area);

    // Render each field
    for (i, (label, _default)) in CONFIG_FIELDS.iter().enumerate() {
        let selected = i == app.config_field;
        let value = get_field_value(app, i);
        let editing = selected && app.editing;

        render_field(frame, field_rows[i], label, &value, selected, editing, app);
    }

    // Summary
    let summary_text = Text::from(vec![Line::from(vec![
        Span::styled("Mode: ", styles::muted()),
        Span::styled(app.config.mode.name(), styles::primary()),
        Span::styled(" │ ", styles::muted()),
        Span::styled("Network: ", styles::muted()),
        Span::styled(app.config.network_mode.name(), styles::info()),
    ])]);
    let summary = Paragraph::new(summary_text).alignment(Alignment::Center);
    frame.render_widget(summary, chunks[2]);

    // Key hints
    let hints = if app.editing {
        Text::from(vec![Line::from(vec![
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" Confirm  ", styles::muted()),
            Span::styled("Esc", styles::key_hint()),
            Span::styled(" Cancel", styles::muted()),
        ])])
    } else {
        Text::from(vec![Line::from(vec![
            Span::styled("↑/↓", styles::key_hint()),
            Span::styled(" Navigate  ", styles::muted()),
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" Edit  ", styles::muted()),
            Span::styled("Tab", styles::key_hint()),
            Span::styled(" Continue  ", styles::muted()),
            Span::styled("Esc", styles::key_hint()),
            Span::styled(" Back", styles::muted()),
        ])])
    };
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[3]);
}

fn render_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    app: &App,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(30)])
        .split(area);

    // Label
    let label_style = if selected {
        styles::primary()
    } else {
        styles::muted()
    };
    let label_text = Paragraph::new(format!("{}:", label)).style(label_style);
    frame.render_widget(label_text, chunks[0]);

    // Value with input box
    let border_style = if editing {
        styles::primary()
    } else if selected {
        styles::info()
    } else {
        styles::border()
    };

    let display_value = if editing {
        format!("{}_", &app.input_buffer)
    } else {
        value.to_string()
    };

    let value_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if selected {
            BorderType::Rounded
        } else {
            BorderType::Plain
        })
        .border_style(border_style);

    let value_text = Paragraph::new(display_value)
        .style(styles::text())
        .block(value_block);
    frame.render_widget(value_text, chunks[1]);
}

fn get_field_value(app: &App, field: usize) -> String {
    match field {
        0 => app.config.install_dir.display().to_string(),
        1 => app.config.data_dir.display().to_string(),
        2 => app.config.config_dir.display().to_string(),
        3 => app.config.network_mode.name().to_string(),
        4 => app.config.bridge_name.clone(),
        5 => app.config.db_host.clone(),
        6 => app.config.db_port.to_string(),
        7 => app.config.db_name.clone(),
        8 => app.config.db_user.clone(),
        9 => {
            if app.config.with_docker {
                "Yes".to_string()
            } else {
                "No".to_string()
            }
        }
        10 => {
            if app.config.with_container_runtime {
                "Yes".to_string()
            } else {
                "No".to_string()
            }
        }
        _ => String::new(),
    }
}
