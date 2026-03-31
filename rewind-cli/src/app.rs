use rewind_core::{
    config::{Config, GameEntry, GamesConfig},
    depot::DepotProgress,
    scanner::InstalledGame,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    FirstRun,
    Main,
    DowngradeWizard,
    VersionPicker,
    Settings,
}

#[derive(Debug, Default)]
pub struct DowngradeWizardState {
    pub manifest_input: String,
    pub steamdb_url: String,
    pub progress_lines: Vec<String>,
    pub is_downloading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct SettingsState {
    pub username_input: String,
    pub library_input: String,
    pub focused_field: usize,
}

#[derive(Debug, Default)]
pub struct VersionPickerState {
    pub selected_index: usize,
}

pub struct App {
    pub screen: Screen,
    pub config: Config,
    pub games_config: GamesConfig,
    pub installed_games: Vec<InstalledGame>,
    pub selected_game_index: usize,
    pub wizard_state: DowngradeWizardState,
    pub settings_state: SettingsState,
    pub version_picker_state: VersionPickerState,
    pub progress_rx: Option<mpsc::Receiver<DepotProgress>>,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, games_config: GamesConfig) -> Self {
        let first_run = config.steam_username.is_none() && config.libraries.is_empty();
        App {
            screen: if first_run { Screen::FirstRun } else { Screen::Main },
            config,
            games_config,
            installed_games: Vec::new(),
            selected_game_index: 0,
            wizard_state: DowngradeWizardState::default(),
            settings_state: SettingsState::default(),
            version_picker_state: VersionPickerState::default(),
            progress_rx: None,
            should_quit: false,
        }
    }

    pub fn selected_game(&self) -> Option<&InstalledGame> {
        self.installed_games.get(self.selected_game_index)
    }

    pub fn selected_game_entry(&self) -> Option<&GameEntry> {
        self.selected_game().and_then(|g| {
            self.games_config
                .games
                .iter()
                .find(|e| e.app_id == g.app_id)
        })
    }

    pub fn scroll_up(&mut self) {
        if self.selected_game_index > 0 {
            self.selected_game_index -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.selected_game_index + 1 < self.installed_games.len() {
            self.selected_game_index += 1;
        }
    }
}
