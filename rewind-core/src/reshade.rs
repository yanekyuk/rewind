// rewind-core/src/reshade.rs
use crate::config::ReshadeApi;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum ReshadeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("extraction failed — could not locate 7z stream in installer")]
    ExtractionFailed,
    #[error("ReShade installer or DLL not found")]
    NotFound,
    #[error("symlink conflict: a real file already exists at {0}")]
    SymlinkConflict(String),
    #[error("sevenz error: {0}")]
    SevenZ(String),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

#[derive(Debug, Clone)]
pub enum ReshadeProgress {
    Line(String),
    Done,
    Error(String),
}

pub fn reshade_dll_path(bin_dir: &Path) -> PathBuf {
    bin_dir.join("ReShade64.dll")
}

pub fn reshade_shaders_cache_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("reshade-shaders")
}

impl ReshadeApi {
    pub fn dll_name(&self) -> &'static str {
        match self {
            ReshadeApi::Dxgi => "dxgi.dll",
            ReshadeApi::D3d9 => "d3d9.dll",
            ReshadeApi::OpenGl32 => "opengl32.dll",
            ReshadeApi::Vulkan1 => "vulkan-1.dll",
        }
    }

    /// Returns the WINEDLLOVERRIDES Steam launch command for this API (Linux/Proton).
    pub fn linux_launch_command(&self) -> String {
        // Strip ".dll" suffix for WINEDLLOVERRIDES key
        let key = self.dll_name().trim_end_matches(".dll");
        format!("WINEDLLOVERRIDES=\"{}=n,b\" %command%", key)
    }
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn enable_reshade(
    game_dir: &Path,
    api: &ReshadeApi,
    reshade_dll: &Path,
    shaders_src: Option<&Path>,
) -> Result<(), ReshadeError> {
    let dll_dest = game_dir.join(api.dll_name());

    // Conflict: real file exists (not a symlink)
    if dll_dest.exists() && !is_symlink(&dll_dest) {
        return Err(ReshadeError::SymlinkConflict(dll_dest.display().to_string()));
    }

    // Remove stale symlink if present
    if is_symlink(&dll_dest) {
        std::fs::remove_file(&dll_dest)?;
    }

    let dll_abs = std::fs::canonicalize(reshade_dll)?;

    #[cfg(unix)]
    std::os::unix::fs::symlink(&dll_abs, &dll_dest)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&dll_abs, &dll_dest)?;

    // Optionally symlink shaders directory
    if let Some(src) = shaders_src {
        let shaders_dest = game_dir.join("reshade-shaders");
        if is_symlink(&shaders_dest) {
            std::fs::remove_file(&shaders_dest)?;
        }
        if !shaders_dest.exists() {
            let src_abs = std::fs::canonicalize(src)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&src_abs, &shaders_dest)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&src_abs, &shaders_dest)?;
        }
    }

    Ok(())
}

pub fn disable_reshade(game_dir: &Path, api: &ReshadeApi) -> Result<(), ReshadeError> {
    let dll_dest = game_dir.join(api.dll_name());
    if is_symlink(&dll_dest) {
        std::fs::remove_file(&dll_dest)?;
    }

    let shaders_dest = game_dir.join("reshade-shaders");
    if is_symlink(&shaders_dest) {
        std::fs::remove_file(&shaders_dest)?;
    }

    Ok(())
}

