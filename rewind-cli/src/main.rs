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

    // Detect terminal image protocol support
    app.image_picker = ratatui_image::picker::Picker::from_query_stdio().ok();

    // Image loading channel
    let (image_tx, mut image_rx) = mpsc::channel::<(u32, Option<image::DynamicImage>)>(16);

    loop {
        // Poll ReShade setup progress channel.
        let reshade_msgs: Vec<rewind_core::reshade::ReshadeProgress> = {
            if let Some(rx) = &mut app.reshade_progress_rx {
                let mut msgs = Vec::new();
                while let Ok(msg) = rx.try_recv() {
                    msgs.push(msg);
                }
                msgs
            } else {
                Vec::new()
            }
        };
        for msg in reshade_msgs {
            match msg {
                rewind_core::reshade::ReshadeProgress::Line(line) => {
                    app.reshade_state.lines.push(line);
                }
                rewind_core::reshade::ReshadeProgress::Done => {
                    finalize_reshade(&mut app);
                }
                rewind_core::reshade::ReshadeProgress::Error(e) => {
                    app.reshade_state.error = Some(e);
                    app.reshade_state.done = false;
                }
            }
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Poll progress channel.
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
            use crate::app::{StepKind, StepStatus};
            match msg {
                rewind_core::depot::DepotProgress::Line(line) => {
                    if let Some(step_name) = line.strip_prefix("__STEP_DONE:") {
                        if let Some(kind) = step_kind_from_str(step_name) {
                            app.set_step_status(&kind, StepStatus::Done);
                        }
                    } else if let Some(step_name) = line.strip_prefix("__STEP_START:") {
                        if let Some(kind) = step_kind_from_str(step_name) {
                            app.set_step_status(&kind, StepStatus::InProgress);
                        }
                    } else {
                        app.wizard_state.depot_lines.push(line);
                        app.last_depot_output = Some(std::time::Instant::now());
                    }
                }
                rewind_core::depot::DepotProgress::ReadyToDownload { binary } => {
                    if let Some(ref dl) = app.pending_download {
                        let (tx_d, rx_d) = mpsc::channel(64);
                        app.progress_rx = Some(rx_d);

                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            tx_d,
                        )
                        .await
                        {
                            Ok((stdin, kill_tx)) => {
                                app.depot_stdin = Some(stdin);
                                app.depot_kill = Some(kill_tx);
                            }
                            Err(e) => {
                                app.set_step_status(
                                    &StepKind::DownloadManifest,
                                    StepStatus::Failed(e.to_string()),
                                );
                                app.wizard_state.error =
                                    Some(format!("Failed to start download: {}", e));
                                app.wizard_state.is_downloading = false;
                            }
                        }
                    }
                }
                rewind_core::depot::DepotProgress::Prompt(label) => {
                    app.wizard_state.prompt_label = Some(label);
                    app.wizard_state.prompt_input = Some(String::new());
                    app.last_depot_output = Some(std::time::Instant::now());
                }
                rewind_core::depot::DepotProgress::Done => {
                    app.set_step_status(&StepKind::DownloadManifest, StepStatus::Done);
                    app.depot_stdin = None;
                    app.depot_kill = None;
                    if let Some(dl) = app.pending_download.take() {
                        finalize_downgrade_with_steps(&mut app, dl);
                    }
                }
                rewind_core::depot::DepotProgress::Error(e) => {
                    app.wizard_state.is_downloading = false;
                    app.depot_stdin = None;
                    if e.contains(".NET runtime not found") {
                        app.wizard_state.error_url =
                            Some("https://dotnet.microsoft.com/download".into());
                    }
                    if let Some(step) = app
                        .wizard_state
                        .steps
                        .iter_mut()
                        .find(|s| s.1 == StepStatus::InProgress)
                    {
                        step.1 = StepStatus::Failed(e.clone());
                    }
                    app.wizard_state.error = Some(e);
                }
            }
        }

        // Recover stdin handle after credential write.
        if let Some(ref mut rx) = app.pending_stdin_return {
            if let Ok(stdin) = rx.try_recv() {
                app.depot_stdin = Some(stdin);
                app.pending_stdin_return = None;
            }
        }

        // Timeout detection: if DepotDownloader has been silent for 30s during download,
        // it may be stuck on an undetected prompt.
        if app.depot_stdin.is_some() && app.wizard_state.prompt_input.is_none() {
            if let Some(last) = app.last_depot_output {
                if last.elapsed() > Duration::from_secs(30)
                    && app.wizard_state.error.is_none()
                {
                    app.wizard_state.error = Some(
                        "DepotDownloader may be waiting for input. Press [R] to restart with terminal mode.".into()
                    );
                }
            }
        }

        // Receive loaded images and convert to cached protocols
        while let Ok((app_id, maybe_img)) = image_rx.try_recv() {
            app.image_state.pending_fetches.remove(&app_id);
            if let Some(img) = maybe_img {
                if let Some(ref picker) = app.image_picker {
                    let protocol = picker.new_resize_protocol(img);
                    app.image_state.protocols.insert(app_id, protocol);
                }
            }
        }

        // Trigger image fetch for selected game if needed
        if app.image_picker.is_some() {
            if let Some(game) = app.selected_game() {
                let app_id = game.app_id;
                if !app.image_state.protocols.contains_key(&app_id)
                    && !app.image_state.pending_fetches.contains(&app_id)
                {
                    app.image_state.pending_fetches.insert(app_id);
                    let tx = image_tx.clone();
                    tokio::spawn(async move {
                        let result = async {
                            let images_dir = rewind_core::image_cache::images_dir()?;
                            // Try cached composited image first, otherwise fetch and composite
                            let bytes = match rewind_core::image_cache::load_cached_composited(&images_dir, app_id) {
                                Some(b) => b,
                                None => rewind_core::image_cache::fetch_and_composite(&images_dir, app_id).await?,
                            };
                            let img = image::load_from_memory(&bytes)?;
                            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(img)
                        }
                        .await;
                        let _ = tx.send((app_id, result.ok())).await;
                    });
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
        Screen::SwitchOverlay => handle_switch_overlay(app, key),
        Screen::ReshadeSetup => handle_reshade_setup(app, key),
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
            if let Some(g) = app.selected_game() {
                let steam_running = rewind_core::steam_guard::is_steam_running();
                app.wizard_state = DowngradeWizardState {
                    app_id: g.app_id,
                    depot_id: g.depot_id,
                    steam_warning: steam_running,
                    ..Default::default()
                };
                app.screen = Screen::DowngradeWizard;
            }
        }
        KeyCode::Char('u') => {
            if app.selected_game_entry().map(|e| e.cached_manifest_ids.len() > 1).unwrap_or(false) {
                let steam_running = rewind_core::steam_guard::is_steam_running();
                app.version_picker_state = app::VersionPickerState {
                    selected_index: 0,
                    steam_warning: steam_running,
                    error: None,
                };
                app.screen = Screen::VersionPicker;
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
        KeyCode::Char('r') => {
            let Some(game) = app.selected_game().cloned() else { return };
            let game_id = game.app_id;
            let entry = app.games_config.games.iter().find(|e| e.app_id == game_id);

            // [R] requires a tracked game (GameEntry must exist in games_config)
            if entry.is_none() {
                return;
            }

            match entry.and_then(|e| e.reshade.as_ref()) {
                None => {
                    // Tracked game, no ReShade yet — open setup wizard
                    app.reshade_state = app::ReshadeSetupState::default();
                    app.screen = Screen::ReshadeSetup;
                }
                Some(r) if r.enabled => {
                    // Disable: remove symlinks + restore Steam launch options (Linux)
                    let api = r.api.clone();
                    let original_launch_opts = r.original_launch_options.clone();
                    match rewind_core::reshade::disable_reshade(&game.install_path, &api) {
                        Ok(()) => {
                            #[cfg(target_os = "linux")]
                            {
                                let restore = original_launch_opts.as_deref().unwrap_or("");
                                for lc_path in rewind_core::localconfig::find_localconfig_paths() {
                                    if rewind_core::localconfig::write_launch_options(
                                        &lc_path, game_id, restore,
                                    ).is_ok() {
                                        break;
                                    }
                                }
                            }
                            if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game_id) {
                                if let Some(ref mut reshade) = entry.reshade {
                                    reshade.enabled = false;
                                }
                            }
                            app.reshade_state.inline_error = None;
                            let _ = config::save_games(&app.games_config);
                        }
                        Err(e) => {
                            app.reshade_state.inline_error = Some(e.to_string());
                        }
                    }
                }
                Some(r) => {
                    // Enable: re-create symlinks
                    let api = r.api.clone();
                    let shaders_enabled = r.shaders_enabled;
                    let Ok(bin_dir) = config::bin_dir() else { return };
                    let Ok(cache_dir) = config::cache_dir() else { return };
                    let reshade_dll = rewind_core::reshade::reshade_dll_path(&bin_dir);
                    let shaders_src = if shaders_enabled {
                        Some(rewind_core::reshade::reshade_shaders_cache_path(&cache_dir))
                    } else {
                        None
                    };
                    match rewind_core::reshade::enable_reshade(
                        &game.install_path,
                        &api,
                        &reshade_dll,
                        shaders_src.as_deref(),
                    ) {
                        Ok(()) => {
                            if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game_id) {
                                if let Some(ref mut reshade) = entry.reshade {
                                    reshade.enabled = true;
                                }
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let launch_cmd = api.linux_launch_command();
                                for lc_path in rewind_core::localconfig::find_localconfig_paths() {
                                    let orig = rewind_core::localconfig::read_launch_options(&lc_path, game_id);
                                    if rewind_core::localconfig::write_launch_options(&lc_path, game_id, &launch_cmd).is_ok() {
                                        if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game_id) {
                                            if let Some(ref mut reshade) = entry.reshade {
                                                reshade.original_launch_options = orig;
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                            app.reshade_state.inline_error = None;
                            let _ = config::save_games(&app.games_config);
                        }
                        Err(e) => {
                            app.reshade_state.inline_error = Some(e.to_string());
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_wizard(app: &mut App, key: KeyCode) {
    // If a credential prompt is active, handle input for that.
    if app.wizard_state.prompt_input.is_some() {
        match key {
            KeyCode::Char(c) => {
                if let Some(ref mut input) = app.wizard_state.prompt_input {
                    input.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut input) = app.wizard_state.prompt_input {
                    input.pop();
                }
            }
            KeyCode::Enter => {
                if let Some(input) = app.wizard_state.prompt_input.take() {
                    if let Some(mut stdin) = app.depot_stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        let response = format!("{}\n", input);
                        let (tx, rx) = mpsc::channel::<tokio::process::ChildStdin>(1);
                        tokio::spawn(async move {
                            let _ = stdin.write_all(response.as_bytes()).await;
                            let _ = stdin.flush().await;
                            let _ = tx.send(stdin).await;
                        });
                        app.pending_stdin_return = Some(rx);
                    }
                    app.wizard_state.prompt_label = None;
                }
            }
            KeyCode::Esc => {
                if let Some(kill) = app.depot_kill.take() {
                    let _ = kill.try_send(());
                }
                app.depot_stdin = None;
                app.wizard_state.prompt_input = None;
                app.wizard_state.prompt_label = None;
                app.wizard_state.is_downloading = false;
                app.screen = Screen::Main;
                app.wizard_state = DowngradeWizardState::default();
            }
            _ => {}
        }
        return;
    }

    // Normal wizard key handling.
    match key {
        KeyCode::Esc => {
            if let Some(kill) = app.depot_kill.take() {
                let _ = kill.try_send(());
            }
            app.depot_stdin = None;
            app.screen = Screen::Main;
            app.wizard_state = DowngradeWizardState::default();
        }
        KeyCode::Char('p') => {
            if !app.wizard_state.is_downloading {
                let url = rewind_core::steamdb::app_patchnotes_url(app.wizard_state.app_id);
                let _ = open::that(url);
            }
        }
        KeyCode::Char('m') => {
            if !app.wizard_state.is_downloading {
                let url = rewind_core::steamdb::depot_manifests_url(app.wizard_state.depot_id);
                let _ = open::that(url);
            }
        }
        KeyCode::Char('o') => {
            if let Some(ref url) = app.wizard_state.error_url {
                let _ = open::that(url.clone());
            }
        }
        KeyCode::Backspace => {
            if !app.wizard_state.is_downloading {
                app.wizard_state.manifest_input.pop();
            }
        }
        KeyCode::Char('r') if app.wizard_state.is_downloading => {
            // Fallback: kill current download and offer to restart.
            if let Some(kill) = app.depot_kill.take() {
                let _ = kill.try_send(());
            }
            app.depot_stdin = None;
            app.progress_rx = None;
            app.last_depot_output = None;
            app.wizard_state.is_downloading = false;
            app.wizard_state.steps.clear();
            app.wizard_state.depot_lines.clear();
            app.wizard_state.error = Some(
                "Download cancelled. Press [Enter] to retry or [Esc] to go back.".into(),
            );
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
                // Check if this is the currently installed manifest
                let is_current = app
                    .selected_game_entry()
                    .map(|e| e.active_manifest_id == manifest_id)
                    .unwrap_or(false);

                if is_current {
                    app.screen = Screen::Main;
                    return;
                }

                if rewind_core::steam_guard::is_steam_running() {
                    app.version_picker_state.error =
                        Some("Steam is running. Quit Steam before switching versions.".into());
                    return;
                }

                let is_latest = app
                    .selected_game_entry()
                    .map(|e| e.latest_manifest_id == manifest_id)
                    .unwrap_or(false);

                // Initialize switch overlay steps
                let mut steps = vec![
                    (app::StepKind::RepointSymlinks, app::StepStatus::Pending),
                    (app::StepKind::PatchAcf, app::StepStatus::Pending),
                ];
                if is_latest {
                    steps.push((app::StepKind::LockAcf, app::StepStatus::Done));
                } else {
                    steps.push((app::StepKind::LockAcf, app::StepStatus::Pending));
                }

                app.switch_overlay_state = app::SwitchOverlayState {
                    steps,
                    target_manifest: manifest_id.clone(),
                    done: false,
                    lock_skipped: is_latest,
                };
                app.screen = Screen::SwitchOverlay;

                switch_to_cached_version(app, manifest_id, is_latest);
            }
        }
        _ => {}
    }
}

fn handle_switch_overlay(app: &mut App, key: KeyCode) {
    if key == KeyCode::Esc && app.switch_overlay_state.done {
        app.switch_overlay_state = app::SwitchOverlayState::default();
        app.screen = Screen::Main;
    }
}

fn handle_reshade_setup(app: &mut App, key: KeyCode) {
    use app::ReshadeSetupStep;

    match app.reshade_state.step {
        ReshadeSetupStep::PickApi => match key {
            KeyCode::Esc => {
                app.screen = Screen::Main;
                app.reshade_state = app::ReshadeSetupState::default();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.reshade_state.selected_api > 0 {
                    app.reshade_state.selected_api -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.reshade_state.selected_api < 3 {
                    app.reshade_state.selected_api += 1;
                }
            }
            KeyCode::Enter => {
                app.reshade_state.step = ReshadeSetupStep::ConfirmShaders;
            }
            _ => {}
        },
        ReshadeSetupStep::ConfirmShaders => match key {
            KeyCode::Esc => {
                app.reshade_state.step = ReshadeSetupStep::PickApi;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.reshade_state.download_shaders = true;
                start_reshade_download(app);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                app.reshade_state.download_shaders = false;
                start_reshade_download(app);
            }
            _ => {}
        },
        ReshadeSetupStep::Downloading => {
            if key == KeyCode::Esc
                && (app.reshade_state.done || app.reshade_state.error.is_some())
            {
                app.screen = Screen::Main;
                app.reshade_state = app::ReshadeSetupState::default();
                app.reshade_progress_rx = None;
            }
        }
    }
}

fn start_reshade_download(app: &mut App) {
    let Ok(bin_dir) = config::bin_dir() else { return };
    let Ok(cache_dir) = config::cache_dir() else { return };
    let download_shaders = app.reshade_state.download_shaders;

    let (tx, rx) = mpsc::channel(64);
    app.reshade_progress_rx = Some(rx);
    app.reshade_state.step = app::ReshadeSetupStep::Downloading;
    app.reshade_state.lines.clear();
    app.reshade_state.done = false;
    app.reshade_state.error = None;

    tokio::spawn(async move {
        match rewind_core::reshade::download_reshade(&bin_dir, tx.clone()).await {
            Ok(_) => {}
            Err(e) => {
                let _ = tx.send(rewind_core::reshade::ReshadeProgress::Error(e.to_string())).await;
                return;
            }
        }

        if download_shaders {
            let shaders_dir = rewind_core::reshade::reshade_shaders_cache_path(&cache_dir);
            if let Err(e) = rewind_core::reshade::download_shaders(&shaders_dir, tx.clone()).await {
                let _ = tx.send(rewind_core::reshade::ReshadeProgress::Error(e.to_string())).await;
                return;
            }
        }

        let _ = tx.send(rewind_core::reshade::ReshadeProgress::Done).await;
    });
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
    use crate::app::{StepKind, StepStatus};

    if app.config.steam_username.is_none() {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    if rewind_core::steam_guard::is_steam_running() {
        app.wizard_state.error = Some("Steam is running. Quit Steam before downloading.".into());
        return;
    }

    let Ok(bin_dir) = config::bin_dir() else { return };
    let Some(game) = app.selected_game().cloned() else { return };
    let Some(username) = app.config.steam_username.clone() else { return };
    let Ok(cache_root) = config::cache_dir() else { return };

    let manifest_id = app.wizard_state.manifest_input.trim().to_string();
    let cache_dir = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &manifest_id,
    );

    let (tx, rx) = mpsc::channel(64);
    app.progress_rx = Some(rx);
    app.wizard_state.is_downloading = true;
    app.wizard_state.depot_lines.clear();
    app.wizard_state.error = None;
    app.wizard_state.error_url = None;
    app.wizard_state.prompt_input = None;
    app.wizard_state.prompt_label = None;
    app.wizard_state.steps = vec![
        (StepKind::CheckDotnet, StepStatus::InProgress),
        (StepKind::DownloadDepot, StepStatus::Pending),
        (StepKind::DownloadManifest, StepStatus::Pending),
        (StepKind::BackupFiles, StepStatus::Pending),
        (StepKind::LinkFiles, StepStatus::Pending),
        (StepKind::PatchManifest, StepStatus::Pending),
        (StepKind::LockManifest, StepStatus::Pending),
    ];

    app.pending_download = Some(PendingDownload {
        app_id: game.app_id,
        depot_id: game.depot_id,
        manifest_id,
        username: username.clone(),
        cache_dir: cache_dir.clone(),
        game_name: game.name.clone(),
        game_install_path: game.install_path.clone(),
        current_manifest_id: game.manifest_id.clone(),
        acf_path: game.acf_path.clone(),
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        if !rewind_core::depot::check_dotnet().await {
            let _ = tx2
                .send(rewind_core::depot::DepotProgress::Error(
                    ".NET runtime not found. Press [O] to open the download page.".into(),
                ))
                .await;
            return;
        }
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_DONE:CheckDotnet".into(),
            ))
            .await;

        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadDepot".into(),
            ))
            .await;
        let binary = match rewind_core::depot::ensure_depot_downloader(&bin_dir).await {
            Ok(b) => b,
            Err(e) => {
                let _ = tx2
                    .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                    .await;
                return;
            }
        };
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_DONE:DownloadDepot".into(),
            ))
            .await;

        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadManifest".into(),
            ))
            .await;
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary })
            .await;
    });
}

