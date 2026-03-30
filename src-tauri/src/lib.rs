pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use domain::game::GameInfo;
use error::RewindError;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, list_games])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
