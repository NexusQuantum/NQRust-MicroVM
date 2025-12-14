//! UI module for the NQR-MicroVM installer.
//!
//! Contains all screen renderers and custom widgets.

#![allow(dead_code)]

pub mod screens;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::{App, Screen};

/// Main UI renderer
pub fn render(frame: &mut Frame, app: &App) {
    // Create main layout with optional status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    // Render current screen
    match app.screen {
        Screen::Welcome => screens::welcome::render(frame, app, chunks[0]),
        Screen::InstallTypeSelect => screens::install_type_select::render(frame, app, chunks[0]),
        Screen::DiskSelect => screens::disk_select::render(frame, app, chunks[0]),
        Screen::DiskConfig => screens::disk_config::render(frame, app, chunks[0]),
        Screen::ModeSelect => screens::mode_select::render(frame, app, chunks[0]),
        Screen::Config => screens::config::render(frame, app, chunks[0]),
        Screen::Preflight => screens::preflight::render(frame, app, chunks[0]),
        Screen::Progress => screens::progress::render(frame, app, chunks[0]),
        Screen::DiskProgress => screens::disk_progress::render(frame, app, chunks[0]),
        Screen::Verify => screens::verify::render(frame, app, chunks[0]),
        Screen::Complete => screens::complete::render(frame, app, chunks[0]),
        Screen::Error => screens::error::render(frame, app, chunks[0]),
    }

    // Render status bar
    widgets::status_bar::render(frame, app, chunks[1]);
}

/// Helper to create centered rect
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Helper to create centered rect with fixed size
pub fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