fn switch_to_cached_version(app: &mut App, manifest_id: String, is_latest: bool) {
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

    // Step 1: Repoint symlinks
    app.set_switch_step_status(&app::StepKind::RepointSymlinks, app::StepStatus::InProgress);
    if let Err(e) = rewind_core::cache::repoint_symlinks(&game.install_path, &new_cache) {
        app.set_switch_step_status(
            &app::StepKind::RepointSymlinks,
            app::StepStatus::Failed(e.to_string()),
        );
        app.switch_overlay_state.done = true;
        return;
    }
    app.set_switch_step_status(&app::StepKind::RepointSymlinks, app::StepStatus::Done);

    // Extract data from entry before doing step-status updates (to avoid borrow conflict)
    let entry_data = app
        .games_config
        .games
        .iter()
        .find(|e| e.app_id == game.app_id)
        .map(|e| (e.acf_path(), e.latest_buildid.clone(), e.latest_manifest_id.clone(), e.depot_id));

    let Some((acf_path, buildid, manifest_for_acf, depot_id)) = entry_data else {
        return;
    };

    // Update active_manifest_id
    if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game.app_id) {
        entry.active_manifest_id = manifest_id.clone();
    }

    // Step 2: Patch ACF
    app.set_switch_step_status(&app::StepKind::PatchAcf, app::StepStatus::InProgress);
    let _ = rewind_core::immutability::unlock_file(&acf_path);

    if let Err(e) = rewind_core::patcher::patch_acf_file(
        &acf_path,
        &buildid,
        &manifest_for_acf,
        depot_id,
    ) {
        app.set_switch_step_status(&app::StepKind::PatchAcf, app::StepStatus::Failed(e.to_string()));
        app.switch_overlay_state.done = true;
        return;
    }
    app.set_switch_step_status(&app::StepKind::PatchAcf, app::StepStatus::Done);

    // Step 3: Lock ACF (only if not switching to latest)
    if is_latest {
        // Don't lock — let Steam manage updates
        if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game.app_id) {
            entry.acf_locked = false;
        }
    } else {
        app.set_switch_step_status(&app::StepKind::LockAcf, app::StepStatus::InProgress);
        if let Err(e) = rewind_core::immutability::lock_file(&acf_path) {
            app.set_switch_step_status(
                &app::StepKind::LockAcf,
                app::StepStatus::Failed(e.to_string()),
            );
            app.switch_overlay_state.done = true;
            return;
        }
        app.set_switch_step_status(&app::StepKind::LockAcf, app::StepStatus::Done);
        if let Some(entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game.app_id) {
            entry.acf_locked = true;
        }
    }

    let _ = config::save_games(&app.games_config);
    app.switch_overlay_state.done = true;
}

