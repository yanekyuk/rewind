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

/// A Steam account found in loginusers.vdf.
#[derive(Debug, Clone)]
pub struct SteamAccount {
    pub id: u64,               // SteamID64
    pub persona_name: String,  // Display name (e.g. "yanekeke")
    pub account_name: String,  // Login name (e.g. "yanekyuk")
}

/// Read all Steam accounts from `<steam_root>/config/loginusers.vdf`.
/// Returns an empty Vec if the file is missing or unparseable.
pub fn read_steam_accounts(steam_root: &Path) -> Vec<SteamAccount> {
    let path = steam_root.join("config").join("loginusers.vdf");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    parse_loginusers_vdf(&content)
}

fn parse_loginusers_vdf(content: &str) -> Vec<SteamAccount> {
    let mut accounts = Vec::new();
    let mut current_id: Option<u64> = None;
    let mut current_persona: Option<String> = None;
    let mut current_account: Option<String> = None;
    let mut depth = 0i32;
    let mut in_users = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            depth += 1;
            continue;
        }

        if trimmed == "}" {
            // Closing a user entry block (depth 2 → 1)
            if depth == 2 && in_users {
                if let (Some(id), Some(persona), Some(account)) = (
                    current_id.take(),
                    current_persona.take(),
                    current_account.take(),
                ) {
                    accounts.push(SteamAccount { id, persona_name: persona, account_name: account });
                }
            }
            depth -= 1;
            continue;
        }

        if depth == 0 && trimmed == "\"users\"" {
            in_users = true;
            continue;
        }

        // SteamID64 key at depth 1 inside "users"
        if in_users && depth == 1 {
            if let Some(id_str) = extract_quoted_only(trimmed) {
                if let Ok(id) = id_str.parse::<u64>() {
                    current_id = Some(id);
                }
            }
            continue;
        }

        // Fields inside a user block at depth 2
        if in_users && depth == 2 {
            if trimmed.starts_with("\"AccountName\"") {
                let rest = trimmed["\"AccountName\"".len()..].trim();
                if let Some(val) = extract_quoted_only(rest) {
                    current_account = Some(val.to_string());
                }
            } else if trimmed.starts_with("\"PersonaName\"") {
                let rest = trimmed["\"PersonaName\"".len()..].trim();
                if let Some(val) = extract_quoted_only(rest) {
                    current_persona = Some(val.to_string());
                }
            }
        }
    }

    accounts
}

const STEAM_ID64_BASE: u64 = 76561197960265728;

/// Convert a SteamID64 to its userdata directory path under `<steam_root>/userdata/<32-bit-id>/`.
/// Returns `None` if the directory does not exist or the ID is below the base constant.
pub fn userdata_dir_for_account(steam_root: &Path, steam_id64: u64) -> Option<PathBuf> {
    let account_id = u32::try_from(steam_id64.checked_sub(STEAM_ID64_BASE)?).ok()?;
    let dir = steam_root.join("userdata").join(account_id.to_string());
    if dir.is_dir() { Some(dir) } else { None }
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

/// Like `extract_quoted` but handles VDF escape sequences (`\"` → `"`, `\\` → `\`).
/// Returns an owned String since the value may be transformed.
fn extract_quoted_escaped(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('"') {
        return None;
    }
    let mut result = String::new();
    let mut chars = s[1..].chars();
    loop {
        match chars.next()? {
            '"' => return Some(result),
            '\\' => match chars.next()? {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                c => { result.push('\\'); result.push(c); }
            },
            c => result.push(c),
        }
    }
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

/// Read launch options for a game from the most recently modified localconfig.vdf
/// found under `steam_root/userdata/*/config/localconfig.vdf`.
/// Returns `None` if not found, Steam root has no userdata, or options are empty.
pub fn read_launch_options(steam_root: &Path, app_id: u32) -> Option<String> {
    let userdata = steam_root.join("userdata");
    let entries = std::fs::read_dir(&userdata).ok()?;

    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let vdf_path = entry.path().join("config").join("localconfig.vdf");
        if vdf_path.exists() {
            if let Ok(meta) = std::fs::metadata(&vdf_path) {
                if let Ok(mtime) = meta.modified() {
                    if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                        best = Some((mtime, vdf_path));
                    }
                }
            }
        }
    }

    let (_, vdf_path) = best?;
    let content = std::fs::read_to_string(&vdf_path).ok()?;
    extract_launch_options_from_vdf(&content, app_id)
}

