use std::path::{Path, PathBuf};

const STEAM_CDN_BASE: &str = "https://cdn.akamai.steamstatic.com/steam/apps";

/// Returns the expected cache file path for a game's hero image.
pub fn hero_cache_path(cache_dir: &Path, app_id: u32) -> PathBuf {
    cache_dir.join(format!("{}_hero.jpg", app_id))
}

/// Returns the expected cache file path for a game's logo image.
pub fn logo_cache_path(cache_dir: &Path, app_id: u32) -> PathBuf {
    cache_dir.join(format!("{}_logo.png", app_id))
}

/// Returns the expected cache file path for the composited image (hero + logo).
pub fn composited_cache_path(cache_dir: &Path, app_id: u32) -> PathBuf {
    cache_dir.join(format!("{}_composited.png", app_id))
}

/// Returns the Steam CDN URL for a game's library hero image.
pub fn hero_url(app_id: u32) -> String {
    format!("{}/{}/library_hero.jpg", STEAM_CDN_BASE, app_id)
}

/// Returns the Steam CDN URL for a game's logo image.
pub fn logo_url(app_id: u32) -> String {
    format!("{}/{}/logo.png", STEAM_CDN_BASE, app_id)
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

/// Loads the cached composited image (hero + logo overlay) if present.
pub fn load_cached_composited(cache_dir: &Path, app_id: u32) -> Option<Vec<u8>> {
    let path = composited_cache_path(cache_dir, app_id);
    std::fs::read(&path).ok()
}

/// Fetches an image from a URL and returns the raw bytes.
async fn fetch_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()).into());
    }
    Ok(response.bytes().await?.to_vec())
}

/// Fetches the hero image from Steam CDN and saves it to the cache directory.
/// Returns the raw image bytes on success.
pub async fn fetch_and_cache_hero(
    cache_dir: &Path,
    app_id: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = fetch_image(&hero_url(app_id)).await?;
    let path = hero_cache_path(cache_dir, app_id);
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(&path, &bytes)?;
    Ok(bytes)
}

/// Fetches hero and logo from Steam CDN, composites the logo onto the hero
/// (bottom-left), caches all three files, and returns the composited image bytes.
/// If the logo fetch fails, returns the hero image alone.
pub async fn fetch_and_composite(
    cache_dir: &Path,
    app_id: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use image::{GenericImageView, ImageFormat};

    std::fs::create_dir_all(cache_dir)?;

    // Fetch hero (required)
    let hero_bytes = match load_cached_hero(cache_dir, app_id) {
        Some(b) => b,
        None => fetch_and_cache_hero(cache_dir, app_id).await?,
    };

    // Fetch logo (optional — if it fails, just return hero)
    let logo_bytes = match std::fs::read(logo_cache_path(cache_dir, app_id)).ok() {
        Some(b) => Some(b),
        None => {
            match fetch_image(&logo_url(app_id)).await {
                Ok(b) => {
                    let _ = std::fs::write(logo_cache_path(cache_dir, app_id), &b);
                    Some(b)
                }
                Err(_) => None,
            }
        }
    };

    // Load hero as mutable image
    let mut hero = image::load_from_memory(&hero_bytes)?;

    // Overlay logo if available
    if let Some(logo_bytes) = logo_bytes {
        if let Ok(logo) = image::load_from_memory(&logo_bytes) {
            let (hero_w, hero_h) = hero.dimensions();
            let (logo_w, logo_h) = logo.dimensions();

            // Scale logo to fit within ~40% of hero width, maintaining aspect ratio
            let max_logo_w = hero_w * 40 / 100;
            let scale = if logo_w > max_logo_w {
                max_logo_w as f64 / logo_w as f64
            } else {
                1.0
            };
            let new_w = (logo_w as f64 * scale) as u32;
            let new_h = (logo_h as f64 * scale) as u32;

            let resized_logo = image::imageops::resize(
                &logo,
                new_w,
                new_h,
                image::imageops::FilterType::Lanczos3,
            );

            // Position: bottom-left with some padding
            let x = hero_w * 5 / 100; // 5% from left
            let y = hero_h.saturating_sub(new_h).saturating_sub(hero_h * 5 / 100); // 5% from bottom

            image::imageops::overlay(&mut hero, &resized_logo, x as i64, y as i64);
        }
    }

    // Save composited result
    let comp_path = composited_cache_path(cache_dir, app_id);
    let mut buf = std::io::Cursor::new(Vec::new());
    hero.write_to(&mut buf, ImageFormat::Png)?;
    let comp_bytes = buf.into_inner();
    std::fs::write(&comp_path, &comp_bytes)?;

    Ok(comp_bytes)
}