fn step_kind_from_str(s: &str) -> Option<app::StepKind> {
    match s {
        "CheckDotnet" => Some(app::StepKind::CheckDotnet),
        "DownloadDepot" => Some(app::StepKind::DownloadDepot),
        "DownloadManifest" => Some(app::StepKind::DownloadManifest),
        "BackupFiles" => Some(app::StepKind::BackupFiles),
        "LinkFiles" => Some(app::StepKind::LinkFiles),
        "PatchManifest" => Some(app::StepKind::PatchManifest),
        "LockManifest" => Some(app::StepKind::LockManifest),
        _ => None,
    }
}

fn finalize_reshade(app: &mut App) {
    use rewind_core::config::{ReshadeApi, ReshadeEntry};

    const APIS: &[ReshadeApi] = &[
        ReshadeApi::Dxgi,
        ReshadeApi::D3d9,
        ReshadeApi::OpenGl32,
        ReshadeApi::Vulkan1,
    ];

    let api = match APIS.get(app.reshade_state.selected_api) {
        Some(a) => a.clone(),
        None => {
            app.reshade_state.error = Some("Invalid API selection.".into());
            return;
        }
    };

    let Ok(bin_dir) = config::bin_dir() else { return };
    let Ok(cache_dir) = config::cache_dir() else { return };

    let reshade_dll = rewind_core::reshade::reshade_dll_path(&bin_dir);
    let shaders_enabled = app.reshade_state.download_shaders;
    let shaders_src = if shaders_enabled {
        Some(rewind_core::reshade::reshade_shaders_cache_path(&cache_dir))
    } else {
        None
    };

    let Some(game) = app.selected_game().cloned() else { return };

    if let Err(e) = rewind_core::reshade::enable_reshade(
        &game.install_path,
        &api,
        &reshade_dll,
        shaders_src.as_deref(),
    ) {
        app.reshade_state.error = Some(format!("Failed to enable ReShade: {}", e));
        return;
    }

    // Linux: write WINEDLLOVERRIDES to Steam launch options automatically.
    #[cfg(target_os = "linux")]
    let original_launch_options: Option<String> = {
        let launch_cmd = api.linux_launch_command();
        let localconfig_paths = rewind_core::localconfig::find_localconfig_paths();
        let mut original = None;
        for lc_path in &localconfig_paths {
            let orig = rewind_core::localconfig::read_launch_options(lc_path, game.app_id);
            if rewind_core::localconfig::write_launch_options(lc_path, game.app_id, &launch_cmd).is_ok() {
                original = orig;
                app.reshade_state.lines.push("Steam launch options updated.".into());
                break;
            }
        }
        original
    };
    #[cfg(not(target_os = "linux"))]
    let original_launch_options: Option<String> = None;

    let entry = ReshadeEntry {
        api,
        enabled: true,
        shaders_enabled,
        original_launch_options,
    };

    if let Some(game_entry) = app.games_config.games.iter_mut().find(|e| e.app_id == game.app_id) {
        game_entry.reshade = Some(entry);
    }

    let _ = config::save_games(&app.games_config);
    app.reshade_state.done = true;
    app.reshade_state.lines.push("ReShade enabled!".into());
}

