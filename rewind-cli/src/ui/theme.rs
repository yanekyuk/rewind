use ratatui::style::{Color, Modifier, Style};

// Steam color palette
pub const BASE_BG: Color = Color::Rgb(27, 40, 56);       // #1b2838
#[allow(dead_code)]
pub const PANEL_BG: Color = Color::Rgb(42, 71, 94);      // #2a475e
pub const ACCENT: Color = Color::Rgb(102, 192, 244);      // #66c0f4
pub const TEXT_PRIMARY: Color = Color::Rgb(199, 213, 224); // #c7d5e0
pub const TEXT_SECONDARY: Color = Color::Rgb(143, 152, 160); // #8f98a0
pub const SUCCESS: Color = Color::Rgb(91, 163, 43);       // #5ba32b
pub const WARNING: Color = Color::Rgb(229, 160, 13);      // #e5a00d
pub const ERROR: Color = Color::Rgb(195, 60, 60);         // #c33c3c
pub const SELECTED_BG: Color = Color::Rgb(61, 108, 142);  // #3d6c8e

// Pre-built styles
pub fn title() -> Style {
    Style::default()
        .fg(ACCENT)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn text() -> Style {
    Style::default().fg(TEXT_PRIMARY).bg(BASE_BG)
}

pub fn text_secondary() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn border() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn border_accent() -> Style {
    Style::default().fg(ACCENT).bg(BASE_BG)
}

pub fn border_focused() -> Style {
    Style::default()
        .fg(WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn selected() -> Style {
    Style::default().fg(TEXT_PRIMARY).bg(SELECTED_BG)
}

pub fn status_success() -> Style {
    Style::default().fg(SUCCESS).bg(BASE_BG)
}

pub fn status_warning() -> Style {
    Style::default()
        .fg(WARNING)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn status_error() -> Style {
    Style::default().fg(ERROR).bg(BASE_BG)
}

pub fn input_active() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn input_inactive() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn help_bar() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

/// Background fill style for areas that should show the base background.
pub fn base_bg() -> Style {
    Style::default().bg(BASE_BG)
}

/// Background fill style for panel areas.
#[allow(dead_code)]
pub fn panel_bg() -> Style {
    Style::default().bg(PANEL_BG)
}
