// rewind-core/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("could not determine data directory")]
    NoDataDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub steam_username: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Library {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GamesConfig {
    #[serde(default)]
    pub games: Vec<GameEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameEntry {
    pub name: String,
    pub app_id: u32,
    pub depot_id: u32,
    pub install_path: PathBuf,
    pub active_manifest_id: String,
    pub latest_manifest_id: String,
    #[serde(default)]
    pub cached_manifest_ids: Vec<String>,
    pub acf_locked: bool,
}

impl GameEntry {
    /// Returns the path to the appmanifest_*.acf file for this game.
    /// ACF is at <steamapps>/appmanifest_<app_id>.acf
    /// install_path is <steamapps>/common/<game_name>
    /// so we go up two levels from install_path.
    pub fn acf_path(&self) -> PathBuf {
        self.install_path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&self.install_path)
            .join(format!("appmanifest_{}.acf", self.app_id))
    }
}

/// Returns the rewind data directory, creating it if needed.
/// Linux/macOS: ~/.local/share/rewind   Windows: %APPDATA%\rewind
pub fn data_dir() -> Result<PathBuf, ConfigError> {
    let dir = dirs::data_dir()
        .map(|d| d.join("rewind"))
        .ok_or(ConfigError::NoDataDir)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn bin_dir() -> Result<PathBuf, ConfigError> {
    let dir = data_dir()?.join("bin");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cache_dir() -> Result<PathBuf, ConfigError> {
    let dir = data_dir()?.join("cache");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn load_config() -> Result<Config, ConfigError> {
    let path = data_dir()?.join("config.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_config(config: &Config) -> Result<(), ConfigError> {
    let path = data_dir()?.join("config.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn load_games() -> Result<GamesConfig, ConfigError> {
    let path = data_dir()?.join("games.toml");
    if !path.exists() {
        return Ok(GamesConfig::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_games(games: &GamesConfig) -> Result<(), ConfigError> {
    let path = data_dir()?.join("games.toml");
    let content = toml::to_string_pretty(games)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_roundtrip() {
        let config = Config {
            steam_username: Some("testuser".into()),
            libraries: vec![Library {
                path: "/tmp/steamapps".into(),
            }],
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.steam_username.as_deref(), Some("testuser"));
        assert_eq!(parsed.libraries.len(), 1);
    }

    #[test]
    fn games_config_roundtrip() {
        let games = GamesConfig {
            games: vec![GameEntry {
                name: "Crimson Desert".into(),
                app_id: 3321460,
                depot_id: 3321461,
                install_path: "/games/crimson-desert".into(),
                active_manifest_id: "abc123".into(),
                latest_manifest_id: "def456".into(),
                cached_manifest_ids: vec!["abc123".into(), "def456".into()],
                acf_locked: true,
            }],
        };
        let toml_str = toml::to_string_pretty(&games).unwrap();
        let parsed: GamesConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.games[0].name, "Crimson Desert");
        assert_eq!(parsed.games[0].app_id, 3321460);
        assert_eq!(parsed.games[0].cached_manifest_ids.len(), 2);
    }

    #[test]
    fn empty_config_is_default() {
        let parsed: Config = toml::from_str("").unwrap();
        assert!(parsed.steam_username.is_none());
        assert!(parsed.libraries.is_empty());
    }

    #[test]
    fn save_and_load_config() {
        let tmp = TempDir::new().unwrap();
        let config = Config {
            steam_username: Some("user1".into()),
            libraries: vec![],
        };
        let path = tmp.path().join("config.toml");
        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &content).unwrap();
        let loaded: Config = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.steam_username.as_deref(), Some("user1"));
    }

    #[test]
    fn acf_path_computed_correctly() {
        let entry = GameEntry {
            name: "Game".into(),
            app_id: 1234,
            depot_id: 5678,
            install_path: "/home/user/.steam/steamapps/common/Game".into(),
            active_manifest_id: "m1".into(),
            latest_manifest_id: "m1".into(),
            cached_manifest_ids: vec![],
            acf_locked: false,
        };
        let acf = entry.acf_path();
        assert!(acf.to_string_lossy().ends_with("appmanifest_1234.acf"));
        assert!(acf.to_string_lossy().contains("steamapps"));
    }
}
