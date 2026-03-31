pub mod downgrade_wizard;
pub mod first_run;
pub mod main_screen;
pub mod settings;
pub mod version_picker;

use crate::app::{App, Screen};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
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
    }
}
