use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path strip error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),
}

/// Returns the cache directory for a specific manifest version.
/// Structure: `<cache_root>/<app_id>/<depot_id>/<manifest_id>/`
pub fn manifest_cache_dir(
    cache_root: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
) -> PathBuf {
    cache_root
        .join(app_id.to_string())
        .join(depot_id.to_string())
        .join(manifest_id)
}

/// After DepotDownloader has populated `target_cache_dir` with downloaded (changed) files:
/// 1. For each file in target_cache_dir, copy the current game dir version → current_cache_dir
/// 2. Replace the game dir file with a symlink → target_cache_dir file
pub fn apply_downloaded(
    game_dir: &Path,
    target_cache_dir: &Path,
    current_cache_dir: &Path,
) -> Result<(), CacheError> {
    std::fs::create_dir_all(current_cache_dir)?;

    for entry in WalkDir::new(target_cache_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel_path = entry.path().strip_prefix(target_cache_dir)?;
        let game_file = game_dir.join(rel_path);
        let current_backup = current_cache_dir.join(rel_path);

        if let Some(parent) = current_backup.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Backup current game file → current_cache
        if game_file.exists() {
            std::fs::copy(&game_file, &current_backup)?;
        }

        // Replace game file with symlink → target cache (use absolute path)
        let target_abs = std::fs::canonicalize(entry.path())?;
        if game_file.exists() || is_symlink(&game_file) {
            remove_file_or_symlink(&game_file)?;
        }
        if let Some(parent) = game_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        create_symlink(&target_abs, &game_file)?;
    }

    Ok(())
}

/// Repoint existing symlinks in game_dir to point to new_cache_dir.
/// Only repoints files that exist in new_cache_dir.
pub fn repoint_symlinks(game_dir: &Path, new_cache_dir: &Path) -> Result<(), CacheError> {
    for entry in WalkDir::new(new_cache_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel_path = entry.path().strip_prefix(new_cache_dir)?;
        let game_file = game_dir.join(rel_path);
        let new_target = std::fs::canonicalize(entry.path())?;

        if game_file.exists() || is_symlink(&game_file) {
            remove_file_or_symlink(&game_file)?;
        }
        if let Some(parent) = game_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        create_symlink(&new_target, &game_file)?;
    }
    Ok(())
}

/// Restore original files: remove symlinks in game_dir and replace with files from backup_cache_dir.
pub fn restore_from_cache(game_dir: &Path, backup_cache_dir: &Path) -> Result<(), CacheError> {
    for entry in WalkDir::new(backup_cache_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel_path = entry.path().strip_prefix(backup_cache_dir)?;
        let game_file = game_dir.join(rel_path);

        if is_symlink(&game_file) {
            remove_file_or_symlink(&game_file)?;
        }
        if let Some(parent) = game_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(entry.path(), &game_file)?;
    }
    Ok(())
}

/// List all cached manifest IDs for a given app/depot in the cache root.
pub fn list_cached_manifests(cache_root: &Path, app_id: u32, depot_id: u32) -> Vec<String> {
    let depot_dir = cache_root
        .join(app_id.to_string())
        .join(depot_id.to_string());

    if !depot_dir.exists() {
        return vec![];
    }

    std::fs::read_dir(&depot_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), CacheError> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), CacheError> {
    std::os::windows::fs::symlink_file(target, link)?;
    Ok(())
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn remove_file_or_symlink(path: &Path) -> Result<(), CacheError> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_game_dir(tmp: &TempDir) -> PathBuf {
        let game_dir = tmp.path().join("game");
        fs::create_dir_all(&game_dir).unwrap();
        fs::write(game_dir.join("main.pak"), b"original content v2").unwrap();
        fs::write(game_dir.join("unchanged.dat"), b"same always").unwrap();
        game_dir
    }

    fn make_cache_with_downloaded(tmp: &TempDir, manifest_id: &str) -> PathBuf {
        let cache_path = tmp.path().join("cache/1/2").join(manifest_id);
        fs::create_dir_all(&cache_path).unwrap();
        fs::write(cache_path.join("main.pak"), b"old content v1").unwrap();
        cache_path
    }

    #[test]
    fn apply_downloaded_creates_symlinks() {
        let tmp = TempDir::new().unwrap();
        let game_dir = make_game_dir(&tmp);
        let target_cache = make_cache_with_downloaded(&tmp, "target_manifest");
        let current_cache = tmp.path().join("cache/1/2/current_manifest");
        fs::create_dir_all(&current_cache).unwrap();

        apply_downloaded(&game_dir, &target_cache, &current_cache).unwrap();

        let link_path = game_dir.join("main.pak");
        assert!(link_path.exists(), "symlink target should exist");
        #[cfg(unix)]
        assert!(link_path.symlink_metadata().unwrap().file_type().is_symlink());

        let backup = current_cache.join("main.pak");
        assert!(backup.exists(), "original should be backed up");
        assert_eq!(fs::read(&backup).unwrap(), b"original content v2");
    }

    #[test]
    fn repoint_symlinks_switches_version() {
        let tmp = TempDir::new().unwrap();
        let game_dir = make_game_dir(&tmp);
        let v1_cache = make_cache_with_downloaded(&tmp, "v1");
        let v2_cache = make_cache_with_downloaded(&tmp, "v2");
        fs::write(v2_cache.join("main.pak"), b"new content v2").unwrap();
        let current_cache = tmp.path().join("cache/1/2/current");
        fs::create_dir_all(&current_cache).unwrap();

        apply_downloaded(&game_dir, &v1_cache, &current_cache).unwrap();
        repoint_symlinks(&game_dir, &v2_cache).unwrap();

        let content = fs::read(game_dir.join("main.pak")).unwrap();
        assert_eq!(content, b"new content v2");
    }

    #[test]
    fn restore_removes_symlinks_and_restores_files() {
        let tmp = TempDir::new().unwrap();
        let game_dir = make_game_dir(&tmp);
        let target_cache = make_cache_with_downloaded(&tmp, "target");
        let current_cache = tmp.path().join("cache/1/2/current");
        fs::create_dir_all(&current_cache).unwrap();

        apply_downloaded(&game_dir, &target_cache, &current_cache).unwrap();
        restore_from_cache(&game_dir, &current_cache).unwrap();

        #[cfg(unix)]
        assert!(!game_dir.join("main.pak").symlink_metadata().unwrap().file_type().is_symlink());
        let content = fs::read(game_dir.join("main.pak")).unwrap();
        assert_eq!(content, b"original content v2");
    }

    #[test]
    fn manifest_cache_dir_structure() {
        let root = Path::new("/tmp/cache");
        let dir = manifest_cache_dir(root, 1234, 5678, "abc123");
        assert_eq!(dir, Path::new("/tmp/cache/1234/5678/abc123"));
    }

    #[test]
    fn list_cached_manifests_returns_dirs() {
        let tmp = TempDir::new().unwrap();
        let cache_root = tmp.path();
        let dir = manifest_cache_dir(cache_root, 1234, 5678, "v1");
        fs::create_dir_all(&dir).unwrap();
        let dir2 = manifest_cache_dir(cache_root, 1234, 5678, "v2");
        fs::create_dir_all(&dir2).unwrap();

        let manifests = list_cached_manifests(cache_root, 1234, 5678);
        assert_eq!(manifests.len(), 2);
        assert!(manifests.contains(&"v1".to_string()));
        assert!(manifests.contains(&"v2".to_string()));
    }
}
