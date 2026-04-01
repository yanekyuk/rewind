use rewind_core::{
    config::{Config, GameEntry, GamesConfig},
    depot::DepotProgress,
    scanner::InstalledGame,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    CheckDotnet,
    DownloadDepot,
    DownloadManifest,
    BackupFiles,
    LinkFiles,
    PatchManifest,
    LockManifest,
}

impl StepKind {
    pub fn label(&self) -> &'static str {
        match self {
            StepKind::CheckDotnet => "Checking .NET runtime",
            StepKind::DownloadDepot => "Downloading DepotDownloader",
            StepKind::DownloadManifest => "Downloading manifest files",
            StepKind::BackupFiles => "Backing up current files",
            StepKind::LinkFiles => "Linking manifest files to game directory",
            StepKind::PatchManifest => "Patching Steam manifest",
            StepKind::LockManifest => "Locking manifest file",
        }
    }
}

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
    pub is_downloading: bool,
    pub error: Option<String>,
    /// When set, pressing [O] opens this URL instead of the SteamDB manifests page.
    pub error_url: Option<String>,
    /// High-level process steps with their status.
    pub steps: Vec<(StepKind, StepStatus)>,
    /// Raw output lines from DepotDownloader (shown in the bordered pane).
    pub depot_lines: Vec<String>,
    /// When Some, a credential prompt is active and this holds the user's input so far.
    pub prompt_input: Option<String>,
    /// The prompt label from DepotDownloader (e.g. "Password:").
    pub prompt_label: Option<String>,
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

#[derive(Debug, Default)]
pub struct ImageState {
    /// Loaded hero images keyed by app_id.
    pub loaded_images: HashMap<u32, image::DynamicImage>,
    /// App IDs currently being fetched (to avoid duplicate requests).
    pub pending_fetches: HashSet<u32>,
}

/// Download parameters for the active DepotDownloader session.
pub struct PendingDownload {
    pub app_id: u32,
    pub depot_id: u32,
    pub manifest_id: String,
    pub username: String,
    pub cache_dir: PathBuf,
    pub game_name: String,
    pub game_install_path: PathBuf,
    pub current_manifest_id: String,
    pub acf_path: PathBuf,
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
    /// Active download parameters (set when download starts, consumed on completion).
    pub pending_download: Option<PendingDownload>,
    /// Stdin handle for the running DepotDownloader process (used to forward credential input).
    pub depot_stdin: Option<tokio::process::ChildStdin>,
    /// Sender to kill the DepotDownloader child process.
    pub depot_kill: Option<mpsc::Sender<()>>,
    pub image_state: ImageState,
    pub image_picker: Option<ratatui_image::picker::Picker>,
    /// Receiver to get the stdin handle back after writing credentials.
    pub pending_stdin_return: Option<mpsc::Receiver<tokio::process::ChildStdin>>,
    /// Tracks when the last DepotDownloader output was received (for timeout detection).
    pub last_depot_output: Option<std::time::Instant>,
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
            pending_download: None,
            depot_stdin: None,
            depot_kill: None,
            image_state: ImageState::default(),
            image_picker: None,
            pending_stdin_return: None,
            last_depot_output: None,
            should_quit: false,
        }
    }

    pub fn set_step_status(&mut self, kind: &StepKind, status: StepStatus) {
        if let Some(step) = self.wizard_state.steps.iter_mut().find(|s| s.0 == *kind) {
            step.1 = status;
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
