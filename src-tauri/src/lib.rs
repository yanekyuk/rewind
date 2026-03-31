pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use std::path::Path;

use tauri::Emitter;
use tauri::Manager;
use tauri::webview::WebviewWindowBuilder;

use application::auth::AuthState;
use application::downgrade::{run_downgrade, DowngradeServices};
use domain::auth::Credentials;
use domain::downgrade::{DowngradeParams, DowngradeProgress};
use domain::game::{GameInfo, SteamDepotInfo};
use domain::manifest::{ManifestListEntry, DepotManifest};
use domain::vdf::AcfPatchParams;
use error::RewindError;
use infrastructure::depot_downloader;
use infrastructure::downgrade as infra_downgrade;
use infrastructure::sidecar::SidecarState;
use infrastructure::steam;

/// Real implementation of DowngradeServices that delegates to infrastructure.
///
/// This struct acts as the composition root adapter, wiring the application
/// layer's trait to the concrete infrastructure functions.
struct RealDowngradeServices {
    app: tauri::AppHandle,
    sidecar: infrastructure::sidecar::SidecarHandle,
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
        _credentials: &Credentials,
    ) -> Result<DepotManifest, RewindError> {
        depot_downloader::get_manifest(&self.sidecar, app_id, depot_id, manifest_id).await
    }

    async fn download(
        &self,
        app_id: &str,
        depot_id: &str,
        manifest_id: &str,
        output_dir: &str,
        filelist_path: &str,
        _credentials: &Credentials,
    ) -> Result<(), RewindError> {
        depot_downloader::download(
            &self.sidecar,
            &self.app,
            app_id,
            depot_id,
            manifest_id,
            output_dir,
            filelist_path,
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

/// Authenticate with Steam via the sidecar daemon.
///
/// Sends a `login` command to the SteamKit sidecar with the provided
/// credentials. On success, the sidecar persists a RefreshToken so
/// subsequent app starts can use `check_session` for silent login.
#[tauri::command]
async fn login(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
    username: String,
    password: String,
    guard_code: Option<String>,
) -> Result<(), RewindError> {
    let credentials = Credentials {
        username,
        password,
        guard_code,
    };

    eprintln!("[login] authenticating with Steam...");
    let sidecar = sidecar_state.get(&app).await?;
    depot_downloader::login(sidecar, &credentials).await?;
    eprintln!("[login] authentication successful");

    state.set_logged_in(&credentials.username);
    Ok(())
}

/// Check for an existing sidecar session (saved RefreshToken).
///
/// Sends a `check-session` command to the sidecar. If a valid RefreshToken
/// exists on disk, the sidecar logs in silently and returns the username.
/// The frontend calls this on mount to skip the login screen.
#[tauri::command]
async fn check_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
) -> Result<Option<String>, RewindError> {
    eprintln!("[check_session] checking for saved session...");
    let sidecar = sidecar_state.get(&app).await?;
    match depot_downloader::check_session(sidecar).await {
        Ok(username) => {
            eprintln!("[check_session] silent login successful for {}", username);
            state.set_logged_in(&username);
            Ok(Some(username))
        }
        Err(_) => {
            eprintln!("[check_session] no valid session found");
            Ok(None)
        }
    }
}

/// Check whether the user has an active authenticated session.
#[tauri::command]
fn get_auth_state(state: tauri::State<'_, AuthState>) -> bool {
    state.is_logged_in()
}

/// Return the username of the authenticated user, if any.
#[tauri::command]
fn get_username(state: tauri::State<'_, AuthState>) -> Option<String> {
    state.username()
}

/// Log out: send logout to the sidecar and clear local auth state.
#[tauri::command]
async fn logout(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
) -> Result<(), RewindError> {
    if let Ok(sidecar) = sidecar_state.get(&app).await {
        let _ = depot_downloader::logout(sidecar).await;
    }
    state.clear();
    Ok(())
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
/// 1. Comparing -- fetch manifests, diff, generate filelist
/// 2. Downloading -- download changed files via SteamKit sidecar
/// 3. Applying -- copy files, delete removed, patch ACF, lock ACF
/// 4. Complete -- emit success or error event
///
/// Progress is emitted on the `downgrade-progress` Tauri event channel.
/// The frontend should listen to this channel for real-time updates.
#[tauri::command]
async fn start_downgrade(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
    params: DowngradeParams,
) -> Result<(), RewindError> {
    if !state.is_logged_in() {
        return Err(RewindError::AuthRequired(
            "No authenticated session. Please sign in.".to_string(),
        ));
    }

    // The sidecar is already authenticated; credentials are only needed
    // as a placeholder for the DowngradeServices trait interface.
    let credentials = Credentials {
        username: state.username().unwrap_or_default(),
        password: String::new(),
        guard_code: None,
    };

    eprintln!(
        "[start_downgrade] starting pipeline for app={} depot={} target={}",
        params.app_id, params.depot_id, params.target_manifest_id
    );

    let sidecar = sidecar_state.get(&app).await?.clone();
    let services = RealDowngradeServices {
        app: app.clone(),
        sidecar,
    };

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
/// Requires an active sidecar session (login or check_session must
/// have succeeded). Returns `AuthRequired` if not logged in.
#[tauri::command]
async fn list_manifests(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
    app_id: String,
    depot_id: String,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    if !state.is_logged_in() {
        return Err(RewindError::AuthRequired(
            "No authenticated session. Please sign in.".to_string(),
        ));
    }
    let sidecar = sidecar_state.get(&app).await?;
    eprintln!("[list_manifests] sending command for app={} depot={}", app_id, depot_id);
    let start = std::time::Instant::now();
    let result = depot_downloader::list_manifests(sidecar, &app_id, &depot_id).await;
    eprintln!("[list_manifests] completed in {:?} with {} entries", start.elapsed(), result.as_ref().map_or(0, |v| v.len()));
    result
}

/// List all depots for an app using the SteamKit sidecar.
///
/// Queries Steam's PICS API to enumerate every depot for the given app,
/// returning metadata (name, max size, DLC app ID) for each.
/// Requires an active sidecar session.
#[tauri::command]
async fn list_depots(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    sidecar_state: tauri::State<'_, SidecarState>,
    app_id: String,
) -> Result<Vec<SteamDepotInfo>, RewindError> {
    if !state.is_logged_in() {
        return Err(RewindError::AuthRequired(
            "No authenticated session. Please sign in.".to_string(),
        ));
    }
    let sidecar = sidecar_state.get(&app).await?;
    eprintln!("[list_depots] sending command for app={}", app_id);
    let start = std::time::Instant::now();
    let result = depot_downloader::list_depots(sidecar, &app_id).await;
    eprintln!(
        "[list_depots] completed in {:?} with {} entries",
        start.elapsed(),
        result.as_ref().map_or(0, |v| v.len())
    );
    result
}

/// JavaScript injected into the SteamDB webview to extract manifest data.
///
/// The script looks for the manifest history table on the SteamDB depot page,
/// extracts manifest IDs, dates, and branch labels from each row, and emits
/// the results back to the main window via the `steamdb-manifests` Tauri event.
const STEAMDB_EXTRACTION_JS: &str = r#"
(function() {
  try {
    // Wait for the table to be fully loaded
    function extractManifests() {
      const rows = document.querySelectorAll('.table .app-history .depot-manifest, table.table tbody tr');
      const manifests = [];

      // Try the standard SteamDB depot manifests table format
      const table = document.querySelector('.table-responsive table, table.table');
      if (table) {
        const trs = table.querySelectorAll('tbody tr');
        for (const tr of trs) {
          const cells = tr.querySelectorAll('td');
          if (cells.length >= 2) {
            const manifestId = cells[0]?.textContent?.trim();
            const date = cells[1]?.textContent?.trim() || null;
            const branch = cells.length >= 3 ? cells[2]?.textContent?.trim() || null : null;

            if (manifestId && /^\d+$/.test(manifestId)) {
              manifests.push({
                manifest_id: manifestId,
                date: date,
                branch: branch
              });
            }
          }
        }
      }

      if (manifests.length > 0) {
        window.__TAURI__?.event?.emit('steamdb-manifests', manifests)
          .catch(function(e) { console.error('Failed to emit manifests:', e); });
      }
    }

    // Try extracting immediately, then retry after a delay for dynamic content
    extractManifests();
    setTimeout(extractManifests, 2000);
    setTimeout(extractManifests, 5000);
  } catch (e) {
    console.error('SteamDB extraction error:', e);
    window.__TAURI__?.event?.emit('steamdb-manifests-error', e.message || 'Extraction failed')
      .catch(function() {});
  }
})();
"#;

/// Open a SteamDB webview window showing the manifest history for a depot.
///
/// Creates a new browser-like window pointing to
/// `https://steamdb.info/depot/<depotId>/manifests/`. On page load, JavaScript
/// is injected to extract the manifest table data. Extracted manifests are
/// emitted on the `steamdb-manifests` event channel.
#[tauri::command]
async fn open_steamdb_webview(
    app: tauri::AppHandle,
    depot_id: String,
) -> Result<(), RewindError> {
    let url = format!("https://steamdb.info/depot/{}/manifests/", depot_id);
    let label = format!("steamdb-{}", depot_id);

    // If a window with this label already exists, focus it instead of creating a new one
    if let Some(existing) = app.get_webview_window(&label) {
        existing.set_focus().map_err(|e| {
            RewindError::Infrastructure(format!("Failed to focus SteamDB window: {}", e))
        })?;
        return Ok(());
    }

    let js = STEAMDB_EXTRACTION_JS.to_string();

    let _webview_window = WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::External(url.parse().map_err(|e| {
            RewindError::Infrastructure(format!("Invalid SteamDB URL: {}", e))
        })?),
    )
    .title(format!("SteamDB - Depot {}", depot_id))
    .inner_size(1024.0, 768.0)
    .on_page_load(move |webview, _payload| {
        let js = js.clone();
        let _ = webview.eval(&js);
    })
    .build()
    .map_err(|e| {
        RewindError::Infrastructure(format!("Failed to create SteamDB webview: {}", e))
    })?;

    Ok(())
}

/// Close the SteamDB webview window for a depot.
#[tauri::command]
async fn close_steamdb_webview(
    app: tauri::AppHandle,
    depot_id: String,
) -> Result<(), RewindError> {
    let label = format!("steamdb-{}", depot_id);
    if let Some(window) = app.get_webview_window(&label) {
        window.destroy().map_err(|e| {
            RewindError::Infrastructure(format!("Failed to close SteamDB window: {}", e))
        })?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AuthState::default())
        .manage(SidecarState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            list_games,
            list_depots,
            list_manifests,
            start_downgrade,
            login,
            check_session,
            get_auth_state,
            get_username,
            logout,
            open_steamdb_webview,
            close_steamdb_webview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
