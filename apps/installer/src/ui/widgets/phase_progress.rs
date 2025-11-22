//! Phase progress widget showing installation phases.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem},
    Frame,
};

use crate::{
    app::{Phase, Status},
    theme::styles,
};

pub fn render(frame: &mut Frame, phases: &[(Phase, Status)], current: Option<Phase>, area: Rect) {
    // Split into progress bar and phase list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress bar
            Constraint::Min(5),    // Phase list
        ])
        .split(area);

    // Calculate overall progress
    let completed = phases.iter().filter(|(_, s)| s.is_complete()).count();
    let total = phases.len();
    let progress = (completed as f64 / total as f64 * 100.0) as u16;

    // Progress bar
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(" Progress ")
                .title_style(styles::secondary()),
        )
        .gauge_style(styles::primary())
        .percent(progress)
        .label(format!("{}% ({}/{})", progress, completed, total));

    frame.render_widget(gauge, chunks[0]);

    // Phase list
    let list_items: Vec<ListItem> = phases
        .iter()
        .map(|(phase, status)| {
            let is_current = current == Some(*phase);
            let (symbol, style) = match status {
                Status::Pending => ("○", styles::muted()),
                Status::InProgress => ("◐", styles::primary()),
                Status::Success => ("✓", styles::success()),
                Status::Warning => ("⚠", styles::warning()),
                Status::Error => ("✗", styles::error()),
                Status::Skipped => ("⊘", styles::muted()),
            };

            let name_style = if is_current {
                styles::primary_bold()
            } else if status.is_complete() {
                styles::text()
            } else {
                styles::muted()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", symbol), style),
                Span::styled(format!("{:2}. ", phase.number()), styles::muted()),
                Span::styled(phase.name(), name_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border())
            .title(" Phases ")
            .title_style(styles::secondary()),
    );

    frame.render_widget(list, chunks[1]);
}
