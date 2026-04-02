use ratatui::style::{Color, Modifier, Style};

// Steam color palette (foreground-only — terminal default background shows through)
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
        .add_modifier(Modifier::BOLD)
}

pub fn text() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub fn text_secondary() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn border() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn border_accent() -> Style {
    Style::default().fg(ACCENT)
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
    Style::default().fg(SUCCESS)
}

pub fn status_warning() -> Style {
    Style::default()
        .fg(WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn status_error() -> Style {
    Style::default().fg(ERROR)
}

pub fn input_active() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn input_inactive() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn help_bar() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}
