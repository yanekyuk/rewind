pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

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

/// List available manifests for a depot using DepotDownloader.
///
/// This Tauri IPC command:
/// 1. Spawns DepotDownloader with the given credentials
/// 2. Collects the manifest listing output
/// 3. Parses it into ManifestListEntry structs
/// 4. Returns the list to the frontend
#[tauri::command]
async fn list_manifests(
    app: tauri::AppHandle,
    app_id: String,
    depot_id: String,
    username: String,
    password: String,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    depot_downloader::list_manifests(&app, &app_id, &depot_id, &username, &password).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, list_games, list_manifests])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
