use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("acf parse error in {path}: {msg}")]
    AcfParse { path: PathBuf, msg: String },
    #[error("steam not found")]
    SteamNotFound,
}

/// A game found in a Steam library.
#[derive(Debug, Clone)]
pub struct InstalledGame {
    pub app_id: u32,
    pub name: String,
    pub depot_id: u32,
    pub manifest_id: String,
    pub install_path: PathBuf,
    pub acf_path: PathBuf,
    pub state_flags: u32,
}

/// Scan one Steam library directory (the steamapps folder) and return all installed games.
pub fn scan_library(steamapps_dir: &Path) -> Result<Vec<InstalledGame>, ScannerError> {
    let mut games = Vec::new();

    let entries = std::fs::read_dir(steamapps_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with("appmanifest_") && name.ends_with(".acf") {
            match parse_acf(&path) {
                Ok(Some(game)) => games.push(game),
                Ok(None) => {}
                Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
            }
        }
    }

    Ok(games)
}

/// Detect all Steam library paths on this machine using steamlocate.
pub fn find_steam_libraries() -> Result<Vec<PathBuf>, ScannerError> {
    use steamlocate::SteamDir;

    let steam_dir = SteamDir::locate().map_err(|_| ScannerError::SteamNotFound)?;
    let mut paths = Vec::new();

    let libraries = steam_dir
        .libraries()
        .map_err(|_| ScannerError::SteamNotFound)?;
    for lib in libraries {
        if let Ok(lib) = lib {
            paths.push(lib.path().to_path_buf());
        }
    }

    Ok(paths)
}

/// Scan all detected Steam libraries and return every installed game.
pub fn scan_all_libraries() -> Result<Vec<InstalledGame>, ScannerError> {
    let libs = find_steam_libraries()?;
    let mut all = Vec::new();
    for lib in libs {
        let steamapps = lib.join("steamapps");
        if steamapps.exists() {
            all.extend(scan_library(&steamapps)?);
        }
    }
    Ok(all)
}

/// Read just the StateFlags field from an ACF file.
pub fn read_acf_state_flags(path: &Path) -> Result<u32, ScannerError> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_str_field(&content, "StateFlags")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0))
}

/// Read just the buildid field from an ACF file.
pub fn read_acf_buildid(path: &Path) -> Result<String, ScannerError> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_str_field(&content, "buildid")
        .unwrap_or("0")
        .to_string())
}

fn parse_acf(path: &Path) -> Result<Option<InstalledGame>, ScannerError> {
    let content = std::fs::read_to_string(path)?;

    let app_id = extract_str_field(&content, "appid")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| ScannerError::AcfParse {
            path: path.to_path_buf(),
            msg: "missing appid".into(),
        })?;

    let name = extract_str_field(&content, "name")
        .unwrap_or_default()
        .to_string();

    let state_flags = extract_str_field(&content, "StateFlags")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    // StateFlags 4 = fully installed. Skip games being downloaded/updated.
    if state_flags & 4 == 0 {
        return Ok(None);
    }

    let installdir = extract_str_field(&content, "installdir").unwrap_or_default();
    let steamapps = path.parent().unwrap_or(Path::new("."));
    let install_path = steamapps.join("common").join(installdir);

    let (depot_id, manifest_id) =
        extract_first_depot(&content).ok_or_else(|| ScannerError::AcfParse {
            path: path.to_path_buf(),
            msg: "no InstalledDepots found".into(),
        })?;

    Ok(Some(InstalledGame {
        app_id,
        name,
        depot_id,
        manifest_id,
        install_path,
        acf_path: path.to_path_buf(),
        state_flags,
    }))
}

/// Extract a top-level string field value from ACF text.
fn extract_str_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let search = format!("\"{}\"", key);
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&search) {
            let rest = &trimmed[search.len()..];
            if let Some(val) = extract_quoted(rest) {
                return Some(val);
            }
        }
    }
    None
}

/// Extract the first depot id + manifest from InstalledDepots block.
fn extract_first_depot(content: &str) -> Option<(u32, String)> {
    let mut in_installed_depots = false;
    let mut depot_id: Option<u32> = None;
    let mut depth = 0i32;
    let mut depots_depth = -1i32;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "{" {
            depth += 1;
            if in_installed_depots && depots_depth < 0 {
                depots_depth = depth;
            }
        }
        if trimmed == "}" {
            depth -= 1;
            if in_installed_depots && depth < depots_depth {
                break;
            }
        }

        if trimmed == "\"InstalledDepots\"" {
            in_installed_depots = true;
            continue;
        }

        if in_installed_depots && depot_id.is_none() {
            if let Some(id_str) = extract_quoted_only(trimmed) {
                if let Ok(id) = id_str.parse::<u32>() {
                    depot_id = Some(id);
                }
            }
        }

        if in_installed_depots && depot_id.is_some() {
            if trimmed.starts_with("\"manifest\"") {
                let rest = &trimmed["\"manifest\"".len()..];
                if let Some(manifest) = extract_quoted(rest) {
                    return Some((depot_id.unwrap(), manifest.to_string()));
                }
            }
        }
    }
    None
}

