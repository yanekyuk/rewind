use serde::Serialize;
use std::path::Path;

use super::vdf::AppState;

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

        GameInfo {
            appid: app_state.appid.clone(),
            name: app_state.name.clone(),
            buildid: app_state.buildid.clone(),
            installdir: app_state.installdir.clone(),
            depots,
            install_path,
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
