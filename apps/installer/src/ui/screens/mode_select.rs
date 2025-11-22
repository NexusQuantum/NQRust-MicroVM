//! Mode selection screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::{App, InstallMode},
    theme::{styles, symbols},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .title(" Select Installation Mode ")
        .title_alignment(Alignment::Center)
        .title_style(styles::title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(1), // Spacing
            Constraint::Min(10),   // Mode list
            Constraint::Length(5), // Description box
            Constraint::Length(3), // Key hints
        ])
        .split(inner);

    // Header
    let header = Paragraph::new(Text::from(vec![Line::from(Span::styled(
        "Choose the installation mode that best fits your needs:",
        styles::text(),
    ))]))
    .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Build list items
    let items: Vec<ListItem> = InstallMode::ALL
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let selected = i == app.mode_selection;
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

            // Build components string
            let components = build_components_string(*mode);

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(mode.name(), style),
                Span::styled(
                    format!("  ({})", components),
                    if selected { style } else { styles::muted() },
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    // Mode list
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Modes ")
                .title_style(styles::secondary()),
        )
        .highlight_style(styles::highlight());

    let mut state = ListState::default();
    state.select(Some(app.mode_selection));
    frame.render_stateful_widget(list, chunks[2], &mut state);

    // Description box
    let selected_mode = InstallMode::ALL[app.mode_selection];
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::primary())
        .title(format!(" {} ", selected_mode.name()))
        .title_style(styles::primary());

    let desc_text = Text::from(vec![
        Line::from(Span::styled(selected_mode.description(), styles::text())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Components: ", styles::muted()),
            Span::styled(build_components_list(selected_mode), styles::info()),
        ]),
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

fn build_components_string(mode: InstallMode) -> String {
    let mut components = Vec::new();
    if mode.includes_manager() {
        components.push("Manager");
    }
    if mode.includes_agent() {
        components.push("Agent");
    }
    if mode.includes_ui() {
        components.push("UI");
    }
    components.join(" + ")
}

fn build_components_list(mode: InstallMode) -> String {
    let mut components = Vec::new();
    if mode.includes_manager() {
        components.push("Manager (orchestration)");
    }
    if mode.includes_agent() {
        components.push("Agent (VM host)");
    }
    if mode.includes_ui() {
        components.push("Web UI");
    }
    components.join(", ")
}
