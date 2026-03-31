use super::{map_get_map, map_get_str, VdfDocument, VdfError, VdfMap, VdfValue};
use std::collections::HashMap;

/// A strongly-typed representation of an ACF app manifest's AppState.
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub appid: String,
    pub name: String,
    pub buildid: String,
    pub installdir: String,
    pub state_flags: String,
    pub installed_depots: HashMap<String, InstalledDepot>,
    pub target_build_id: Option<String>,
    pub bytes_to_download: Option<String>,
    pub full_validate_after_next_update: Option<String>,
    pub last_updated: Option<String>,
    pub last_played: Option<String>,
    pub size_on_disk: Option<String>,
}

/// Parameters for patching an ACF manifest to prevent Steam from detecting
/// a version mismatch after a downgrade.
#[derive(Debug, Clone)]
pub struct AcfPatchParams {
    /// The latest build ID (from Steam servers, not the target version).
    pub latest_buildid: String,
    /// The latest manifest ID for the depot being downgraded.
    pub latest_manifest: String,
    /// The latest depot size value.
    pub latest_size: String,
    /// The depot ID being downgraded.
    pub depot_id: String,
}

/// A single installed depot entry.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledDepot {
    pub manifest: String,
    pub size: String,
}

impl AppState {
    /// Extract an AppState from a parsed VDF document.
    pub fn from_vdf(doc: &VdfDocument) -> Result<Self, VdfError> {
        if doc.key != "AppState" {
            return Err(VdfError::InvalidField {
                field: "root".into(),
                detail: format!("expected root key 'AppState', got '{}'", doc.key),
            });
        }

        let map = match &doc.value {
            VdfValue::Map(m) => m,
            _ => {
                return Err(VdfError::InvalidField {
                    field: "root".into(),
                    detail: "expected root value to be a map".into(),
                })
            }
        };

        let appid = require_str(map, "appid")?;
        let name = require_str(map, "name")?;
        let buildid = require_str(map, "buildid")?;
        let installdir = require_str(map, "installdir")?;
        let state_flags = require_str(map, "StateFlags")?;

        let target_build_id = map_get_str(map, "TargetBuildID").map(|s| s.to_string());
        let bytes_to_download = map_get_str(map, "BytesToDownload").map(|s| s.to_string());
        let full_validate_after_next_update =
            map_get_str(map, "FullValidateAfterNextUpdate").map(|s| s.to_string());
        let last_updated = map_get_str(map, "LastUpdated").map(|s| s.to_string());
        let last_played = map_get_str(map, "LastPlayed").map(|s| s.to_string());
        let size_on_disk = map_get_str(map, "SizeOnDisk").map(|s| s.to_string());

        let installed_depots = parse_installed_depots(map)?;

        Ok(AppState {
            appid,
            name,
            buildid,
            installdir,
            state_flags,
            installed_depots,
            target_build_id,
            bytes_to_download,
            full_validate_after_next_update,
            last_updated,
            last_played,
            size_on_disk,
        })
    }

    /// Patch this AppState for a downgrade operation.
    pub fn patch_for_downgrade(&mut self, params: &AcfPatchParams) {
        self.buildid = params.latest_buildid.clone();
        self.state_flags = "4".to_string();
        self.target_build_id = Some("0".to_string());
        self.bytes_to_download = Some("0".to_string());
        self.full_validate_after_next_update = Some("0".to_string());

        if let Some(depot) = self.installed_depots.get_mut(&params.depot_id) {
            depot.manifest = params.latest_manifest.clone();
            depot.size = params.latest_size.clone();
        }
    }

    /// Convert this AppState back into a VdfDocument for serialization.
    pub fn to_vdf(&self) -> VdfDocument {
        let mut map: VdfMap = vec![
            ("appid".into(), VdfValue::String(self.appid.clone())),
            ("name".into(), VdfValue::String(self.name.clone())),
            ("buildid".into(), VdfValue::String(self.buildid.clone())),
            ("installdir".into(), VdfValue::String(self.installdir.clone())),
            ("StateFlags".into(), VdfValue::String(self.state_flags.clone())),
        ];

        if let Some(ref target) = self.target_build_id {
            map.push(("TargetBuildID".into(), VdfValue::String(target.clone())));
        }

        if let Some(ref bytes) = self.bytes_to_download {
            map.push(("BytesToDownload".into(), VdfValue::String(bytes.clone())));
        }

        if let Some(ref val) = self.full_validate_after_next_update {
            map.push((
                "FullValidateAfterNextUpdate".into(),
                VdfValue::String(val.clone()),
            ));
        }

        if let Some(ref ts) = self.last_updated {
            map.push(("LastUpdated".into(), VdfValue::String(ts.clone())));
        }

        if let Some(ref ts) = self.last_played {
            map.push(("LastPlayed".into(), VdfValue::String(ts.clone())));
        }

        if let Some(ref size) = self.size_on_disk {
            map.push(("SizeOnDisk".into(), VdfValue::String(size.clone())));
        }

        // Build InstalledDepots map
        let mut depots: VdfMap = Vec::new();
        let mut depot_ids: Vec<&String> = self.installed_depots.keys().collect();
        depot_ids.sort();
        for depot_id in depot_ids {
            let depot = &self.installed_depots[depot_id];
            let depot_map: VdfMap = vec![
                ("manifest".into(), VdfValue::String(depot.manifest.clone())),
                ("size".into(), VdfValue::String(depot.size.clone())),
            ];
            depots.push((depot_id.clone(), VdfValue::Map(depot_map)));
        }
        map.push(("InstalledDepots".into(), VdfValue::Map(depots)));

        VdfDocument {
            key: "AppState".into(),
            value: VdfValue::Map(map),
        }
    }
}

