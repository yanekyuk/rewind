use std::path::{Path, PathBuf};

const STEAM_CDN_BASE: &str = "https://cdn.akamai.steamstatic.com/steam/apps";

/// Returns the expected cache file path for a game's hero image.
pub fn hero_cache_path(cache_dir: &Path, app_id: u32) -> PathBuf {
    cache_dir.join(format!("{}_hero.jpg", app_id))
}

/// Returns the Steam CDN URL for a game's library hero image.
pub fn hero_url(app_id: u32) -> String {
    format!("{}/{}/library_hero.jpg", STEAM_CDN_BASE, app_id)
}

/// Returns the image directory inside the rewind data dir, creating it if needed.
pub fn images_dir() -> Result<PathBuf, crate::config::ConfigError> {
    let dir = crate::config::data_dir()?.join("images");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Loads a cached hero image from disk, returning the raw bytes if present.
pub fn load_cached_hero(cache_dir: &Path, app_id: u32) -> Option<Vec<u8>> {
    let path = hero_cache_path(cache_dir, app_id);
    std::fs::read(&path).ok()
}

/// Fetches the hero image from Steam CDN and saves it to the cache directory.
/// Returns the raw image bytes on success.
pub async fn fetch_and_cache_hero(
    cache_dir: &Path,
    app_id: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let url = hero_url(app_id);
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()).into());
    }

    let bytes = response.bytes().await?.to_vec();
    let path = hero_cache_path(cache_dir, app_id);
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(&path, &bytes)?;
    Ok(bytes)
}
