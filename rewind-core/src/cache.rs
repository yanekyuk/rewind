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
            let mut manifests: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            manifests.sort();
            manifests
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestEntry {
    pub name: String,
    pub sha1: String,
    pub size_bytes: u64,
}

/// Parse a DepotDownloader manifest txt file (produced by `-manifest-only`) into entries.
/// Skips the header block. Data rows are whitespace-split: [size, chunks, sha1, flags, name]
pub fn parse_manifest_txt(path: &Path) -> Result<Vec<ManifestEntry>, CacheError> {
    let content = std::fs::read_to_string(path)?;
    let mut entries = Vec::new();
    let mut in_data = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Size") && trimmed.contains("Chunks") && trimmed.contains("File SHA") {
            in_data = true;
            continue;
        }
        if !in_data || trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let Ok(size_bytes) = parts[0].parse::<u64>() else { continue };
        let sha1 = parts[2].to_string();
        let name = parts[4..].join(" ");
        entries.push(ManifestEntry { name, sha1, size_bytes });
    }

    Ok(entries)
}

/// Move `src` into `depot_dir/.objects/<sha1>`.
/// If the object already exists, the source file is removed (discard the duplicate).
/// Returns the object path.
pub fn intern_object(depot_dir: &Path, src: &Path, sha1: &str) -> Result<PathBuf, CacheError> {
    let objects_dir = depot_dir.join(".objects");
    std::fs::create_dir_all(&objects_dir)?;
    let dest = objects_dir.join(sha1);
    if dest.try_exists().unwrap_or(false) {
        std::fs::remove_file(src)?;
    } else {
        std::fs::rename(src, &dest)?;
    }
    Ok(dest)
}

/// Returns entries whose SHA1 is not present in `depot_dir/.objects/<sha1>`.
pub fn missing_entries<'a>(
    depot_dir: &Path,
    entries: &'a [ManifestEntry],
) -> Vec<&'a ManifestEntry> {
    let objects_dir = depot_dir.join(".objects");
    entries
        .iter()
        .filter(|e| !objects_dir.join(&e.sha1).try_exists().unwrap_or(false))
        .collect()
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

    #[test]
    fn parse_manifest_txt_parses_entries() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.txt");
        fs::write(&path,
            "Content Manifest for Depot 3321461\n\
             \n\
             Manifest ID / date     : 123 / 01/01/2024 00:00:00\n\
             Total number of files  : 2\n\
             Total number of chunks : 5\n\
             Total bytes on disk    : 1000\n\
             Total bytes compressed : 800\n\
             \n\
             \n\
                       Size Chunks File SHA                                 Flags Name\n\
                    100      1 aabbccdd00112233445566778899001122334455     0 dir/file.pak\n\
                    200      2 ffeeddccbbaa99887766554433221100ffeeddcc     0 other.bin\n"
        ).unwrap();

        let entries = parse_manifest_txt(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "dir/file.pak");
        assert_eq!(entries[0].sha1, "aabbccdd00112233445566778899001122334455");
        assert_eq!(entries[0].size_bytes, 100);
        assert_eq!(entries[1].name, "other.bin");
        assert_eq!(entries[1].sha1, "ffeeddccbbaa99887766554433221100ffeeddcc");
        assert_eq!(entries[1].size_bytes, 200);
    }

    #[test]
    fn parse_manifest_txt_empty_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.txt");
        fs::write(&path, "").unwrap();
        let entries = parse_manifest_txt(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_manifest_txt_header_only_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.txt");
        fs::write(&path, "Content Manifest for Depot 123\n\nManifest ID: 456\n").unwrap();
        let entries = parse_manifest_txt(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_manifest_txt_handles_name_with_spaces() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.txt");
        fs::write(&path,
            "          Size Chunks File SHA                                 Flags Name\n\
                    100      1 aabbccdd00112233445566778899001122334455     0 Data Files/main.pak\n"
        ).unwrap();

        let entries = parse_manifest_txt(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Data Files/main.pak");
    }

    #[test]
    fn missing_entries_all_missing() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            ManifestEntry { name: "a.pak".into(), sha1: "aaaa".into(), size_bytes: 10 },
            ManifestEntry { name: "b.pak".into(), sha1: "bbbb".into(), size_bytes: 20 },
        ];
        let missing = missing_entries(tmp.path(), &entries);
        assert_eq!(missing.len(), 2);
    }

    #[test]
    fn missing_entries_all_present() {
        let tmp = TempDir::new().unwrap();
        let objects = tmp.path().join(".objects");
        fs::create_dir_all(&objects).unwrap();
        fs::write(objects.join("aaaa"), b"content a").unwrap();
        fs::write(objects.join("bbbb"), b"content b").unwrap();
        let entries = vec![
            ManifestEntry { name: "a.pak".into(), sha1: "aaaa".into(), size_bytes: 10 },
            ManifestEntry { name: "b.pak".into(), sha1: "bbbb".into(), size_bytes: 20 },
        ];
        let missing = missing_entries(tmp.path(), &entries);
        assert!(missing.is_empty());
    }

    #[test]
    fn missing_entries_mixed() {
        let tmp = TempDir::new().unwrap();
        let objects = tmp.path().join(".objects");
        fs::create_dir_all(&objects).unwrap();
        fs::write(objects.join("aaaa"), b"content a").unwrap();
        let entries = vec![
            ManifestEntry { name: "a.pak".into(), sha1: "aaaa".into(), size_bytes: 10 },
            ManifestEntry { name: "b.pak".into(), sha1: "bbbb".into(), size_bytes: 20 },
        ];
        let missing = missing_entries(tmp.path(), &entries);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].sha1, "bbbb");
    }

    #[test]
    fn intern_object_moves_file_to_objects() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.pak");
        fs::write(&src, b"game content").unwrap();
        let depot_dir = tmp.path().join("depot");

        let result = intern_object(&depot_dir, &src, "abc123").unwrap();

        assert!(!src.exists());
        assert_eq!(result, depot_dir.join(".objects/abc123"));
        assert_eq!(fs::read(&result).unwrap(), b"game content");
    }

    #[test]
    fn intern_object_idempotent_when_object_exists() {
        let tmp = TempDir::new().unwrap();
        let objects = tmp.path().join(".objects");
        fs::create_dir_all(&objects).unwrap();
        fs::write(objects.join("abc123"), b"existing").unwrap();

        let src = tmp.path().join("dup.pak");
        fs::write(&src, b"duplicate content").unwrap();

        let result = intern_object(tmp.path(), &src, "abc123").unwrap();

        assert!(!src.exists());
        assert_eq!(fs::read(&result).unwrap(), b"existing");
    }

    #[test]
    fn intern_object_returns_correct_path() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("x.pak");
        fs::write(&src, b"x").unwrap();

        let path = intern_object(tmp.path(), &src, "deadbeef").unwrap();
        assert_eq!(path, tmp.path().join(".objects/deadbeef"));
    }
}
