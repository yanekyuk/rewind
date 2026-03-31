mod app;
mod ui;

use app::{App, DowngradeWizardState, PendingDownload, Screen};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use rewind_core::{config, scanner};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load_config().unwrap_or_default();
    let games_cfg = config::load_games().unwrap_or_default();
    run(cfg, games_cfg).await
}

async fn run(
    cfg: rewind_core::config::Config,
    games_cfg: rewind_core::config::GamesConfig,
) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(cfg, games_cfg);

    // Auto-detect Steam libraries on first run
    if app.config.libraries.is_empty() {
        if let Ok(lib_paths) = rewind_core::scanner::find_steam_libraries() {
            for path in lib_paths {
                if !app.config.libraries.iter().any(|l| l.path == path) {
                    app.config.libraries.push(rewind_core::config::Library { path });
                }
            }
            if !app.config.libraries.is_empty() {
                let _ = config::save_config(&app.config);
            }
        }
    }

    // Scan libraries on startup
    for lib in &app.config.libraries.clone() {
        let steamapps = lib.path.join("steamapps");
        if steamapps.exists() {
            if let Ok(games) = scanner::scan_library(&steamapps) {
                app.installed_games.extend(games);
            }
        }
    }

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // If the binary is ready, suspend the TUI and run DepotDownloader interactively.
        // This lets DepotDownloader prompt for credentials (password, Steam Guard) normally.
        if let Some(dl) = app.pending_download.take() {
            ratatui::restore();
            println!();
            let dl_result = rewind_core::depot::run_depot_downloader_interactive(
                &dl.binary,
                dl.app_id,
                dl.depot_id,
                &dl.manifest_id,
                &dl.username,
                &dl.cache_dir,
            )
            .await;
            terminal = ratatui::init();

            app.progress_rx = None;
            app.wizard_state.is_downloading = false;

            match dl_result {
                Ok(()) => finalize_downgrade_from(&mut app, dl),
                Err(e) => {
                    app.wizard_state.error = Some(format!("Download failed: {}", e));
                }
            }
            continue;
        }

        // Poll progress channel for binary-download status lines and ready signal.
        let progress_msgs: Vec<rewind_core::depot::DepotProgress> = {
            if let Some(rx) = &mut app.progress_rx {
                let mut msgs = Vec::new();
                while let Ok(msg) = rx.try_recv() {
                    msgs.push(msg);
                }
                msgs
            } else {
                Vec::new()
            }
        };
        for msg in progress_msgs {
            match msg {
                rewind_core::depot::DepotProgress::Line(line) => {
                    app.wizard_state.progress_lines.push(line);
                }
                rewind_core::depot::DepotProgress::ReadyToDownload { binary } => {
                    // Binary is ready — build PendingDownload so the loop suspends the TUI.
                    if let (Some(game), Some(username)) = (
                        app.selected_game().cloned(),
                        app.config.steam_username.clone(),
                    ) {
                        if let Ok(cache_root) = config::cache_dir() {
                            let manifest_id = app.wizard_state.manifest_input.trim().to_string();
                            let cache_dir = rewind_core::cache::manifest_cache_dir(
                                &cache_root,
                                game.app_id,
                                game.depot_id,
                                &manifest_id,
                            );
                            app.pending_download = Some(PendingDownload {
                                binary,
                                app_id: game.app_id,
                                depot_id: game.depot_id,
                                manifest_id,
                                username,
                                cache_dir,
                                game_name: game.name.clone(),
                                game_install_path: game.install_path.clone(),
                                current_manifest_id: game.manifest_id.clone(),
                                acf_path: game.acf_path.clone(),
                            });
                        }
                    }
                }
                rewind_core::depot::DepotProgress::Done => {
                    // Should not be reached (interactive path handles completion), but handle gracefully.
                    app.wizard_state.is_downloading = false;
                }
                rewind_core::depot::DepotProgress::Error(e) => {
                    app.wizard_state.is_downloading = false;
                    app.wizard_state.error = Some(e);
                }
            }
        }

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            handle_key(&mut app, key.code, key.modifiers).await;
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

async fn handle_key(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C quits from any screen.
    if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    match app.screen {
        Screen::FirstRun => handle_first_run(app, key),
        Screen::Main => handle_main(app, key),
        Screen::DowngradeWizard => handle_wizard(app, key).await,
        Screen::VersionPicker => handle_version_picker(app, key),
        Screen::Settings => handle_settings(app, key),
    }
}

fn handle_first_run(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Enter => {
            app.settings_state.username_input =
                app.config.steam_username.clone().unwrap_or_default();
            app.screen = Screen::Settings;
        }
        _ => {}
    }
}