fn require_str(map: &VdfMap, key: &str) -> Result<String, VdfError> {
    map_get_str(map, key)
        .map(|s| s.to_string())
        .ok_or_else(|| VdfError::MissingField(key.into()))
}

fn parse_installed_depots(map: &VdfMap) -> Result<HashMap<String, InstalledDepot>, VdfError> {
    let mut result = HashMap::new();

    let depots_map = match map_get_map(map, "InstalledDepots") {
        Some(m) => m,
        None => return Ok(result),
    };

    for (depot_id, depot_value) in depots_map {
        let depot_entries = match depot_value {
            VdfValue::Map(m) => m,
            _ => {
                return Err(VdfError::InvalidField {
                    field: format!("InstalledDepots.{}", depot_id),
                    detail: "expected depot entry to be a map".into(),
                })
            }
        };

        let manifest = require_str(depot_entries, "manifest").map_err(|_| {
            VdfError::MissingField(format!("InstalledDepots.{}.manifest", depot_id))
        })?;

        let size = require_str(depot_entries, "size").map_err(|_| {
            VdfError::MissingField(format!("InstalledDepots.{}.size", depot_id))
        })?;

        result.insert(depot_id.clone(), InstalledDepot { manifest, size });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn patch_for_downgrade_sets_buildid_to_latest() {
        let mut app = make_app_state();
        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        app.patch_for_downgrade(&params);

        assert_eq!(app.buildid, "99999999");
    }

    #[test]
    fn patch_for_downgrade_sets_manifest_and_size_to_latest() {
        let mut app = make_app_state();
        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        app.patch_for_downgrade(&params);

        let depot = app.installed_depots.get("3321461").unwrap();
        assert_eq!(depot.manifest, "8888888888888888888");
        assert_eq!(depot.size, "200000000000");
    }

    #[test]
    fn patch_for_downgrade_sets_state_flags_to_4() {
        let mut app = make_app_state();
        app.state_flags = "1".to_string();

        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        app.patch_for_downgrade(&params);

        assert_eq!(app.state_flags, "4");
    }

    #[test]
    fn patch_for_downgrade_clears_update_fields() {
        let mut app = make_app_state();
        app.target_build_id = Some("22570000".to_string());
        app.bytes_to_download = Some("5000000".to_string());
        app.full_validate_after_next_update = Some("1".to_string());

        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        app.patch_for_downgrade(&params);

        assert_eq!(app.target_build_id, Some("0".to_string()));
        assert_eq!(app.bytes_to_download, Some("0".to_string()));
        assert_eq!(app.full_validate_after_next_update, Some("0".to_string()));
    }

    #[test]
    fn patch_for_downgrade_ignores_missing_depot() {
        let mut app = make_app_state();
        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "9999999".to_string(),
        };

        app.patch_for_downgrade(&params);

        assert_eq!(app.buildid, "99999999");
        assert_eq!(app.state_flags, "4");
        let depot = app.installed_depots.get("3321461").unwrap();
        assert_eq!(depot.manifest, "7446650175280810671");
    }

    #[test]
    fn patch_for_downgrade_round_trips_through_vdf() {
        let mut app = make_app_state();
        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        app.patch_for_downgrade(&params);

        let doc = app.to_vdf();
        let serialized = super::super::serialize(&doc);
        let reparsed_doc = super::super::parse(&serialized).unwrap();
        let reparsed = AppState::from_vdf(&reparsed_doc).unwrap();

        assert_eq!(reparsed.buildid, "99999999");
        assert_eq!(reparsed.state_flags, "4");
        assert_eq!(reparsed.target_build_id, Some("0".to_string()));
        assert_eq!(reparsed.bytes_to_download, Some("0".to_string()));
        assert_eq!(
            reparsed.full_validate_after_next_update,
            Some("0".to_string())
        );
        let depot = reparsed.installed_depots.get("3321461").unwrap();
        assert_eq!(depot.manifest, "8888888888888888888");
        assert_eq!(depot.size, "200000000000");
    }
}
