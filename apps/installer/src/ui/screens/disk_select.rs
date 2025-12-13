//! Disk selection screen for full disk installation.
//! Shows available disks and allows selection.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::App,
    theme::{styles, symbols},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Select Target Disk ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/warning
            Constraint::Length(1), // Spacing
            Constraint::Min(8),    // Disk list
            Constraint::Length(5), // Details box
            Constraint::Length(4), // Hostname input
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Warning header
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("⚠ ", styles::warning()),
            Span::styled("WARNING: ", styles::warning()),
            Span::styled(
                "This will ERASE ALL DATA on the selected disk!",
                styles::warning(),
            ),
        ]),
        Line::from(Span::styled(
            "Select the disk where you want to install NQRust-MicroVM:",
            styles::text(),
        )),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Build disk list
    if app.available_disks.is_empty() {
        let no_disks = Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled("No suitable disks found.", styles::error())),
            Line::from(Span::styled("Disks must be at least 8GB.", styles::muted())),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Disks ")
                .title_style(styles::secondary()),
        );
        frame.render_widget(no_disks, chunks[2]);
    } else {
        let items: Vec<ListItem> = app
            .available_disks
            .iter()
            .enumerate()
            .map(|(i, disk)| {
                let selected = i == app.disk_selection;
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

                let removable = if disk.is_removable { " [USB]" } else { "" };

                let line = Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{}", disk.path.display()), style),
                    Span::styled(format!("  {}  ", disk.size_human), styles::info()),
                    Span::styled(&disk.model, styles::muted()),
                    Span::styled(removable, styles::warning()),
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
                    .title(" Available Disks ")
                    .title_style(styles::secondary()),
            )
            .highlight_style(styles::highlight());

        let mut state = ListState::default();
        state.select(Some(app.disk_selection));
        frame.render_stateful_widget(list, chunks[2], &mut state);
    }

    // Details box with selected disk info
    if !app.available_disks.is_empty() && app.disk_selection < app.available_disks.len() {
        let disk = &app.available_disks[app.disk_selection];
        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::primary())
            .title(" Disk Details ")
            .title_style(styles::primary());

        let details_text = Text::from(vec![
            Line::from(vec![
                Span::styled("Path: ", styles::muted()),
                Span::styled(format!("{}", disk.path.display()), styles::text()),
            ]),
            Line::from(vec![
                Span::styled("Size: ", styles::muted()),
                Span::styled(&disk.size_human, styles::info()),
                Span::styled(" | Model: ", styles::muted()),
                Span::styled(&disk.model, styles::text()),
            ]),
        ]);

        let details = Paragraph::new(details_text).block(details_block);
        frame.render_widget(details, chunks[3]);
    }

    // Hostname input
    let hostname_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.editing {
            styles::primary()
        } else {
            styles::border()
        })
        .title(" Hostname ")
        .title_style(styles::secondary());

    let hostname_text = if app.editing {
        Text::from(Line::from(vec![
            Span::styled(&app.input_buffer, styles::highlight()),
            Span::styled("▌", styles::primary()),
        ]))
    } else {
        Text::from(Line::from(Span::styled(&app.disk_hostname, styles::text())))
    };

    let hostname = Paragraph::new(hostname_text).block(hostname_block);
    frame.render_widget(hostname, chunks[4]);

    // Key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("↑/↓", styles::key_hint()),
        Span::styled(" Navigate  ", styles::muted()),
        Span::styled("h", styles::key_hint()),
        Span::styled(" Edit hostname  ", styles::muted()),
        Span::styled("Enter", styles::key_hint()),
        Span::styled(" Confirm  ", styles::muted()),
        Span::styled("Esc", styles::key_hint()),
        Span::styled(" Back", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[5]);
}