fn handle_main(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
        KeyCode::Char('d') => {
            let has_cached = app
                .selected_game_entry()
                .map(|e| e.cached_manifest_ids.len() > 1)
                .unwrap_or(false);

            if has_cached {
                app.version_picker_state.selected_index = 0;
                app.screen = Screen::VersionPicker;
            } else if let Some(g) = app.selected_game() {
                let url = rewind_core::steamdb::depot_manifests_url(g.depot_id);
                app.wizard_state = DowngradeWizardState {
                    steamdb_url: url,
                    ..Default::default()
                };
                app.screen = Screen::DowngradeWizard;
            }
        }
        KeyCode::Char('u') => {
            // Upgrade: open version picker (user selects a different cached version)
            if app.selected_game_entry().map(|e| e.cached_manifest_ids.len() > 1).unwrap_or(false) {
                app.version_picker_state.selected_index = 0;
                app.screen = Screen::VersionPicker;
            }
        }
        KeyCode::Char('l') => {
            // Toggle ACF lock
            let app_id = app.selected_game().map(|g| g.app_id);
            if let Some(aid) = app_id {
                if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == aid) {
                    let acf = entry.acf_path();
                    if entry.acf_locked {
                        if let Err(e) = rewind_core::immutability::unlock_file(&acf) {
                            eprintln!("Warning: failed to unlock ACF: {}", e);
                        }
                        entry.acf_locked = false;
                    } else {
                        if let Err(e) = rewind_core::immutability::lock_file(&acf) {
                            eprintln!("Warning: failed to lock ACF: {}", e);
                        }
                        entry.acf_locked = true;
                    }
                    let _ = config::save_games(&app.games_config);
                }
            }
        }
        KeyCode::Char('s') => {
            app.settings_state.username_input =
                app.config.steam_username.clone().unwrap_or_default();
            app.settings_state.library_input.clear();
            app.settings_state.focused_field = 0;
            app.screen = Screen::Settings;
        }
        KeyCode::Char('o') => {
            if let Some(game) = app.selected_game() {
                let url = rewind_core::steamdb::app_url(game.app_id);
                let _ = open::that(url);
            }
        }
        _ => {}
    }
}

async fn handle_wizard(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.screen = Screen::Main;
            app.wizard_state = DowngradeWizardState::default();
        }
        KeyCode::Char('o') => {
            let url = app.wizard_state.steamdb_url.clone();
            let _ = open::that(url);
        }
        KeyCode::Backspace => {
            if !app.wizard_state.is_downloading {
                app.wizard_state.manifest_input.pop();
            }
        }
        KeyCode::Char(c) => {
            if !app.wizard_state.is_downloading {
                app.wizard_state.manifest_input.push(c);
            }
        }
        KeyCode::Enter if !app.wizard_state.is_downloading => {
            if !app.wizard_state.manifest_input.trim().is_empty() {
                start_download(app).await;
            }
        }
        _ => {}
    }
}

fn handle_version_picker(app: &mut App, key: KeyCode) {
    let cached_len = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.len())
        .unwrap_or(0);

    match key {
        KeyCode::Esc => app.screen = Screen::Main,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.version_picker_state.selected_index > 0 {
                app.version_picker_state.selected_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.version_picker_state.selected_index + 1 < cached_len {
                app.version_picker_state.selected_index += 1;
            }
        }
        KeyCode::Enter => {
            let target_manifest = app
                .selected_game_entry()
                .and_then(|e| e.cached_manifest_ids.get(app.version_picker_state.selected_index))
                .cloned();

            if let Some(manifest_id) = target_manifest {
                switch_to_cached_version(app, manifest_id);
            }
            app.screen = Screen::Main;
        }
        _ => {}
    }
}

fn handle_settings(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.config.steam_username =
                Some(app.settings_state.username_input.clone()).filter(|s| !s.is_empty());
            let _ = config::save_config(&app.config);
            app.screen = Screen::Main;
        }
        KeyCode::Backspace => match app.settings_state.focused_field {
            0 => {
                app.settings_state.username_input.pop();
            }
            _ => {
                app.settings_state.library_input.pop();
            }
        },
        KeyCode::Char(c) => match app.settings_state.focused_field {
            0 => app.settings_state.username_input.push(c),
            _ => app.settings_state.library_input.push(c),
        },
        KeyCode::Tab => {
            app.settings_state.focused_field = (app.settings_state.focused_field + 1) % 2;
        }
        KeyCode::Enter => match app.settings_state.focused_field {
            0 => {
                app.config.steam_username =
                    Some(app.settings_state.username_input.clone()).filter(|s| !s.is_empty());
                let _ = config::save_config(&app.config);
            }
            _ => {
                let path =
                    std::path::PathBuf::from(app.settings_state.library_input.trim());
                if path.exists()
                    && !app.config.libraries.iter().any(|l| l.path == path)
                {
                    app.config
                        .libraries
                        .push(rewind_core::config::Library { path });
                    let _ = config::save_config(&app.config);
                    app.settings_state.library_input.clear();
                    // Rescan
                    app.installed_games.clear();
                    for lib in &app.config.libraries.clone() {
                        let steamapps = lib.path.join("steamapps");
                        if steamapps.exists() {
                            if let Ok(games) = scanner::scan_library(&steamapps) {
                                app.installed_games.extend(games);
                            }
                        }
                    }
                }
            }
        },
        _ => {}
    }
}

