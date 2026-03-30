//! Steam installation detection and game scanning.
//!
//! This module provides cross-platform Steam path detection, library folder
//! discovery, and appmanifest scanning. It is the infrastructure layer's
//! implementation for locating installed Steam games.

use std::path::{Path, PathBuf};

use crate::domain::vdf::{self, AppState};
use crate::error::RewindError;

/// Returns the default `steamapps/` directory for the current platform.
///
/// Returns `None` if the home directory cannot be determined or if the
/// default Steam path does not exist on disk.
pub fn default_steamapps_path() -> Option<PathBuf> {
    let path = default_steamapps_path_unchecked()?;
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// Returns the default `steamapps/` directory for the current platform
/// without checking whether it exists. Useful for testing.
fn default_steamapps_path_unchecked() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::data_dir().map(|d| d.join("Steam").join("steamapps"))
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("Application Support")
                .join("Steam")
                .join("steamapps")
        })
    }

    #[cfg(target_os = "windows")]
    {
        Some(PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Parse `libraryfolders.vdf` to discover additional Steam library folders.
///
/// The file is typically at `<default_steamapps>/libraryfolders.vdf`.
/// Each entry in the VDF file has a `path` key pointing to the Steam library root.
/// We append `/steamapps` to each path to get the actual steamapps directory.
///
/// This is a pure function that takes the VDF content as a string.
pub fn parse_library_folders(content: &str) -> Result<Vec<PathBuf>, RewindError> {
    let doc = vdf::parse(content).map_err(|e| RewindError::Domain(e.to_string()))?;

    let entries = match &doc.value {
        vdf::VdfValue::Map(map) => map,
        _ => return Ok(Vec::new()),
    };

    let mut paths = Vec::new();

    for (_key, value) in entries {
        if let vdf::VdfValue::Map(entry_map) = value {
            if let Some(path_str) = vdf::map_get_str(entry_map, "path") {
                let steamapps = PathBuf::from(path_str).join("steamapps");
                paths.push(steamapps);
            }
        }
    }

    Ok(paths)
}

/// Discover all `steamapps/` directories on the system.
///
/// This checks:
/// 1. The default platform-specific Steam path
/// 2. Additional library folders from `libraryfolders.vdf`
///
/// Returns a deduplicated list of existing directories.
pub async fn discover_steamapps_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Default path
    if let Some(default) = default_steamapps_path() {
        dirs.push(default);
    }

    // 2. Additional library folders from libraryfolders.vdf
    if let Some(default) = default_steamapps_path() {
        let library_vdf = default.join("libraryfolders.vdf");
        if let Ok(content) = tokio::fs::read_to_string(&library_vdf).await {
            if let Ok(extra_dirs) = parse_library_folders(&content) {
                for dir in extra_dirs {
                    if dir.is_dir() && !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            }
        }
    }

    dirs
}

/// Parse a single appmanifest ACF file into an `AppState`.
pub fn parse_appmanifest(content: &str) -> Result<AppState, RewindError> {
    let doc = vdf::parse(content).map_err(|e| RewindError::Domain(e.to_string()))?;
    AppState::from_vdf(&doc).map_err(|e| RewindError::Domain(e.to_string()))
}

/// Scan a single `steamapps/` directory for appmanifest files and parse them.
///
/// Malformed files are skipped with a warning logged to stderr.
/// Returns a list of `(AppState, steamapps_path)` tuples.
pub async fn scan_appmanifests(
    steamapps_dir: &Path,
) -> Result<Vec<(AppState, PathBuf)>, RewindError> {
    let mut results = Vec::new();

    let mut entries = tokio::fs::read_dir(steamapps_dir).await.map_err(|e| {
        RewindError::Infrastructure(format!(
            "failed to read directory {}: {}",
            steamapps_dir.display(),
            e
        ))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        RewindError::Infrastructure(format!(
            "failed to read entry in {}: {}",
            steamapps_dir.display(),
            e
        ))
    })? {
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => match parse_appmanifest(&content) {
                    Ok(app_state) => {
                        results.push((app_state, steamapps_dir.to_path_buf()));
                    }
                    Err(e) => {
                        eprintln!("Warning: skipping {}: {}", path.display(), e);
                    }
                },
                Err(e) => {
                    eprintln!("Warning: could not read {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_steamapps_path_unchecked_returns_some() {
        // On any supported platform, the unchecked path should be computable
        let path = default_steamapps_path_unchecked();
        assert!(path.is_some(), "should return a path on supported platforms");
        let path = path.unwrap();
        assert!(
            path.to_string_lossy().contains("steamapps"),
            "path should contain 'steamapps'"
        );
    }

    #[test]
    fn parse_library_folders_basic() {
        let content = r#""libraryfolders"
{
    "0"
    {
        "path"    "/home/user/.local/share/Steam"
        "label"   ""
    }
    "1"
    {
        "path"    "/mnt/games/SteamLibrary"
        "label"   ""
    }
}"#;
        let paths = parse_library_folders(content).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(
            paths[0],
            PathBuf::from("/home/user/.local/share/Steam/steamapps")
        );
        assert_eq!(
            paths[1],
            PathBuf::from("/mnt/games/SteamLibrary/steamapps")
        );
    }

    #[test]
    fn parse_library_folders_empty() {
        let content = r#""libraryfolders"
{
}"#;
        let paths = parse_library_folders(content).unwrap();
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_library_folders_invalid_vdf() {
        let content = "this is not valid vdf";
        let result = parse_library_folders(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_appmanifest_valid() {
        let content = r#""AppState"
{
    "appid"        "3321460"
    "name"         "Crimson Desert"
    "buildid"      "22560074"
    "installdir"   "Crimson Desert"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}"#;
        let app = parse_appmanifest(content).unwrap();
        assert_eq!(app.appid, "3321460");
        assert_eq!(app.name, "Crimson Desert");
    }

    #[test]
    fn parse_appmanifest_invalid() {
        let result = parse_appmanifest("not valid");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_appmanifests_with_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let steamapps = dir.path();

        // Write a valid appmanifest
        let acf = r#""AppState"
{
    "appid"        "12345"
    "name"         "Test Game"
    "buildid"      "100"
    "installdir"   "TestGame"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "12346"
        {
            "manifest"  "999999"
            "size"      "1000"
        }
    }
}"#;
        tokio::fs::write(steamapps.join("appmanifest_12345.acf"), acf)
            .await
            .unwrap();

        // Write an invalid appmanifest (should be skipped)
        tokio::fs::write(steamapps.join("appmanifest_99999.acf"), "invalid data")
            .await
            .unwrap();

        // Write a non-appmanifest file (should be ignored)
        tokio::fs::write(steamapps.join("libraryfolders.vdf"), "ignored")
            .await
            .unwrap();

        let results = scan_appmanifests(steamapps).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.appid, "12345");
        assert_eq!(results[0].0.name, "Test Game");
        assert_eq!(results[0].1, steamapps);
    }

    #[tokio::test]
    async fn scan_appmanifests_nonexistent_dir() {
        let result = scan_appmanifests(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }
}
