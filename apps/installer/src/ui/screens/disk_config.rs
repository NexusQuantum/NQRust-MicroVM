//! Disk configuration screen for full disk installation.
//! Similar to the config screen but with disk-specific options.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{app::App, theme::styles};

/// Configuration fields for disk installation
const DISK_CONFIG_FIELDS: &[(&str, &str)] = &[
    ("Hostname", "Host name for the installed system"),
    ("Root Password", "Password for root user"),
    ("Network Mode", "NAT / Bridged"),
    ("Bridge Name", "Network bridge name (fcbr0)"),
    ("Install Docker", "Yes/No (for DockerHub features)"),
    ("Container Runtime", "Yes/No (Docker-in-VM support)"),
];

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Disk Installation Configuration ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with disk info
            Constraint::Min(12),   // Config fields
            Constraint::Length(4), // Summary
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Header with selected disk info
    let disk_info =
        if !app.available_disks.is_empty() && app.disk_selection < app.available_disks.len() {
            let disk = &app.available_disks[app.disk_selection];
            format!("{} ({})", disk.path.display(), disk.size_human)
        } else {
            "No disk selected".to_string()
        };

    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("Target Disk: ", styles::muted()),
            Span::styled(&disk_info, styles::primary()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Configure installation options:",
            styles::text(),
        )),
    ]))
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
            DISK_CONFIG_FIELDS
                .iter()
                .map(|_| Constraint::Length(3))
                .chain(std::iter::once(Constraint::Min(0)))
                .collect::<Vec<_>>(),
        )
        .split(field_area);

    // Render each field
    for (i, (label, _hint)) in DISK_CONFIG_FIELDS.iter().enumerate() {
        let selected = i == app.disk_config_field;
        let value = get_disk_field_value(app, i);
        let editing = selected && app.editing;

        render_field(frame, field_rows[i], label, &value, selected, editing, app);
    }

    // Summary
    let summary_text = Text::from(vec![Line::from(vec![
        Span::styled("Network: ", styles::muted()),
        Span::styled(app.config.network_mode.name(), styles::info()),
        Span::styled(" │ ", styles::muted()),
        Span::styled("Docker: ", styles::muted()),
        Span::styled(
            if app.config.with_docker { "Yes" } else { "No" },
            styles::primary(),
        ),
        Span::styled(" │ ", styles::muted()),
        Span::styled("Container Runtime: ", styles::muted()),
        Span::styled(
            if app.config.with_container_runtime {
                "Yes"
            } else {
                "No"
            },
            styles::primary(),
        ),
    ])])
    .alignment(Alignment::Center);
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
            Span::styled("e/Space", styles::key_hint()),
            Span::styled(" Edit  ", styles::muted()),
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" Start Install  ", styles::muted()),
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

    // Mask password display
    let masked_value = if label.contains("Password") && !editing {
        "*".repeat(display_value.len().min(8))
    } else {
        display_value
    };

    let value_text = Paragraph::new(masked_value)
        .style(styles::text())
        .block(value_block);
    frame.render_widget(value_text, chunks[1]);
}

fn get_disk_field_value(app: &App, field: usize) -> String {
    match field {
        0 => app.disk_hostname.clone(),
        1 => app.disk_root_password.clone(),
        2 => app.config.network_mode.name().to_string(),
        3 => app.config.bridge_name.clone(),
        4 => if app.config.with_docker { "Yes" } else { "No" }.to_string(),
        5 => if app.config.with_container_runtime {
            "Yes"
        } else {
            "No"
        }
        .to_string(),
        _ => String::new(),
    }
}

/// Apply a disk config field value from input buffer
pub fn apply_disk_config_field(app: &mut App) {
    let value = app.input_buffer.clone();
    match app.disk_config_field {
        0 => app.disk_hostname = value,
        1 => app.disk_root_password = value,
        2 => {
            app.config.network_mode = if value.to_lowercase() == "bridged" {
                crate::app::NetworkMode::Bridged
            } else {
                crate::app::NetworkMode::Nat
            };
        }
        3 => app.config.bridge_name = value,
        4 => {
            let lower = value.to_lowercase();
            app.config.with_docker =
                lower == "yes" || lower == "y" || lower == "true" || lower == "1";
        }
        5 => {
            let lower = value.to_lowercase();
            app.config.with_container_runtime =
                lower == "yes" || lower == "y" || lower == "true" || lower == "1";
        }
        _ => {}
    }
}
