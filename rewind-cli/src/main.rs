mod app;
mod ui;

use app::{App, DowngradeWizardState, Screen};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use rewind_core::{config, scanner};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load_config().unwrap_or_default();
    let games_cfg = config::load_games().unwrap_or_default();

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, cfg, games_cfg).await;
    ratatui::restore();

    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    cfg: rewind_core::config::Config,
    games_cfg: rewind_core::config::GamesConfig,
) -> anyhow::Result<()> {
    let mut app = App::new(cfg, games_cfg);

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

        // Poll progress channel — collect first, then process to avoid borrow conflicts
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
                rewind_core::depot::DepotProgress::Done => {
                    app.wizard_state.is_downloading = false;
                    finalize_downgrade(&mut app);
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

    Ok(())
}

async fn handle_key(app: &mut App, key: KeyCode, _modifiers: KeyModifiers) {
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
                        let _ = rewind_core::immutability::unlock_file(&acf);
                        entry.acf_locked = false;
                    } else {
                        let _ = rewind_core::immutability::lock_file(&acf);
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
            let manifest_id = app.wizard_state.manifest_input.trim().to_string();
            if !manifest_id.is_empty() {
                start_download(app, manifest_id).await;
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

async fn start_download(app: &mut App, manifest_id: String) {
    let Some(game) = app.selected_game().cloned() else {
        return;
    };
    let Some(username) = app.config.steam_username.clone() else {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    let Ok(bin_dir) = config::bin_dir() else { return };
    let Ok(cache_root) = config::cache_dir() else { return };

    let (tx, rx) = mpsc::channel(100);
    app.progress_rx = Some(rx);
    app.wizard_state.is_downloading = true;
    app.wizard_state.progress_lines.clear();
    app.wizard_state.error = None;

    let cache_dir = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &manifest_id,
    );

    tokio::spawn(async move {
        let binary = match rewind_core::depot::ensure_depot_downloader(&bin_dir).await {
            Ok(b) => b,
            Err(e) => {
                let _ = tx
                    .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                    .await;
                return;
            }
        };

        let _ = rewind_core::depot::run_depot_downloader(
            &binary,
            game.app_id,
            game.depot_id,
            &manifest_id,
            &username,
            &cache_dir,
            tx,
        )
        .await;
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
        let _ = rewind_core::patcher::patch_acf_file(
            &entry.acf_path(),
            "0",
            &manifest_id,
            entry.depot_id,
        );
        let _ = rewind_core::immutability::lock_file(&entry.acf_path());
        entry.acf_locked = true;
    }

    let _ = config::save_games(&app.games_config);
}

fn finalize_downgrade(app: &mut App) {
    let Some(game) = app.selected_game().cloned() else {
        return;
    };
    let Ok(cache_root) = config::cache_dir() else { return };

    let manifest_id = app.wizard_state.manifest_input.trim().to_string();
    if manifest_id.is_empty() {
        return;
    }

    let target_cache =
        rewind_core::cache::manifest_cache_dir(&cache_root, game.app_id, game.depot_id, &manifest_id);
    let current_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &game.manifest_id,
    );

    if let Err(e) =
        rewind_core::cache::apply_downloaded(&game.install_path, &target_cache, &current_cache)
    {
        app.wizard_state.error = Some(format!("Failed to apply files: {}", e));
        return;
    }

    let acf_path = game
        .install_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(format!("appmanifest_{}.acf", game.app_id)))
        .unwrap_or_else(|| game.install_path.join(format!("appmanifest_{}.acf", game.app_id)));

    let existing = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == game.app_id);

    if let Some(entry) = existing {
        entry.active_manifest_id = manifest_id.clone();
        if !entry.cached_manifest_ids.contains(&manifest_id) {
            entry.cached_manifest_ids.push(manifest_id.clone());
        }
        if !entry.cached_manifest_ids.contains(&game.manifest_id) {
            entry.cached_manifest_ids.push(game.manifest_id.clone());
        }
        entry.acf_locked = true;
    } else {
        app.games_config.games.push(rewind_core::config::GameEntry {
            name: game.name.clone(),
            app_id: game.app_id,
            depot_id: game.depot_id,
            install_path: game.install_path.clone(),
            active_manifest_id: manifest_id.clone(),
            latest_manifest_id: game.manifest_id.clone(),
            cached_manifest_ids: vec![game.manifest_id.clone(), manifest_id.clone()],
            acf_locked: true,
        });
    }

    let _ = rewind_core::patcher::patch_acf_file(&acf_path, "0", &manifest_id, game.depot_id);
    let _ = rewind_core::immutability::lock_file(&acf_path);
    let _ = config::save_games(&app.games_config);

    app.screen = Screen::Main;
}