fn finalize_downgrade_with_steps(app: &mut App, dl: PendingDownload) {
    use crate::app::{StepKind, StepStatus};

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

    // Step 4: Backup + Step 5: Link
    app.set_step_status(&StepKind::BackupFiles, StepStatus::InProgress);
    if let Err(e) =
        rewind_core::cache::apply_downloaded(&dl.game_install_path, &target_cache, &current_cache)
    {
        app.set_step_status(&StepKind::BackupFiles, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to apply files: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::BackupFiles, StepStatus::Done);
    app.set_step_status(&StepKind::LinkFiles, StepStatus::Done);

    // Update game config
    let existing = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == dl.app_id);

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
            reshade: None,
        });
    }

    // Unlock ACF before patching (it may be locked from a previous downgrade).
    let _ = rewind_core::immutability::unlock_file(&dl.acf_path);

    // Step 6: Patch ACF
    app.set_step_status(&StepKind::PatchManifest, StepStatus::InProgress);
    if let Err(e) = rewind_core::patcher::patch_acf_file(
        &dl.acf_path,
        &latest_buildid,
        &dl.current_manifest_id,
        dl.depot_id,
    ) {
        app.set_step_status(&StepKind::PatchManifest, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to patch ACF: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::PatchManifest, StepStatus::Done);

    // Step 7: Lock ACF
    app.set_step_status(&StepKind::LockManifest, StepStatus::InProgress);
    if let Err(e) = rewind_core::immutability::lock_file(&dl.acf_path) {
        app.set_step_status(&StepKind::LockManifest, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to lock ACF: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::LockManifest, StepStatus::Done);

    let _ = config::save_games(&app.games_config);
    app.wizard_state.is_downloading = false;
    app.screen = Screen::Main;
}
