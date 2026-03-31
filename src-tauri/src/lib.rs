pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use std::path::Path;

use tauri::Emitter;

use application::auth::{
    clear_saved_username, delete_from_keychain, load_from_keychain, load_username, save_to_keychain,
    save_username, AuthStore,
};
use application::downgrade::{run_downgrade, DowngradeServices};
use domain::auth::Credentials;
use domain::downgrade::{DowngradeParams, DowngradeProgress};
use domain::game::GameInfo;
use domain::manifest::{ManifestListEntry, DepotManifest};
use domain::vdf::AcfPatchParams;
use error::RewindError;
use infrastructure::depot_downloader;
use infrastructure::downgrade as infra_downgrade;
use infrastructure::steam;

/// Real implementation of DowngradeServices that delegates to infrastructure.
///
/// This struct acts as the composition root adapter, wiring the application
/// layer's trait to the concrete infrastructure functions.
struct RealDowngradeServices {
    app: tauri::AppHandle,
}

impl DowngradeServices for RealDowngradeServices {
    fn emit_progress(&self, progress: DowngradeProgress) {
        let _ = self.app.emit("downgrade-progress", progress);
    }

    async fn get_manifest(
        &self,
        app_id: &str,
        depot_id: &str,
        manifest_id: &str,
        credentials: &Credentials,
    ) -> Result<DepotManifest, RewindError> {
        depot_downloader::get_manifest(&self.app, app_id, depot_id, manifest_id, credentials).await
    }

    async fn download(
        &self,
        app_id: &str,
        depot_id: &str,
        manifest_id: &str,
        output_dir: &str,
        filelist_path: &str,
        credentials: &Credentials,
    ) -> Result<(), RewindError> {
        depot_downloader::download(
            &self.app,
            app_id,
            depot_id,
            manifest_id,
            output_dir,
            filelist_path,
            credentials,
        )
        .await
    }

    async fn apply_files(
        &self,
        install_path: &Path,
        download_dir: &Path,
    ) -> Result<(), RewindError> {
        infra_downgrade::apply_files(install_path, download_dir).await
    }

    async fn delete_removed_files(
        &self,
        install_path: &Path,
        removed_files: &[String],
    ) -> Result<(), RewindError> {
        infra_downgrade::delete_removed_files(install_path, removed_files).await
    }

    async fn patch_acf(
        &self,
        acf_path: &Path,
        params: &AcfPatchParams,
    ) -> Result<(), RewindError> {
        infra_downgrade::patch_acf(acf_path, params).await
    }

    async fn lock_acf(&self, acf_path: &Path) -> Result<(), RewindError> {
        infra_downgrade::lock_acf(acf_path).await
    }

