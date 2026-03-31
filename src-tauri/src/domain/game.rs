use serde::Serialize;
use std::path::Path;

use super::vdf::AppState;

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Information about a single installed depot, suitable for frontend display.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepotInfo {
    pub depot_id: String,
    pub manifest: String,
    pub size: String,
}

/// Information about a single installed game, suitable for frontend display.
///
/// This is the domain type that crosses the IPC boundary to the frontend.
/// It is derived from an `AppState` (parsed from an ACF file) combined with
/// the steamapps base path to compute the full installation path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GameInfo {
    pub appid: String,
    pub name: String,
    pub buildid: String,
    pub installdir: String,
    pub depots: Vec<DepotInfo>,
    pub install_path: String,
    pub state_flags: u32,
    pub update_pending: bool,
    pub target_build_id: Option<String>,
    pub bytes_to_download: Option<String>,
    pub size_on_disk: String,
    pub last_updated: Option<String>,
}

impl GameInfo {
    /// Convert an `AppState` into a `GameInfo`, computing the full install path
    /// from the given `steamapps` base directory.
    pub fn from_app_state(app_state: &AppState, steamapps_path: &Path) -> Self {
        let install_path = steamapps_path
            .join("common")
            .join(&app_state.installdir)
            .to_string_lossy()
            .into_owned();

        let mut depots: Vec<DepotInfo> = app_state
            .installed_depots
            .iter()
            .map(|(depot_id, depot)| DepotInfo {
                depot_id: depot_id.clone(),
                manifest: depot.manifest.clone(),
                size: depot.size.clone(),
            })
            .collect();

        // Sort by depot_id for deterministic output
        depots.sort_by(|a, b| a.depot_id.cmp(&b.depot_id));

        let state_flags: u32 = app_state.state_flags.parse().unwrap_or(0);
        let update_pending = app_state
            .target_build_id
            .as_ref()
            .map_or(false, |t| t != &app_state.buildid)
            || app_state
                .bytes_to_download
                .as_ref()
                .map_or(false, |b| b != "0");

        let size_on_disk: u64 = app_state
            .size_on_disk
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| {
                depots.iter().filter_map(|d| d.size.parse::<u64>().ok()).sum()
            });

        // Pass raw epoch timestamp — frontend will format it
        let last_updated = app_state.last_updated.clone();

        GameInfo {
            appid: app_state.appid.clone(),
            name: app_state.name.clone(),
            buildid: app_state.buildid.clone(),
            installdir: app_state.installdir.clone(),
            depots,
            install_path,
            state_flags,
            update_pending,
            target_build_id: app_state.target_build_id.clone(),
            bytes_to_download: app_state.bytes_to_download.clone(),
            size_on_disk: format_bytes(size_on_disk),
            last_updated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vdf::InstalledDepot;
    use std::collections::HashMap;

    fn make_app_state() -> AppState {
        let mut depots = HashMap::new();
        depots.insert(
            "3321461".to_string(),
            InstalledDepot {
                manifest: "7446650175280810671".to_string(),
                size: "133575233011".to_string(),
            },
        );
        AppState {
            appid: "3321460".to_string(),
            name: "Crimson Desert".to_string(),
            buildid: "22560074".to_string(),
            installdir: "Crimson Desert".to_string(),
            state_flags: "4".to_string(),
            installed_depots: depots,
            target_build_id: None,
            bytes_to_download: None,
            full_validate_after_next_update: None,
            last_updated: Some("1765887799".to_string()),
            last_played: Some("1765994178".to_string()),
            size_on_disk: Some("133575233011".to_string()),
        }
    }

    #[test]
    fn game_info_from_app_state_basic() {
        let app = make_app_state();
        let steamapps = Path::new("/home/user/.local/share/Steam/steamapps");
        let game = GameInfo::from_app_state(&app, steamapps);

        assert_eq!(game.appid, "3321460");
        assert_eq!(game.name, "Crimson Desert");
        assert_eq!(game.buildid, "22560074");
        assert_eq!(game.installdir, "Crimson Desert");
        assert_eq!(
            game.install_path,
            "/home/user/.local/share/Steam/steamapps/common/Crimson Desert"
        );
        assert_eq!(game.depots.len(), 1);
        assert_eq!(game.depots[0].depot_id, "3321461");
        assert_eq!(game.depots[0].manifest, "7446650175280810671");
        assert_eq!(game.depots[0].size, "133575233011");
        assert_eq!(game.state_flags, 4);
        assert!(!game.update_pending);
        assert!(game.target_build_id.is_none());
        assert_eq!(game.size_on_disk, "124.4 GB");
    }

    #[test]
    fn game_info_from_app_state_multiple_depots() {
        let mut app = make_app_state();
        app.installed_depots.insert(
            "3321462".to_string(),
            InstalledDepot {
                manifest: "9999999999".to_string(),
                size: "5000".to_string(),
            },
        );
        let steamapps = Path::new("/steamapps");
        let game = GameInfo::from_app_state(&app, steamapps);

        assert_eq!(game.depots.len(), 2);
        // Sorted by depot_id
        assert_eq!(game.depots[0].depot_id, "3321461");
        assert_eq!(game.depots[1].depot_id, "3321462");
    }

    #[test]
    fn game_info_from_app_state_empty_depots() {
        let mut app = make_app_state();
        app.installed_depots.clear();
        let steamapps = Path::new("/steamapps");
        let game = GameInfo::from_app_state(&app, steamapps);

        assert!(game.depots.is_empty());
    }

    #[test]
    fn game_info_serializes_to_json() {
        let app = make_app_state();
        let steamapps = Path::new("/steamapps");
        let game = GameInfo::from_app_state(&app, steamapps);
        let json = serde_json::to_string(&game).unwrap();

        assert!(json.contains("\"appid\":\"3321460\""));
        assert!(json.contains("\"name\":\"Crimson Desert\""));
    }
}