/// Download the official ReShade installer from reshade.me, extract `ReShade64.dll`
/// from the embedded NSIS 7z stream, and write it to `bin_dir`.
///
/// Skips the download if `ReShade64.dll` already exists in `bin_dir`.
/// Progress lines are sent over `tx`; the channel is NOT closed here — the caller
/// decides when to send `ReshadeProgress::Done`.
pub async fn download_reshade(
    bin_dir: &Path,
    tx: mpsc::Sender<ReshadeProgress>,
) -> Result<PathBuf, ReshadeError> {
    let dest = reshade_dll_path(bin_dir);
    if dest.exists() {
        return Ok(dest);
    }

    std::fs::create_dir_all(bin_dir)?;

    let client = reqwest::Client::builder()
        .user_agent("rewind-cli/0.1")
        .build()?;

    let _ = tx.send(ReshadeProgress::Line("Fetching ReShade download URL...".into())).await;

    // Scrape reshade.me for the installer URL (pattern: /downloads/ReShade_Setup_X.Y.Z.exe)
    let page = client.get("https://reshade.me/").send().await?.text().await?;
    let installer_path = page
        .split('"')
        .find(|s| s.starts_with("/downloads/ReShade_Setup_") && s.ends_with(".exe"))
        .ok_or(ReshadeError::NotFound)?
        .to_string();
    let installer_url = format!("https://reshade.me{}", installer_path);

    let _ = tx.send(ReshadeProgress::Line(format!("Downloading {}...", installer_path))).await;

    let bytes = client.get(&installer_url).send().await?.bytes().await?;

    // NSIS installers embed a 7z-compressed payload. Locate it by 7z magic bytes.
    const SEVEN_Z_MAGIC: &[u8] = b"\x37\x7a\xbc\xaf\x27\x1c";
    let offset = bytes
        .windows(SEVEN_Z_MAGIC.len())
        .position(|w| w == SEVEN_Z_MAGIC)
        .ok_or(ReshadeError::ExtractionFailed)?;

    let _ = tx.send(ReshadeProgress::Line("Extracting ReShade64.dll...".into())).await;

    // Write the 7z stream to a temp file so sevenz-rust can open it
    let tmp_7z = std::env::temp_dir().join("rewind_reshade_setup.7z");
    std::fs::write(&tmp_7z, &bytes[offset..])?;

    let tmp_extract = std::env::temp_dir().join("rewind_reshade_extract");
    if tmp_extract.exists() {
        std::fs::remove_dir_all(&tmp_extract)?;
    }
    std::fs::create_dir_all(&tmp_extract)?;

    sevenz_rust::decompress_file(&tmp_7z, &tmp_extract)
        .map_err(|e| ReshadeError::SevenZ(e.to_string()))?;
    let _ = std::fs::remove_file(&tmp_7z);

    // Find ReShade64.dll in extracted output (may be at root or in a subdir)
    let dll_src = walkdir::WalkDir::new(&tmp_extract)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name() == std::ffi::OsStr::new("ReShade64.dll"))
        .map(|e| e.path().to_path_buf())
        .ok_or(ReshadeError::NotFound)?;

    std::fs::copy(&dll_src, &dest)?;
    let _ = std::fs::remove_dir_all(&tmp_extract);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_name_correct_for_each_api() {
        assert_eq!(ReshadeApi::Dxgi.dll_name(), "dxgi.dll");
        assert_eq!(ReshadeApi::D3d9.dll_name(), "d3d9.dll");
        assert_eq!(ReshadeApi::OpenGl32.dll_name(), "opengl32.dll");
        assert_eq!(ReshadeApi::Vulkan1.dll_name(), "vulkan-1.dll");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_launch_command_includes_dll_stem() {
        let cmd = ReshadeApi::Dxgi.linux_launch_command();
        assert!(cmd.contains("dxgi=n,b"));
        assert!(cmd.contains("%command%"));

        let cmd9 = ReshadeApi::D3d9.linux_launch_command();
        assert!(cmd9.contains("d3d9=n,b"));
    }

    #[test]
    fn reshade_dll_path_uses_bin_dir() {
        let p = reshade_dll_path(std::path::Path::new("/tmp/bin"));
        assert_eq!(p, std::path::Path::new("/tmp/bin/ReShade64.dll"));
    }

    #[test]
    fn reshade_shaders_cache_path_uses_cache_dir() {
        let p = reshade_shaders_cache_path(std::path::Path::new("/tmp/cache"));
        assert_eq!(p, std::path::Path::new("/tmp/cache/reshade-shaders"));
    }

    #[cfg(unix)]
    #[test]
    fn enable_reshade_creates_dll_symlink() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let game_dir = tmp.path().join("game");
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();

        // Create a fake ReShade64.dll in bin_dir
        let dll_path = bin_dir.join("ReShade64.dll");
        std::fs::write(&dll_path, b"fake dll").unwrap();

        enable_reshade(&game_dir, &ReshadeApi::Dxgi, &dll_path, None).unwrap();

        let link = game_dir.join("dxgi.dll");
        assert!(link.exists(), "symlink should exist");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn enable_reshade_symlink_conflict_returns_error() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let game_dir = tmp.path().join("game");
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();

        let dll_path = bin_dir.join("ReShade64.dll");
        std::fs::write(&dll_path, b"fake dll").unwrap();

        // Pre-existing real file in game dir
        std::fs::write(game_dir.join("dxgi.dll"), b"game's own dxgi").unwrap();

        let result = enable_reshade(&game_dir, &ReshadeApi::Dxgi, &dll_path, None);
        assert!(matches!(result, Err(ReshadeError::SymlinkConflict(_))));
    }

    #[cfg(unix)]
    #[test]
    fn disable_reshade_removes_dll_symlink() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let game_dir = tmp.path().join("game");
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();

        let dll_path = bin_dir.join("ReShade64.dll");
        std::fs::write(&dll_path, b"fake dll").unwrap();

        enable_reshade(&game_dir, &ReshadeApi::Dxgi, &dll_path, None).unwrap();
        disable_reshade(&game_dir, &ReshadeApi::Dxgi).unwrap();

        assert!(!game_dir.join("dxgi.dll").exists());
    }

    #[cfg(unix)]
    #[test]
    fn disable_reshade_is_noop_when_no_symlink() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let game_dir = tmp.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        // Should not error even if no symlink exists
        disable_reshade(&game_dir, &ReshadeApi::Dxgi).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn enable_reshade_also_symlinks_shaders_dir() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let game_dir = tmp.path().join("game");
        let bin_dir = tmp.path().join("bin");
        let shaders_cache = tmp.path().join("reshade-shaders");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&shaders_cache).unwrap();

        let dll_path = bin_dir.join("ReShade64.dll");
        std::fs::write(&dll_path, b"fake").unwrap();

        enable_reshade(&game_dir, &ReshadeApi::Dxgi, &dll_path, Some(&shaders_cache)).unwrap();

        let shaders_link = game_dir.join("reshade-shaders");
        assert!(shaders_link.exists());
        assert!(std::fs::symlink_metadata(&shaders_link).unwrap().file_type().is_symlink());
    }

    #[tokio::test]
    async fn download_reshade_skips_when_dll_exists() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().to_path_buf();
        // Pre-create the DLL to simulate an already-downloaded state
        std::fs::write(bin_dir.join("ReShade64.dll"), b"existing").unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let result = download_reshade(&bin_dir, tx).await.unwrap();

        assert_eq!(result, bin_dir.join("ReShade64.dll"));
        // No Line messages expected — returns immediately without sending anything
        assert!(rx.try_recv().is_err()); // channel empty
    }
}
