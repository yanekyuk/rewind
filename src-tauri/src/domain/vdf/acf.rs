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
}

/// A single installed depot entry.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledDepot {
    pub manifest: String,
    pub size: String,
}

impl AppState {
    /// Extract an AppState from a parsed VDF document.
    ///
    /// The document must have root key "AppState" with a map value containing
    /// the required fields.
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
        })
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

        // Build InstalledDepots map
        let mut depots: VdfMap = Vec::new();
        // Sort depot IDs for deterministic output
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

/// Get a required string field from a VdfMap, returning MissingField error if absent.
fn require_str(map: &VdfMap, key: &str) -> Result<String, VdfError> {
    map_get_str(map, key)
        .map(|s| s.to_string())
        .ok_or_else(|| VdfError::MissingField(key.into()))
}

/// Parse the InstalledDepots section from an AppState map.
fn parse_installed_depots(map: &VdfMap) -> Result<HashMap<String, InstalledDepot>, VdfError> {
    let mut result = HashMap::new();

    let depots_map = match map_get_map(map, "InstalledDepots") {
        Some(m) => m,
        None => return Ok(result), // InstalledDepots is optional (could be empty)
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
