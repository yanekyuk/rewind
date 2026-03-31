pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use application::auth::{clear_saved_username, load_username, save_username, AuthStore};
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

/// Remove credentials from memory and saved username from disk.
#[tauri::command]
fn clear_credentials(state: tauri::State<'_, AuthStore>) {
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

/// List available manifests for a depot using the SteamKit sidecar.
///
/// Requires full credentials (username + password) from the current session.
/// If only a saved username exists (no password), returns AuthRequired error
/// so the frontend can prompt for re-login.
#[tauri::command]
async fn list_manifests(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthStore>,
    app_id: String,
    depot_id: String,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    let credentials = state.get().ok_or_else(|| {
        RewindError::AuthRequired("Session expired. Please sign in again.".to_string())
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
    let auth_store = AuthStore::with_saved_username(saved_username);

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
            get_username,
            clear_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
