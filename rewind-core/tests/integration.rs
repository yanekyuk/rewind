// rewind-core/tests/integration.rs
use rewind_core::{
    cache::{self, intern_object, link_object, missing_entries, ManifestEntry},
    immutability, patcher,
};
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

// Helper: build a ManifestEntry for tests.
fn entry(name: &str, sha1: &str) -> ManifestEntry {
    ManifestEntry { name: name.to_string(), sha1: sha1.to_string(), size_bytes: 4 }
}

/// Scenario: all files missing from .objects/ → every entry must be interned + linked.
#[test]
fn dedup_all_files_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let depot_dir = tmp.path().join("depot");
    let manifest_dir = tmp.path().join("manifest");
    fs::create_dir_all(&manifest_dir).unwrap();

    let entries = vec![
        entry("a.pak", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        entry("b.pak", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    ];

    // Nothing in .objects/ yet — all are missing.
    let missing = missing_entries(&depot_dir, &entries);
    assert_eq!(missing.len(), 2);

    // Simulate download: place files in manifest_dir as DepotDownloader would.
    fs::write(manifest_dir.join("a.pak"), b"aaa content").unwrap();
    fs::write(manifest_dir.join("b.pak"), b"bbb content").unwrap();

    // Intern + link each file.
    for e in &entries {
        let src = manifest_dir.join(&e.name);
        intern_object(&depot_dir, &src, &e.sha1).unwrap();
        link_object(&depot_dir, &e.sha1, &manifest_dir, &e.name).unwrap();
    }

    // Objects exist.
    assert!(depot_dir.join(".objects").join(&entries[0].sha1).exists());
    assert!(depot_dir.join(".objects").join(&entries[1].sha1).exists());

    // Symlinks in manifest_dir resolve to correct content.
    assert_eq!(fs::read(manifest_dir.join("a.pak")).unwrap(), b"aaa content");
    assert_eq!(fs::read(manifest_dir.join("b.pak")).unwrap(), b"bbb content");

    // All present now — nothing missing.
    let missing_after = missing_entries(&depot_dir, &entries);
    assert!(missing_after.is_empty());
}

/// Scenario: all files already present in .objects/ → nothing to download, only link.
#[test]
fn dedup_all_files_present() {
    let tmp = tempfile::TempDir::new().unwrap();
    let depot_dir = tmp.path().join("depot");
    let manifest_dir = tmp.path().join("manifest");
    fs::create_dir_all(&manifest_dir).unwrap();

    let entries = vec![entry("0000/0.paz", "cccccccccccccccccccccccccccccccccccccccc")];

    // Pre-populate .objects/ as if a prior manifest had already downloaded this file.
    let objects_dir = depot_dir.join(".objects");
    fs::create_dir_all(&objects_dir).unwrap();
    fs::write(objects_dir.join(&entries[0].sha1), b"shared content").unwrap();

    // Nothing missing — skip download entirely.
    let missing = missing_entries(&depot_dir, &entries);
    assert!(missing.is_empty());

    // Still create symlinks.
    for e in &entries {
        link_object(&depot_dir, &e.sha1, &manifest_dir, &e.name).unwrap();
    }

    // Symlink resolves correctly without any download.
    assert_eq!(
        fs::read(manifest_dir.join("0000/0.paz")).unwrap(),
        b"shared content"
    );
}

/// Scenario: partial overlap — only missing files downloaded + interned, all linked.
#[test]
fn dedup_partial_overlap() {
    let tmp = tempfile::TempDir::new().unwrap();
    let depot_dir = tmp.path().join("depot");
    let manifest_dir = tmp.path().join("manifest");
    fs::create_dir_all(&manifest_dir).unwrap();

    let entries = vec![
        entry("old.pak", "dddddddddddddddddddddddddddddddddddddddd"), // already cached
        entry("new.pak", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"), // needs download
    ];

    // Pre-populate only the first object.
    let objects_dir = depot_dir.join(".objects");
    fs::create_dir_all(&objects_dir).unwrap();
    fs::write(objects_dir.join(&entries[0].sha1), b"old content").unwrap();

    // Only new.pak is missing.
    let missing = missing_entries(&depot_dir, &entries);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].sha1, entries[1].sha1);

    // Simulate downloading only new.pak.
    fs::write(manifest_dir.join("new.pak"), b"new content").unwrap();
    intern_object(&depot_dir, &manifest_dir.join("new.pak"), &entries[1].sha1).unwrap();

    // Link all entries (both cached and newly downloaded).
    for e in &entries {
        link_object(&depot_dir, &e.sha1, &manifest_dir, &e.name).unwrap();
    }

    assert_eq!(fs::read(manifest_dir.join("old.pak")).unwrap(), b"old content");
    assert_eq!(fs::read(manifest_dir.join("new.pak")).unwrap(), b"new content");
    assert!(missing_entries(&depot_dir, &entries).is_empty());
}

/// Regression: restore_from_cache correctly copies content through a symlink chain
/// (manifest_dir symlink → .objects/ real file).
#[test]
fn restore_from_cache_follows_symlink_chain() {
    let tmp = tempfile::TempDir::new().unwrap();
    let depot_dir = tmp.path().join("depot");
    let manifest_dir = tmp.path().join("manifest");
    let game_dir = tmp.path().join("game");
    fs::create_dir_all(&game_dir).unwrap();

    let e = entry("main.pak", "ffffffffffffffffffffffffffffffffffffffff");

    // Intern the object.
    let objects_dir = depot_dir.join(".objects");
    fs::create_dir_all(&objects_dir).unwrap();
    fs::write(objects_dir.join(&e.sha1), b"v1 content").unwrap();

    // Link into manifest_dir.
    link_object(&depot_dir, &e.sha1, &manifest_dir, &e.name).unwrap();

    // Repoint game_dir to use the symlinks in manifest_dir.
    cache::repoint_symlinks(&game_dir, &manifest_dir).unwrap();
    assert_eq!(fs::read(game_dir.join("main.pak")).unwrap(), b"v1 content");

    // restore_from_cache must copy actual bytes, not a dangling symlink.
    cache::restore_from_cache(&game_dir, &manifest_dir).unwrap();
    assert_eq!(fs::read(game_dir.join("main.pak")).unwrap(), b"v1 content");

    #[cfg(unix)]
    {
        let meta = game_dir.join("main.pak").symlink_metadata().unwrap();
        assert!(!meta.file_type().is_symlink(), "restored file must not be a symlink");
    }
}

#[test]
fn manifest_cache_dir_path_structure() {
    let root = std::path::Path::new("/tmp/cache");
    let dir = cache::manifest_cache_dir(root, 1234, 5678, "abc");
    assert_eq!(dir, std::path::Path::new("/tmp/cache/1234/5678/abc"));
}
