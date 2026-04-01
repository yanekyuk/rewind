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

    repair_stale_locks(&mut app);

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
                    if e.contains(".NET runtime not found") {
                        app.wizard_state.error_url =
                            Some("https://dotnet.microsoft.com/download".into());
                    }
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

/// For every managed game with `acf_locked = true`, check whether Steam corrupted the
/// StateFlags (e.g. started an update before the lock was in place).  If the ACF exists but
/// reports StateFlags != 4, unlock it, patch it back to a fully-installed state, and re-lock.
/// After any repair, rescan so `installed_games` reflects the corrected ACF.
fn repair_stale_locks(app: &mut App) {
    let candidates: Vec<_> = app
        .games_config
        .games
        .iter()
        .filter(|e| e.acf_locked)
        .map(|e| (e.app_id, e.depot_id, e.acf_path(), e.latest_manifest_id.clone(), e.latest_buildid.clone()))
        .collect();

    let mut repaired = false;
    for (app_id, depot_id, acf_path, latest_manifest_id, latest_buildid) in candidates {
        if !acf_path.exists() {
            continue;
        }
        let state_flags = match rewind_core::scanner::read_acf_state_flags(&acf_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Warning: could not read StateFlags for app {app_id}: {e}");
                continue;
            }
        };
        if state_flags == 4 {
            continue; // healthy
        }
        // Steam left the ACF in an update/broken state.  Restore it by writing the
        // latest buildid/manifest so Steam thinks the game is already up to date.
        eprintln!(
            "Info: repairing ACF for app {app_id} (StateFlags={state_flags}, expected 4)"
        );
        let _ = rewind_core::immutability::unlock_file(&acf_path);
        if let Err(e) =
            rewind_core::patcher::patch_acf_file(&acf_path, &latest_buildid, &latest_manifest_id, depot_id)
        {
            eprintln!("Warning: failed to repair ACF for app {app_id}: {e}");
            continue;
        }
        if let Err(e) = rewind_core::immutability::lock_file(&acf_path) {
            eprintln!("Warning: failed to re-lock ACF for app {app_id}: {e}");
        }
        repaired = true;
    }

    if repaired {
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

async fn handle_key(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C quits from any screen.
    if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    match app.screen {
        Screen::FirstRun => handle_first_run(app, key),
        Screen::Main => handle_main(app, key),
        Screen::DowngradeWizard => handle_wizard(app, key),
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

fn handle_wizard(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.screen = Screen::Main;
            app.wizard_state = DowngradeWizardState::default();
        }
        KeyCode::Char('o') => {
            let url = if let Some(ref err_url) = app.wizard_state.error_url {
                err_url.clone()
            } else {
                app.wizard_state.steamdb_url.clone()
            };
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
                start_download(app);
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

fn start_download(app: &mut App) {
    if app.config.steam_username.is_none() {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    let Ok(bin_dir) = config::bin_dir() else { return };

    let (tx, rx) = mpsc::channel(10);
    app.progress_rx = Some(rx);
    app.wizard_state.is_downloading = true;
    app.wizard_state.progress_lines.clear();
    app.wizard_state.error = None;
    app.wizard_state.error_url = None;

    // All async work (dotnet check + binary download) runs in a background task so the
    // main event loop is never blocked and the TUI stays responsive.
    tokio::spawn(async move {
        let _ = tx
            .send(rewind_core::depot::DepotProgress::Line(
                "Checking .NET runtime...".into(),
            ))
            .await;
        if !rewind_core::depot::check_dotnet().await {
            let _ = tx
                .send(rewind_core::depot::DepotProgress::Error(
                    ".NET runtime not found. Press [O] to open the download page.".into(),
                ))
                .await;
            return;
        }

        let _ = tx
            .send(rewind_core::depot::DepotProgress::Line(
                "Locating DepotDownloader...".into(),
            ))
            .await;
        match rewind_core::depot::ensure_depot_downloader(&bin_dir).await {
            Ok(binary) => {
                let _ = tx
                    .send(rewind_core::depot::DepotProgress::Line(
                        "Ready. Starting download...".into(),
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
        // Patch the ACF with the latest buildid/manifest to trick Steam into thinking
        // the game is already on the newest version, suppressing update prompts.
        let latest_buildid = entry.latest_buildid.clone();
        let latest_manifest = entry.latest_manifest_id.clone();
        let depot_id = entry.depot_id;
        let acf_path = entry.acf_path();
        if let Err(e) = rewind_core::patcher::patch_acf_file(
            &acf_path,
            &latest_buildid,
            &latest_manifest,
            depot_id,
        ) {
            eprintln!("Warning: failed to patch ACF: {}", e);
        }
        if let Err(e) = rewind_core::immutability::lock_file(&acf_path) {
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

    // Read the latest buildid from the ACF now, before we overwrite it.
    // At this point the ACF still has the pre-downgrade (i.e. latest) values.
    let latest_buildid = rewind_core::scanner::read_acf_buildid(&dl.acf_path)
        .unwrap_or_else(|_| "0".to_string());

    if let Some(entry) = existing {
        entry.active_manifest_id = dl.manifest_id.clone();
        if !entry.cached_manifest_ids.contains(&dl.manifest_id) {
            entry.cached_manifest_ids.push(dl.manifest_id.clone());
        }
        if !entry.cached_manifest_ids.contains(&dl.current_manifest_id) {
            entry.cached_manifest_ids.push(dl.current_manifest_id.clone());
        }
        entry.latest_buildid = latest_buildid.clone();
        entry.acf_locked = true;
    } else {
        app.games_config.games.push(rewind_core::config::GameEntry {
            name: dl.game_name.clone(),
            app_id: dl.app_id,
            depot_id: dl.depot_id,
            install_path: dl.game_install_path.clone(),
            active_manifest_id: dl.manifest_id.clone(),
            latest_manifest_id: dl.current_manifest_id.clone(),
            latest_buildid: latest_buildid.clone(),
            cached_manifest_ids: vec![dl.current_manifest_id.clone(), dl.manifest_id.clone()],
            acf_locked: true,
        });
    }

    // Patch the ACF with the *latest* buildid and manifest so Steam believes the game
    // is already on the newest version and won't queue an update.
    if let Err(e) = rewind_core::patcher::patch_acf_file(
        &dl.acf_path,
        &latest_buildid,
        &dl.current_manifest_id,
        dl.depot_id,
    ) {
        eprintln!("Warning: failed to patch ACF: {}", e);
    }
    if let Err(e) = rewind_core::immutability::lock_file(&dl.acf_path) {
        eprintln!("Warning: failed to lock ACF: {}", e);
    }
    let _ = config::save_games(&app.games_config);

    app.screen = Screen::Main;
}
