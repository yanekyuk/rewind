pub mod downgrade_wizard;
pub mod first_run;
pub mod main_screen;
pub mod settings;
pub mod switch_overlay;
pub mod theme;
pub mod version_picker;

use crate::app::{App, Screen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

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

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::FirstRun => first_run::draw(f, app),
        Screen::Main => main_screen::draw(f, app),
        Screen::DowngradeWizard => {
            main_screen::draw(f, app);
            downgrade_wizard::draw(f, app);
        }
        Screen::VersionPicker => {
            main_screen::draw(f, app);
            version_picker::draw(f, app);
        }
        Screen::Settings => settings::draw(f, app),
        Screen::SwitchOverlay => {
            main_screen::draw(f, app);
            switch_overlay::draw(f, app);
        }
        Screen::ReshadeSetup => {},
    }
}