async fn start_download(app: &mut App) {
    if app.config.steam_username.is_none() {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    let Ok(bin_dir) = config::bin_dir() else { return };

    // Check .NET runtime is available
    if !rewind_core::depot::check_dotnet().await {
        app.wizard_state.error = Some(
            "Error: .NET runtime not found.\nPlease install from https://dotnet.microsoft.com/download".into(),
        );
        return;
    }

    let (tx, rx) = mpsc::channel(10);
    app.progress_rx = Some(rx);
    app.wizard_state.is_downloading = true;
    app.wizard_state.progress_lines.clear();
    app.wizard_state.error = None;

    // Background task: locate/download the DepotDownloader binary.
    // Once ready, signal the main loop via ReadyToDownload so it can suspend the TUI
    // and run the download interactively (required for password / Steam Guard prompts).
    tokio::spawn(async move {
        let _ = tx
            .send(rewind_core::depot::DepotProgress::Line(
                "Locating DepotDownloader...".into(),
            ))
            .await;
        match rewind_core::depot::ensure_depot_downloader(&bin_dir).await {
            Ok(binary) => {
                let _ = tx
                    .send(rewind_core::depot::DepotProgress::Line(
                        "DepotDownloader ready. Starting download...".into(),
                    ))
                    .await;
                let _ = tx
                    .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                    .await;
            }
        }
    });
}

fn switch_to_cached_version(app: &mut App, manifest_id: String) {
    let Some(game) = app.selected_game().cloned() else {
        return;
    };
    let Ok(cache_root) = config::cache_dir() else { return };

    let new_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &manifest_id,
    );

    if let Err(e) = rewind_core::cache::repoint_symlinks(&game.install_path, &new_cache) {
        eprintln!("Failed to switch version: {}", e);
        return;
    }

    if let Some(entry) = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == game.app_id)
    {
        entry.active_manifest_id = manifest_id.clone();
        // buildid "0" is a safe placeholder: ACF is locked so Steam can't update.
        if let Err(e) = rewind_core::patcher::patch_acf_file(
            &entry.acf_path(),
            "0",
            &manifest_id,
            entry.depot_id,
        ) {
            eprintln!("Warning: failed to patch ACF: {}", e);
        }
        if let Err(e) = rewind_core::immutability::lock_file(&entry.acf_path()) {
            eprintln!("Warning: failed to lock ACF: {}", e);
        }
        entry.acf_locked = true;
    }

    let _ = config::save_games(&app.games_config);
}

fn finalize_downgrade_from(app: &mut App, dl: PendingDownload) {
    let Ok(cache_root) = config::cache_dir() else { return };

    let target_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.manifest_id,
    );
    let current_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.current_manifest_id,
    );

    if let Err(e) =
        rewind_core::cache::apply_downloaded(&dl.game_install_path, &target_cache, &current_cache)
    {
        app.wizard_state.error = Some(format!("Failed to apply files: {}", e));
        return;
    }

    let existing = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == dl.app_id);

    if let Some(entry) = existing {
        entry.active_manifest_id = dl.manifest_id.clone();
        if !entry.cached_manifest_ids.contains(&dl.manifest_id) {
            entry.cached_manifest_ids.push(dl.manifest_id.clone());
        }
        if !entry.cached_manifest_ids.contains(&dl.current_manifest_id) {
            entry.cached_manifest_ids.push(dl.current_manifest_id.clone());
        }
        entry.acf_locked = true;
    } else {
        app.games_config.games.push(rewind_core::config::GameEntry {
            name: dl.game_name.clone(),
            app_id: dl.app_id,
            depot_id: dl.depot_id,
            install_path: dl.game_install_path.clone(),
            active_manifest_id: dl.manifest_id.clone(),
            latest_manifest_id: dl.current_manifest_id.clone(),
            cached_manifest_ids: vec![dl.current_manifest_id.clone(), dl.manifest_id.clone()],
            acf_locked: true,
        });
    }

    if let Err(e) =
        rewind_core::patcher::patch_acf_file(&dl.acf_path, "0", &dl.manifest_id, dl.depot_id)
    {
        eprintln!("Warning: failed to patch ACF: {}", e);
    }
    if let Err(e) = rewind_core::immutability::lock_file(&dl.acf_path) {
        eprintln!("Warning: failed to lock ACF: {}", e);
    }
    let _ = config::save_games(&app.games_config);

    app.screen = Screen::Main;
}
