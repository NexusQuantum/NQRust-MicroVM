//! Checklist widget for displaying check items.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::{
    app::{CheckItem, Status},
    theme::styles,
};

pub fn render(frame: &mut Frame, items: &[CheckItem], area: Rect) {
    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| {
            let (symbol, style) = match item.status {
                Status::Pending => ("○", styles::muted()),
                Status::InProgress => ("◐", styles::primary()),
                Status::Success => ("✓", styles::success()),
                Status::Warning => ("⚠", styles::warning()),
                Status::Error => ("✗", styles::error()),
                Status::Skipped => ("⊘", styles::muted()),
            };

            let mut spans = vec![
                Span::styled(format!(" {} ", symbol), style),
                Span::styled(&item.name, styles::text()),
            ];

            if let Some(ref msg) = item.message {
                spans.push(Span::styled(format!(" - {}", msg), styles::muted()));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border())
            .title(" Checks ")
            .title_style(styles::secondary()),
    );

    frame.render_widget(list, area);
}