/// Convenience wrapper: resolves Steam root via steamlocate, then calls `read_launch_options`.
/// Returns `None` if Steam is not found or the game has no launch options set.
pub fn find_launch_options(app_id: u32) -> Option<String> {
    use steamlocate::SteamDir;
    let steam_dir = SteamDir::locate().ok()?;
    read_launch_options(steam_dir.path(), app_id)
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
                return None;
            }
            continue;
        }

        if !in_apps && trimmed == "\"apps\"" && depth == 4 {
            in_apps = true;
            continue;
        }

        if in_apps && !in_target_app && trimmed == app_id_key {
            in_target_app = true;
            continue;
        }

        if in_target_app && trimmed.starts_with("\"LaunchOptions\"") {
            let rest = &trimmed["\"LaunchOptions\"".len()..];
            if let Some(val) = extract_quoted_escaped(rest) {
                return if val.is_empty() { None } else { Some(val) };
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

    #[test]
    fn extract_launch_options_ignores_apps_at_wrong_depth() {
        // "apps" appearing at depth != 4 must not be matched
        let vdf = r#""UserLocalConfigStore"
{
    "apps"
    {
        "12345"
        {
            "LaunchOptions"		"wrong"
        }
    }
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
                        "LaunchOptions"		"correct"
                    }
                }
            }
        }
    }
}"#;
        assert_eq!(
            extract_launch_options_from_vdf(vdf, 12345),
            Some("correct".to_string())
        );
    }

    #[test]
    fn read_launch_options_finds_value_in_userdata() {
        let tmp = TempDir::new().unwrap();
        let user_dir = tmp.path().join("userdata").join("123456").join("config");
        fs::create_dir_all(&user_dir).unwrap();

        let vdf_content = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "42"
                    {
                        "LaunchOptions"		"DXVK_ASYNC=1 %command%"
                    }
                }
            }
        }
    }
}"#;
        fs::write(user_dir.join("localconfig.vdf"), vdf_content).unwrap();

        assert_eq!(
            read_launch_options(tmp.path(), 42),
            Some("DXVK_ASYNC=1 %command%".to_string())
        );
    }

    #[test]
    fn read_launch_options_returns_none_when_no_userdata() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(read_launch_options(tmp.path(), 42), None);
    }

    #[test]
    fn read_launch_options_returns_none_when_app_not_found() {
        let tmp = TempDir::new().unwrap();
        let user_dir = tmp.path().join("userdata").join("123456").join("config");
        fs::create_dir_all(&user_dir).unwrap();

        let vdf_content = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "99"
                    {
                        "LaunchOptions"		"-novid"
                    }
                }
            }
        }
    }
}"#;
        fs::write(user_dir.join("localconfig.vdf"), vdf_content).unwrap();

        assert_eq!(read_launch_options(tmp.path(), 42), None);
    }

    #[test]
    fn read_steam_accounts_parses_loginusers_vdf() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let vdf = "\"users\"\n{\n\t\"76561198858787719\"\n\t{\n\t\t\"AccountName\"\t\t\"yanekyuk\"\n\t\t\"PersonaName\"\t\t\"yanekeke\"\n\t}\n\t\"76561199258820835\"\n\t{\n\t\t\"AccountName\"\t\t\"chwantt\"\n\t\t\"PersonaName\"\t\t\"chwantt\"\n\t}\n}";
        fs::write(config_dir.join("loginusers.vdf"), vdf).unwrap();
        let accounts = read_steam_accounts(tmp.path());
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].id, 76561198858787719u64);
        assert_eq!(accounts[0].account_name, "yanekyuk");
        assert_eq!(accounts[0].persona_name, "yanekeke");
        assert_eq!(accounts[1].id, 76561199258820835u64);
    }

    #[test]
    fn read_steam_accounts_returns_empty_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(read_steam_accounts(tmp.path()).is_empty());
    }

    #[test]
    fn userdata_dir_for_account_returns_path_when_exists() {
        let tmp = TempDir::new().unwrap();
        // SteamID64 76561197960265729 → account ID = 76561197960265729 - 76561197960265728 = 1
        let account_dir = tmp.path().join("userdata").join("1");
        fs::create_dir_all(&account_dir).unwrap();
        let result = userdata_dir_for_account(tmp.path(), 76561197960265729u64);
        assert_eq!(result, Some(account_dir));
    }

    #[test]
    fn userdata_dir_for_account_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("userdata")).unwrap();
        let result = userdata_dir_for_account(tmp.path(), 76561197960265729u64);
        assert!(result.is_none());
    }

    #[test]
    fn userdata_dir_for_account_returns_none_on_underflow() {
        let tmp = TempDir::new().unwrap();
        let result = userdata_dir_for_account(tmp.path(), 0u64);
        assert!(result.is_none());
    }
}
