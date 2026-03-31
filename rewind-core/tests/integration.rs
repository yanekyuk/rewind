// rewind-core/tests/integration.rs
use rewind_core::{cache, immutability, patcher};
use std::fs;
use tempfile::TempDir;

/// Full downgrade → version switch → restore flow without network.
#[test]
fn full_downgrade_and_switch_flow() {
    let tmp = TempDir::new().unwrap();
    let game_dir = tmp.path().join("game");
    let cache_root = tmp.path().join("cache");

    fs::create_dir_all(&game_dir).unwrap();
    fs::write(game_dir.join("main.pak"), b"version 2 content").unwrap();
    fs::write(game_dir.join("static.dat"), b"unchanged").unwrap();

    // Simulate DepotDownloader output: v1 manifest downloaded
    let target_cache = cache::manifest_cache_dir(&cache_root, 1234, 5678, "v1_manifest");
    fs::create_dir_all(&target_cache).unwrap();
    fs::write(target_cache.join("main.pak"), b"version 1 content").unwrap();

    let current_cache = cache::manifest_cache_dir(&cache_root, 1234, 5678, "v2_manifest");

    // Step 1: apply downloaded — backup current, symlink to v1
    cache::apply_downloaded(&game_dir, &target_cache, &current_cache).unwrap();

    // game_dir/main.pak reads v1 via symlink
    assert_eq!(fs::read(game_dir.join("main.pak")).unwrap(), b"version 1 content");
    // static.dat untouched
    assert_eq!(fs::read(game_dir.join("static.dat")).unwrap(), b"unchanged");
    // current_cache has backup of v2
    assert_eq!(
        fs::read(current_cache.join("main.pak")).unwrap(),
        b"version 2 content"
    );

    // Step 2: create + patch ACF
    let acf_content = "\"AppState\"\n{\n\t\"appid\"\t\t\"1234\"\n\t\"StateFlags\"\t\"6\"\n\t\"buildid\"\t\"100\"\n\t\"InstalledDepots\"\n\t{\n\t\t\"5678\"\n\t\t{\n\t\t\t\"manifest\"\t\"v2_manifest\"\n\t\t}\n\t}\n}";
    let acf_path = tmp.path().join("appmanifest_1234.acf");
    fs::write(&acf_path, acf_content).unwrap();

    patcher::patch_acf_file(&acf_path, "99", "v1_manifest", 5678).unwrap();
    let patched = fs::read_to_string(&acf_path).unwrap();
    assert!(patched.contains("\"StateFlags\"\t\t\"4\""));
    assert!(patched.contains("\"manifest\"\t\t\"v1_manifest\""));

    // Step 3: lock ACF
    immutability::lock_file(&acf_path).unwrap();
    assert!(immutability::is_locked(&acf_path).unwrap());

    // Step 4: switch back to v2 (instant symlink repoint)
    cache::repoint_symlinks(&game_dir, &current_cache).unwrap();
    assert_eq!(
        fs::read(game_dir.join("main.pak")).unwrap(),
        b"version 2 content"
    );

    // Step 5: unlock and restore real files
    immutability::unlock_file(&acf_path).unwrap();
    assert!(!immutability::is_locked(&acf_path).unwrap());

    cache::restore_from_cache(&game_dir, &current_cache).unwrap();
    assert_eq!(
        fs::read(game_dir.join("main.pak")).unwrap(),
        b"version 2 content"
    );

    // After restore: game_dir/main.pak should NOT be a symlink
    #[cfg(unix)]
    {
        let meta = game_dir.join("main.pak").symlink_metadata().unwrap();
        assert!(
            !meta.file_type().is_symlink(),
            "file should be real after restore"
        );
    }
}

#[test]
fn manifest_cache_dir_path_structure() {
    let root = std::path::Path::new("/tmp/cache");
    let dir = cache::manifest_cache_dir(root, 1234, 5678, "abc");
    assert_eq!(dir, std::path::Path::new("/tmp/cache/1234/5678/abc"));
}