fn extract_quoted(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.starts_with('"') {
        let inner = &s[1..];
        inner.find('"').map(|end| &inner[..end])
    } else {
        None
    }
}

fn extract_quoted_only(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

fn extract_launch_options_from_vdf(content: &str, app_id: u32) -> Option<String> {
    let app_id_key = format!("\"{}\"", app_id);
    let mut in_apps = false;
    let mut in_target_app = false;
    let mut depth = 0i32;
    let mut apps_depth = -1i32;
    let mut app_depth = -1i32;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            depth += 1;
            if in_apps && !in_target_app && apps_depth < 0 {
                apps_depth = depth;
            }
            if in_target_app && app_depth < 0 {
                app_depth = depth;
            }
            continue;
        }

        if trimmed == "}" {
            depth -= 1;
            if in_target_app && depth < app_depth {
                in_target_app = false;
                app_depth = -1;
            }
            if in_apps && !in_target_app && depth < apps_depth {
                in_apps = false;
                apps_depth = -1;
            }
            continue;
        }

        if !in_apps && trimmed == "\"apps\"" {
            in_apps = true;
            continue;
        }

        if in_apps && !in_target_app && trimmed == app_id_key {
            in_target_app = true;
            continue;
        }

        if in_target_app && trimmed.starts_with("\"LaunchOptions\"") {
            let rest = &trimmed["\"LaunchOptions\"".len()..];
            if let Some(val) = extract_quoted(rest) {
                return if val.is_empty() { None } else { Some(val.to_string()) };
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_acf(dir: &Path, app_id: u32, name: &str, manifest_id: &str, depot_id: u32) {
        let content = format!(
            r#""AppState"
{{
	"appid"		"{app_id}"
	"Universe"	"1"
	"name"		"{name}"
	"StateFlags"	"4"
	"installdir"	"{name}"
	"LastUpdated"	"1700000000"
	"buildid"	"99999"
	"InstalledDepots"
	{{
		"{depot_id}"
		{{
			"manifest"	"{manifest_id}"
			"size"		"1000000"
		}}
	}}
}}"#
        );
        let acf_path = dir.join(format!("appmanifest_{app_id}.acf"));
        fs::write(acf_path, content).unwrap();
    }

    #[test]
    fn parse_acf_returns_installed_game() {
        let tmp = TempDir::new().unwrap();
        write_acf(tmp.path(), 3321460, "Crimson Desert", "abc123", 3321461);

        let games = scan_library(tmp.path()).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_id, 3321460);
        assert_eq!(games[0].name, "Crimson Desert");
        assert_eq!(games[0].manifest_id, "abc123");
        assert_eq!(games[0].depot_id, 3321461);
    }

    #[test]
    fn scan_library_finds_multiple_games() {
        let tmp = TempDir::new().unwrap();
        write_acf(tmp.path(), 1, "Game One", "m1", 2);
        write_acf(tmp.path(), 3, "Game Two", "m2", 4);

        let games = scan_library(tmp.path()).unwrap();
        assert_eq!(games.len(), 2);
    }

    #[test]
    fn empty_library_returns_empty_vec() {
        let tmp = TempDir::new().unwrap();
        let games = scan_library(tmp.path()).unwrap();
        assert!(games.is_empty());
    }

    #[test]
    fn skips_games_not_fully_installed() {
        let tmp = TempDir::new().unwrap();
        // StateFlags "2" = downloading (not fully installed)
        let content = r#""AppState"
{
	"appid"		"999"
	"name"		"Partial Game"
	"StateFlags"	"2"
	"installdir"	"PartialGame"
	"InstalledDepots"
	{
		"1000"
		{
			"manifest"	"xyz"
		}
	}
}"#;
        fs::write(tmp.path().join("appmanifest_999.acf"), content).unwrap();
        let games = scan_library(tmp.path()).unwrap();
        assert!(games.is_empty());
    }

    #[test]
    fn extract_launch_options_finds_value() {
        let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LaunchOptions"		"-novid %command%"
                    }
                }
            }
        }
    }
}"#;
        assert_eq!(
            extract_launch_options_from_vdf(vdf, 12345),
            Some("-novid %command%".to_string())
        );
    }

    #[test]
    fn extract_launch_options_returns_none_for_missing_app() {
        let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "99999"
                    {
                        "LaunchOptions"		"-novid %command%"
                    }
                }
            }
        }
    }
}"#;
        assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
    }

    #[test]
    fn extract_launch_options_returns_none_for_empty_value() {
        let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LaunchOptions"		""
                    }
                }
            }
        }
    }
}"#;
        assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
    }

    #[test]
    fn extract_launch_options_returns_none_for_absent_key() {
        let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LastPlayed"		"1234567890"
                    }
                }
            }
        }
    }
}"#;
        assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
    }
}
