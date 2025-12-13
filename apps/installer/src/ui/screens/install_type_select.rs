//! Install type selection screen (ISO mode only).
//! Allows choosing between Live Install and Full Disk Install.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::{App, InstallType},
    theme::{styles, symbols},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Select Installation Type ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Length(1), // Spacing
            Constraint::Min(8),    // Options list
            Constraint::Length(6), // Description box
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Header
    let header = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "NQRust-MicroVM Air-Gapped Installation",
            styles::header(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Choose how you want to install:",
            styles::text(),
        )),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Build list items
    let items: Vec<ListItem> = InstallType::ALL
        .iter()
        .enumerate()
        .map(|(i, install_type)| {
            let selected = i == app.install_type_selection;
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

            let icon = match install_type {
                InstallType::LiveInstall => "󰌽",
                InstallType::DiskInstall => "",
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{} ", icon), style),
                Span::styled(install_type.name(), style),
            ]);

            ListItem::new(line)
        })
        .collect();

    // Options list
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Options ")
                .title_style(styles::secondary()),
        )
        .highlight_style(styles::highlight());

    let mut state = ListState::default();
    state.select(Some(app.install_type_selection));
    frame.render_stateful_widget(list, chunks[2], &mut state);

    // Description box
    let selected_type = InstallType::ALL[app.install_type_selection];
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::primary())
        .title(format!(" {} ", selected_type.name()))
        .title_style(styles::primary());

    let desc_text = Text::from(vec![
        Line::from(Span::styled(selected_type.description(), styles::text())),
        Line::from(""),
        Line::from(Span::styled(selected_type.details(), styles::warning())),
    ]);

    let description = Paragraph::new(desc_text).block(desc_block);
    frame.render_widget(description, chunks[3]);

    // Key hints
    let hints = Text::from(vec![Line::from(vec![
        Span::styled("↑/↓", styles::key_hint()),
        Span::styled(" Navigate  ", styles::muted()),
        Span::styled("Enter", styles::key_hint()),
        Span::styled(" Select  ", styles::muted()),
        Span::styled("Esc", styles::key_hint()),
        Span::styled(" Back  ", styles::muted()),
        Span::styled("q", styles::key_hint()),
        Span::styled(" Quit", styles::muted()),
    ])]);
    let hints_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hints_para, chunks[4]);
}
