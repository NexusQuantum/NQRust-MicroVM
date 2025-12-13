//! Disk installation progress screen.
//! Shows progress of full disk installation.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::{
    app::{App, LogLevel},
    theme::styles,
};

const DISK_INSTALL_STEPS: &[&str] = &[
    "Partitioning disk",
    "Formatting partitions",
    "Copying live system",
    "Mounting filesystems",
    "Generating fstab",
    "Configuring system",
    "Setting up users",
    "Installing bootloader",
    "Installing NQRust",
    "Creating services",
    "Cleaning up",
];

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Installing to Disk ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Progress bar
            Constraint::Length(1), // Spacing
            Constraint::Min(10),   // Log viewer
            Constraint::Length(3), // Status/hints
        ])
        .split(inner);

    // Header with disk info
    let disk_name =
        if !app.available_disks.is_empty() && app.disk_selection < app.available_disks.len() {
            format!("{}", app.available_disks[app.disk_selection].path.display())
        } else {
            "Unknown".to_string()
        };

    let header = Paragraph::new(Text::from(vec![Line::from(vec![
        Span::styled("Target: ", styles::muted()),
        Span::styled(&disk_name, styles::info()),
        Span::styled("  |  Hostname: ", styles::muted()),
        Span::styled(&app.disk_hostname, styles::info()),
    ])]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Calculate progress from logs
    let completed_steps = app
        .logs
        .iter()
        .filter(|l| l.level == LogLevel::Success)
        .count();
    let total_steps = DISK_INSTALL_STEPS.len();
    let progress = (completed_steps as f64 / total_steps as f64).min(1.0);

    // Current step
    let current_step = if completed_steps < total_steps {
        DISK_INSTALL_STEPS[completed_steps]
    } else {
        "Complete"
    };

    // Progress bar
    let progress_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border())
        .title(format!(" {} ", current_step))
        .title_style(styles::primary());

    let gauge = Gauge::default()
        .block(progress_block)
        .gauge_style(styles::success())
        .ratio(progress)
        .label(format!("{:.0}%", progress * 100.0));
    frame.render_widget(gauge, chunks[1]);

    // Log viewer
    let log_items: Vec<ListItem> = app
        .logs
        .iter()
        .map(|log| {
            let style = match log.level {
                LogLevel::Info => styles::text(),
                LogLevel::Success => styles::success(),
                LogLevel::Warning => styles::warning(),
                LogLevel::Error => styles::error(),
                LogLevel::Debug => styles::muted(),
            };
            let prefix = match log.level {
                LogLevel::Info => "  ",
                LogLevel::Success => "✓ ",
                LogLevel::Warning => "⚠ ",
                LogLevel::Error => "✗ ",
                LogLevel::Debug => "  ",
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&log.message, style),
            ]))
        })
        .collect();

    let log_list = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border())
            .title(" Installation Log ")
            .title_style(styles::secondary()),
    );
    frame.render_widget(log_list, chunks[3]);

    // Scrollbar for logs
    if app.logs.len() > 8 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(app.logs.len()).position(app.log_scroll);
        frame.render_stateful_widget(scrollbar, chunks[3], &mut scrollbar_state);
    }

    // Status hints
    let hints = if progress >= 1.0 {
        Text::from(vec![Line::from(vec![
            Span::styled("Installation complete! Press ", styles::success()),
            Span::styled("Enter", styles::key_hint()),
            Span::styled(" to continue", styles::success()),
        ])])
    } else {
        Text::from(vec![Line::from(vec![Span::styled(
            "Installing... Please wait",
            styles::muted(),
        )])])
    };
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[4]);
}
