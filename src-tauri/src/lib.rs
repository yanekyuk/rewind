pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use application::auth::{clear_from_keychain, load_from_keychain, save_to_keychain, AuthStore};
use domain::auth::Credentials;
use domain::game::GameInfo;
use domain::manifest::ManifestListEntry;
use error::RewindError;
use infrastructure::depot_downloader;
use infrastructure::steam;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Store Steam credentials in memory for the current session.
///
/// Validates that username and password are non-empty, then stores them
/// in the application's in-memory auth store. Credentials are never
/// persisted to disk.
#[tauri::command]
fn set_credentials(
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
    state
        .set(credentials.clone())
        .map_err(|e| RewindError::AuthFailed(e.to_string()))?;
    save_to_keychain(&credentials);
    Ok(())
}

/// Check whether credentials have been stored in the current session.
///
/// Returns `true` if credentials are available for SteamKit sidecar operations.
#[tauri::command]
fn get_auth_state(state: tauri::State<'_, AuthStore>) -> bool {
    state.is_set()
}

/// Remove credentials from memory and from the OS keychain.
#[tauri::command]
fn clear_credentials(state: tauri::State<'_, AuthStore>) {
    state.clear();
    clear_from_keychain();
}

/// List all installed Steam games across all detected Steam library folders.
///
/// This Tauri IPC command:
/// 1. Detects Steam installation paths (default + additional library folders)
/// 2. Scans each steamapps directory for appmanifest ACF files
/// 3. Parses each into an AppState, then converts to GameInfo
/// 4. Returns the full list to the frontend
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

    // Sort by name for consistent ordering
    games.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(games)
}

/// List available manifests for a depot using the SteamKit sidecar.
///
/// This Tauri IPC command:
/// 1. Reads credentials from the AuthStore
/// 2. Spawns the SteamKit sidecar with stored credentials
/// 3. Collects the manifest listing JSON output (newline-delimited)
/// 4. Parses it into ManifestListEntry structs
/// 5. Returns the list to the frontend
#[tauri::command]
async fn list_manifests(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthStore>,
    app_id: String,
    depot_id: String,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    let credentials = state.get().ok_or_else(|| {
        RewindError::AuthRequired("Credentials not set. Please sign in first.".to_string())
    })?;
    depot_downloader::list_manifests(&app, &app_id, &depot_id, &credentials).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Pre-populate AuthStore from the OS keychain if credentials were saved previously.
    let auth_store = AuthStore::default();
    if let Some(saved) = load_from_keychain() {
        let _ = auth_store.set(saved);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(auth_store)
        .invoke_handler(tauri::generate_handler![
            greet,
            list_games,
            list_manifests,
            set_credentials,
            get_auth_state,
            clear_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