    async fn is_steam_running(&self) -> bool {
        infra_downgrade::is_steam_running().await
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<(), RewindError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                RewindError::Infrastructure(format!(
                    "failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        tokio::fs::write(path, content).await.map_err(|e| {
            RewindError::Infrastructure(format!(
                "failed to write file {}: {}",
                path.display(),
                e
            ))
        })
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Authenticate with Steam and store credentials for the session.
///
/// Spawns the SteamKit sidecar `login` command to perform actual Steam
/// authentication (including phone approval / Steam Guard). On success,
/// the sidecar persists a session token so subsequent commands skip auth.
#[tauri::command]
async fn set_credentials(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthStore>,
    username: String,
    password: String,
    guard_code: Option<String>,
) -> Result<(), RewindError> {
    let credentials = Credentials {
        username,
        password,
        guard_code,
    };

    // Actually authenticate with Steam via the sidecar
    eprintln!("[set_credentials] authenticating with Steam...");
    depot_downloader::login(&app, &credentials).await?;
    eprintln!("[set_credentials] authentication successful");

    save_username(&credentials.username);
    if !credentials.password.is_empty() {
        save_to_keychain(&credentials.username, &credentials.password);
    }
    state
        .set(credentials)
        .map_err(|e| RewindError::AuthFailed(e.to_string()))?;
    Ok(())
}

/// Check whether the user has an active or saved session.
///
/// Returns `true` if credentials are available this session OR if a
/// username was saved from a previous session (sidecar session may still be valid).
#[tauri::command]
fn get_auth_state(state: tauri::State<'_, AuthStore>) -> bool {
    state.is_set() || state.has_saved_session()
}

/// Return the username of the authenticated or saved user, if any.
#[tauri::command]
fn get_username(state: tauri::State<'_, AuthStore>) -> Option<String> {
    state.username()
}

/// Check whether full credentials (username + password) are stored.
///
/// Returns `true` if the password was loaded from the OS keychain on startup
/// or if credentials were set during this session. Used by the frontend to
/// show the "Welcome back" UI.
#[tauri::command]
fn has_credentials(state: tauri::State<'_, AuthStore>) -> bool {
    state.has_stored_password()
}

/// Remove credentials from memory, saved username from disk, and password
/// from the OS keychain.
#[tauri::command]
fn clear_credentials(state: tauri::State<'_, AuthStore>) {
    // Get the username before clearing so we can delete from keychain
    if let Some(username) = state.username() {
        delete_from_keychain(&username);
    }
    state.clear();
    clear_saved_username();
}

/// List all installed Steam games across all detected Steam library folders.
#[tauri::command]
async fn list_games() -> Result<Vec<GameInfo>, RewindError> {
    let steamapps_dirs = steam::discover_steamapps_dirs().await;

    if steamapps_dirs.is_empty() {
        return Ok(Vec::new());
    }

    let mut games = Vec::new();

    for dir in &steamapps_dirs {
        match steam::scan_appmanifests(dir).await {
            Ok(manifests) => {
                for (app_state, steamapps_path) in manifests {
                    let game_info = GameInfo::from_app_state(&app_state, &steamapps_path);
                    games.push(game_info);
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to scan {}: {}", dir.display(), e);
            }
        }
    }

    games.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(games)
}

/// Start the downgrade pipeline for a game.
///
/// Orchestrates the full 4-phase downgrade workflow:
/// 1. Comparing — fetch manifests, diff, generate filelist
/// 2. Downloading — download changed files via SteamKit sidecar
/// 3. Applying — copy files, delete removed, patch ACF, lock ACF
/// 4. Complete — emit success or error event
///
/// Progress is emitted on the `downgrade-progress` Tauri event channel.
/// The frontend should listen to this channel for real-time updates.
#[tauri::command]
async fn start_downgrade(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthStore>,
    params: DowngradeParams,
) -> Result<(), RewindError> {
    let credentials = state.get_or_saved().ok_or_else(|| {
        RewindError::AuthRequired("No credentials available. Please sign in.".to_string())
    })?;

    eprintln!(
        "[start_downgrade] starting pipeline for app={} depot={} target={}",
        params.app_id, params.depot_id, params.target_manifest_id
    );

    let services = RealDowngradeServices { app: app.clone() };

    let result = run_downgrade(&params, &credentials, &services).await;

    if let Err(ref e) = result {
        let _ = app.emit(
            "downgrade-progress",
            DowngradeProgress::Error {
                message: e.to_string(),
            },
        );
    }

    result
}

/// List available manifests for a depot using the SteamKit sidecar.
///
/// Uses full credentials if available, or falls back to the saved username
/// (with empty password) to let the sidecar attempt session-token auth.
/// If the sidecar reports AUTH_REQUIRED (no saved session and no password),
/// returns `RewindError::AuthRequired` so the frontend shows the login form.
#[tauri::command]
async fn list_manifests(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthStore>,
    app_id: String,
    depot_id: String,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    let credentials = state.get_or_saved().ok_or_else(|| {
        RewindError::AuthRequired("No credentials available. Please sign in.".to_string())
    })?;
    eprintln!("[list_manifests] spawning sidecar for app={} depot={}", app_id, depot_id);
    let start = std::time::Instant::now();
    let result = depot_downloader::list_manifests(&app, &app_id, &depot_id, &credentials).await;
    eprintln!("[list_manifests] completed in {:?} with {} entries", start.elapsed(), result.as_ref().map_or(0, |v| v.len()));
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let saved_username = load_username();
    let saved_password = saved_username
        .as_deref()
        .and_then(load_from_keychain);
    let auth_store = AuthStore::with_saved_credentials(saved_username, saved_password);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(auth_store)
        .invoke_handler(tauri::generate_handler![
            greet,
            list_games,
            list_manifests,
            start_downgrade,
            set_credentials,
            get_auth_state,
            get_username,
            has_credentials,
            clear_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
