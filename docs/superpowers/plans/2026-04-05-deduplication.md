# Content-Addressed Cache Deduplication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Avoid downloading files already in cache by checking SHA1 before download, and store all cached game files once in a content-addressed object store.

**Architecture:** DepotDownloader's `-manifest-only` flag produces a manifest txt with per-file SHA1s. Before each download, rewind checks which files are already in `cache/<app_id>/<depot_id>/.objects/<sha1>` and downloads only the missing ones. After download, new files are moved into `.objects/` and manifest dirs hold only symlinks.

**Tech Stack:** Rust, `walkdir`, `tempfile` (tests), `tokio::process::Command`

---

## File Structure

- **Modify:** `rewind-core/src/cache.rs` — add `ManifestEntry`, `parse_manifest_txt`, `missing_entries`, `intern_object`, `link_object`; fix `list_cached_manifests` and `repoint_symlinks`
- **Modify:** `rewind-core/src/depot.rs` — add `run_manifest_only`, `build_filelist_args`; update `DepotProgress::ReadyToDownload`; update `run_depot_downloader` signature
- **Modify:** `rewind-cli/src/main.rs` — update `start_download` spawn, `ReadyToDownload` handler, `finalize_downgrade_with_steps`

---

### Task 1: `ManifestEntry` and `parse_manifest_txt`

**Files:**
- Modify: `rewind-core/src/cache.rs`

- [ ] **Step 1: Write the failing tests**

Add at the bottom of the `#[cfg(test)]` block in `rewind-core/src/cache.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p rewind-core parse_manifest_txt
```

Expected: FAIL with "cannot find function `parse_manifest_txt`"

- [ ] **Step 3: Add `ManifestEntry` and `parse_manifest_txt` to `cache.rs`**

Add before the `#[cfg(test)]` block:

```rust
#[derive(Debug, Clone)]
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
        let size_bytes: u64 = parts[0].parse().unwrap_or(0);
        let sha1 = parts[2].to_string();
        let name = parts[4..].join(" ");
        entries.push(ManifestEntry { name, sha1, size_bytes });
    }

    Ok(entries)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p rewind-core parse_manifest_txt
```

Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/cache.rs
git commit -m "feat(core): add ManifestEntry and parse_manifest_txt"
```

---

### Task 2: `missing_entries`

**Files:**
- Modify: `rewind-core/src/cache.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p rewind-core missing_entries
```

Expected: FAIL with "cannot find function `missing_entries`"

- [ ] **Step 3: Add `missing_entries` to `cache.rs`**

Add after `parse_manifest_txt`:

```rust
/// Returns entries whose SHA1 is not present in `depot_dir/.objects/<sha1>`.
pub fn missing_entries<'a>(
    depot_dir: &Path,
    entries: &'a [ManifestEntry],
) -> Vec<&'a ManifestEntry> {
    let objects_dir = depot_dir.join(".objects");
    entries
        .iter()
        .filter(|e| !objects_dir.join(&e.sha1).exists())
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p rewind-core missing_entries
```

Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/cache.rs
git commit -m "feat(core): add missing_entries"
```

---

### Task 3: `intern_object`

**Files:**
- Modify: `rewind-core/src/cache.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p rewind-core intern_object
```

Expected: FAIL with "cannot find function `intern_object`"

- [ ] **Step 3: Add `intern_object` to `cache.rs`**

Add after `missing_entries`:

```rust
/// Move `src` into `depot_dir/.objects/<sha1>`.
/// If the object already exists, the source file is removed (discard the duplicate).
/// Returns the object path.
pub fn intern_object(depot_dir: &Path, src: &Path, sha1: &str) -> Result<PathBuf, CacheError> {
    let objects_dir = depot_dir.join(".objects");
    std::fs::create_dir_all(&objects_dir)?;
    let dest = objects_dir.join(sha1);
    if dest.exists() {
        std::fs::remove_file(src)?;
    } else {
        std::fs::rename(src, &dest)?;
    }
    Ok(dest)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p rewind-core intern_object
```

Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/cache.rs
git commit -m "feat(core): add intern_object"
```

---

### Task 4: `link_object`

**Files:**
- Modify: `rewind-core/src/cache.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block:

```rust
#[test]
fn link_object_creates_readable_symlink() {
    let tmp = TempDir::new().unwrap();
    let objects = tmp.path().join(".objects");
    fs::create_dir_all(&objects).unwrap();
    fs::write(objects.join("abc123"), b"game content").unwrap();

    let manifest_dir = tmp.path().join("manifest_aaa");
    fs::create_dir_all(&manifest_dir).unwrap();

    link_object(tmp.path(), "abc123", &manifest_dir, "file.pak").unwrap();

    let link = manifest_dir.join("file.pak");
    #[cfg(unix)]
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read(&link).unwrap(), b"game content");
}

#[test]
fn link_object_creates_subdirectories() {
    let tmp = TempDir::new().unwrap();
    let objects = tmp.path().join(".objects");
    fs::create_dir_all(&objects).unwrap();
    fs::write(objects.join("deadbeef"), b"chunk data").unwrap();

    let manifest_dir = tmp.path().join("manifest_bbb");
    fs::create_dir_all(&manifest_dir).unwrap();

    link_object(tmp.path(), "deadbeef", &manifest_dir, "0000/0.paz").unwrap();

    let link = manifest_dir.join("0000/0.paz");
    assert!(link.exists());
    assert_eq!(fs::read(&link).unwrap(), b"chunk data");
}

#[test]
fn link_object_overwrites_existing_symlink() {
    let tmp = TempDir::new().unwrap();
    let objects = tmp.path().join(".objects");
    fs::create_dir_all(&objects).unwrap();
    fs::write(objects.join("v1hash"), b"v1 content").unwrap();
    fs::write(objects.join("v2hash"), b"v2 content").unwrap();

    let manifest_dir = tmp.path().join("manifest");
    fs::create_dir_all(&manifest_dir).unwrap();

    link_object(tmp.path(), "v1hash", &manifest_dir, "file.pak").unwrap();
    link_object(tmp.path(), "v2hash", &manifest_dir, "file.pak").unwrap();

    assert_eq!(fs::read(manifest_dir.join("file.pak")).unwrap(), b"v2 content");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p rewind-core link_object
```

Expected: FAIL with "cannot find function `link_object`"

- [ ] **Step 3: Add `link_object` to `cache.rs`**

Add after `intern_object`:

```rust
/// Create a symlink at `manifest_dir/<name>` pointing to the absolute path of
/// `depot_dir/.objects/<sha1>`. Creates parent directories as needed.
/// Overwrites an existing symlink at the same path.
pub fn link_object(
    depot_dir: &Path,
    sha1: &str,
    manifest_dir: &Path,
    name: &str,
) -> Result<(), CacheError> {
    let object_path = depot_dir.join(".objects").join(sha1);
    let target_abs = std::fs::canonicalize(&object_path)?;
    let link_path = manifest_dir.join(name);

    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if link_path.exists() || is_symlink(&link_path) {
        remove_file_or_symlink(&link_path)?;
    }
    create_symlink(&target_abs, &link_path)?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p rewind-core link_object
```

Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/cache.rs
git commit -m "feat(core): add link_object"
```

---

### Task 5: Fix `list_cached_manifests` and `repoint_symlinks`

**Files:**
- Modify: `rewind-core/src/cache.rs`

`list_cached_manifests` must exclude `.objects` (which is now a subdirectory of the depot dir).  
`repoint_symlinks` must follow symlinks so it works when manifest dirs contain symlinks to `.objects/`.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block:

```rust
#[test]
fn list_cached_manifests_excludes_objects_dir() {
    let tmp = TempDir::new().unwrap();
    let cache_root = tmp.path();
    let dir1 = manifest_cache_dir(cache_root, 1234, 5678, "v1");
    fs::create_dir_all(&dir1).unwrap();
    let objects = cache_root.join("1234/5678/.objects");
    fs::create_dir_all(&objects).unwrap();

    let manifests = list_cached_manifests(cache_root, 1234, 5678);
    assert_eq!(manifests, vec!["v1".to_string()]);
    assert!(!manifests.contains(&".objects".to_string()));
}

#[test]
fn repoint_symlinks_follows_symlinks_into_objects() {
    let tmp = TempDir::new().unwrap();
    // Set up object store
    let depot_dir = tmp.path().join("cache/1/2");
    let objects = depot_dir.join(".objects");
    fs::create_dir_all(&objects).unwrap();
    fs::write(objects.join("sha_v2"), b"v2 content").unwrap();

    // Manifest dir where file is a symlink to .objects
    let manifest_dir = depot_dir.join("v2");
    fs::create_dir_all(&manifest_dir).unwrap();
    let obj_abs = fs::canonicalize(objects.join("sha_v2")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&obj_abs, manifest_dir.join("main.pak")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&obj_abs, manifest_dir.join("main.pak")).unwrap();

    // Game dir with an old symlink
    let game_dir = tmp.path().join("game");
    fs::create_dir_all(&game_dir).unwrap();
    let old_obj = objects.parent().unwrap().join(".objects/sha_v1");
    // (no v1 object needed — just create the game dir file as a regular file)
    fs::write(game_dir.join("main.pak"), b"old content").unwrap();

    repoint_symlinks(&game_dir, &manifest_dir).unwrap();

    assert_eq!(fs::read(game_dir.join("main.pak")).unwrap(), b"v2 content");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p rewind-core list_cached_manifests_excludes_objects_dir repoint_symlinks_follows_symlinks_into_objects
```

Expected: `list_cached_manifests_excludes_objects_dir` fails (returns `.objects`); `repoint_symlinks_follows_symlinks_into_objects` fails (no files found via symlinks)

- [ ] **Step 3: Fix `list_cached_manifests`**

In `list_cached_manifests`, add a filter to exclude `.objects`. Find this block:

```rust
            let mut manifests: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
```

Replace with:

```rust
            let mut manifests: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name != ".objects")
                .collect();
```

- [ ] **Step 4: Fix `repoint_symlinks`**

In `repoint_symlinks`, add `.follow_links(true)` to the WalkDir call. Find:

```rust
    for entry in WalkDir::new(new_cache_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
```

Replace with:

```rust
    for entry in WalkDir::new(new_cache_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
```

- [ ] **Step 5: Run tests to verify they pass**

```
cargo test -p rewind-core
```

Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add rewind-core/src/cache.rs
git commit -m "fix(core): exclude .objects from manifest list; repoint_symlinks follows links"
```

---

### Task 6: Depot functions — `run_manifest_only`, `build_filelist_args`, updated `run_depot_downloader`

**Files:**
- Modify: `rewind-core/src/depot.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `depot.rs`:

```rust
#[test]
fn build_filelist_args_includes_filelist_flag() {
    let args = build_filelist_args(570, 571, "abc123", "user", "/tmp/out", "/tmp/list.txt");
    let s: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    assert!(s.contains(&"-filelist"));
    let idx = s.iter().position(|&x| x == "-filelist").unwrap();
    assert_eq!(s[idx + 1], "/tmp/list.txt");
    // Standard args still present
    assert!(s.contains(&"-app"));
    assert!(s.contains(&"-manifest"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

```
cargo test -p rewind-core build_filelist_args
```

Expected: FAIL with "cannot find function `build_filelist_args`"

- [ ] **Step 3: Add `build_filelist_args` to `depot.rs`**

Add after `build_args`:

```rust
/// Build args for a targeted download using a filelist.
/// Used when only a subset of manifest files need to be downloaded (deduplication).
pub fn build_filelist_args(
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    output_dir: &str,
    filelist_path: &str,
) -> Vec<String> {
    let mut args = build_args(app_id, depot_id, manifest_id, username, output_dir);
    args.push("-filelist".into());
    args.push(filelist_path.into());
    args
}
```

- [ ] **Step 4: Run the test to verify it passes**

```
cargo test -p rewind-core build_filelist_args
```

Expected: PASS

- [ ] **Step 5: Add `run_manifest_only` to `depot.rs`**

Add after `run_depot_downloader_interactive`. This function runs DepotDownloader with `-manifest-only`, producing `manifest_<depot_id>_<manifest_id>.txt` in `cache_dir`. No interactive I/O needed.

```rust
/// Run DepotDownloader with `-manifest-only` to produce a human-readable manifest file.
/// No game files are downloaded. The manifest txt is written to `cache_dir`.
pub async fn run_manifest_only(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
) -> Result<(), DepotError> {
    std::fs::create_dir_all(cache_dir)?;
    let mut args = build_args(
        app_id,
        depot_id,
        manifest_id,
        username,
        cache_dir.to_string_lossy().as_ref(),
    );
    args.push("-manifest-only".into());

    let mut cmd = Command::new(binary);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let status = cmd.spawn()?.wait().await?;
    if status.success() {
        Ok(())
    } else {
        Err(DepotError::ExitFailure(status.code().unwrap_or(-1)))
    }
}
```

- [ ] **Step 6: Update `DepotProgress::ReadyToDownload` to carry the filelist path**

Find:

```rust
    /// DepotDownloader binary is ready at this path; interactive download can start.
    ReadyToDownload { binary: std::path::PathBuf },
```

Replace with:

```rust
    /// DepotDownloader binary is ready; interactive download can start.
    /// `filelist_path` is Some when only missing files need downloading (deduplication).
    /// If None, all files are already cached and the download should be skipped.
    ReadyToDownload { binary: std::path::PathBuf, filelist_path: Option<std::path::PathBuf> },
```

- [ ] **Step 7: Update `run_depot_downloader` to accept an optional filelist path**

Find the function signature:

```rust
pub async fn run_depot_downloader(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
    tx: mpsc::Sender<DepotProgress>,
) -> Result<(tokio::process::ChildStdin, mpsc::Sender<()>), DepotError> {
    std::fs::create_dir_all(cache_dir)?;

    let args = build_args(
        app_id,
        depot_id,
        manifest_id,
        username,
        cache_dir.to_string_lossy().as_ref(),
    );
```

Replace with:

```rust
pub async fn run_depot_downloader(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
    filelist_path: Option<&Path>,
    tx: mpsc::Sender<DepotProgress>,
) -> Result<(tokio::process::ChildStdin, mpsc::Sender<()>), DepotError> {
    std::fs::create_dir_all(cache_dir)?;

    let args = match filelist_path {
        Some(fp) => build_filelist_args(
            app_id,
            depot_id,
            manifest_id,
            username,
            cache_dir.to_string_lossy().as_ref(),
            fp.to_string_lossy().as_ref(),
        ),
        None => build_args(
            app_id,
            depot_id,
            manifest_id,
            username,
            cache_dir.to_string_lossy().as_ref(),
        ),
    };
```

- [ ] **Step 8: Run all tests**

```
cargo test -p rewind-core
```

Expected: all tests pass. (The CLI will fail to compile until Task 7 — that's fine.)

- [ ] **Step 9: Commit**

```bash
git add rewind-core/src/depot.rs
git commit -m "feat(core): add run_manifest_only, build_filelist_args; update ReadyToDownload and run_depot_downloader"
```

---

### Task 7: Wire up two-phase download flow in `main.rs`

**Files:**
- Modify: `rewind-cli/src/main.rs`

This task has three parts: (A) update the tokio spawn to run manifest-only and compute the filelist, (B) update the `ReadyToDownload` handler, (C) replace `apply_downloaded` in `finalize_downgrade_with_steps`.

- [ ] **Step 1: Fix compile errors from `ReadyToDownload` variant change**

`ReadyToDownload` now has `filelist_path`. Find the match arm (around line 140):

```rust
                rewind_core::depot::DepotProgress::ReadyToDownload { binary } => {
```

Replace with (temporary — full rewrite follows in Step 3):

```rust
                rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path } => {
```

Also find the send in `start_download` (around line 1043):

```rust
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary })
            .await;
```

Replace with:

```rust
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path: None })
            .await;
```

Also fix the `run_depot_downloader` call (it now requires `filelist_path`). Find:

```rust
                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            tx_d,
                        )
```

Replace with:

```rust
                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            filelist_path.as_deref(),
                            tx_d,
                        )
```

- [ ] **Step 2: Verify it compiles**

```
cargo check
```

Expected: no errors

- [ ] **Step 3: Update `start_download` spawn to run manifest-only and compute the filelist**

In `start_download`, clone the download parameters before the spawn. Find (around line 989):

```rust
    app.pending_download = Some(PendingDownload {
        app_id: game.app_id,
        depot_id: game.depot_id,
        manifest_id,
        username: username.clone(),
        cache_dir: cache_dir.clone(),
        game_name: game.name.clone(),
        game_install_path: game.install_path.clone(),
        current_manifest_id: game.manifest_id.clone(),
        acf_path: game.acf_path.clone(),
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
```

Replace with:

```rust
    let dl_app_id = game.app_id;
    let dl_depot_id = game.depot_id;
    let dl_manifest_id = manifest_id.clone();
    let dl_username = username.clone();
    let dl_cache_dir = cache_dir.clone();

    app.pending_download = Some(PendingDownload {
        app_id: game.app_id,
        depot_id: game.depot_id,
        manifest_id,
        username: username.clone(),
        cache_dir: cache_dir.clone(),
        game_name: game.name.clone(),
        game_install_path: game.install_path.clone(),
        current_manifest_id: game.manifest_id.clone(),
        acf_path: game.acf_path.clone(),
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
```

Now find the end of the spawn (around line 1037), which currently reads:

```rust
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadManifest".into(),
            ))
            .await;
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path: None })
            .await;
    });
```

Replace with:

```rust
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadManifest".into(),
            ))
            .await;

        // Phase 1: run manifest-only to get per-file SHA1s without downloading
        if let Err(e) = rewind_core::depot::run_manifest_only(
            &binary,
            dl_app_id,
            dl_depot_id,
            &dl_manifest_id,
            &dl_username,
            &dl_cache_dir,
        )
        .await
        {
            let _ = tx2
                .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                .await;
            return;
        }

        // Parse manifest txt and determine which files are missing from the object store
        let manifest_txt = dl_cache_dir
            .join(format!("manifest_{}_{}.txt", dl_depot_id, dl_manifest_id));
        let entries = rewind_core::cache::parse_manifest_txt(&manifest_txt).unwrap_or_default();
        let depot_dir = dl_cache_dir
            .parent()
            .expect("manifest cache dir has parent depot dir")
            .to_path_buf();
        let missing = rewind_core::cache::missing_entries(&depot_dir, &entries);

        let filelist_path = if missing.is_empty() {
            None
        } else {
            let path = dl_cache_dir.join(".filelist");
            let content = missing.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join("\n");
            if let Err(e) = std::fs::write(&path, content) {
                let _ = tx2
                    .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                    .await;
                return;
            }
            Some(path)
        };

        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path })
            .await;
    });
```

- [ ] **Step 4: Update `ReadyToDownload` handler to skip download when nothing is missing**

Find the `ReadyToDownload` handler (around line 140):

```rust
                rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path } => {
                    if let Some(ref dl) = app.pending_download {
                        let (tx_d, rx_d) = mpsc::channel(64);
                        app.progress_rx = Some(rx_d);

                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            filelist_path.as_deref(),
                            tx_d,
                        )
                        .await
                        {
                            Ok((stdin, kill_tx)) => {
                                app.depot_stdin = Some(stdin);
                                app.depot_kill = Some(kill_tx);
                            }
                            Err(e) => {
                                app.set_step_status(
                                    &StepKind::DownloadManifest,
                                    StepStatus::Failed(e.to_string()),
                                );
                                app.wizard_state.error =
                                    Some(format!("Failed to start download: {}", e));
                                app.wizard_state.is_downloading = false;
                            }
                        }
                    }
                }
```

Replace with:

```rust
                rewind_core::depot::DepotProgress::ReadyToDownload { binary, filelist_path } => {
                    if filelist_path.is_none() {
                        // All files already in object store — skip download, finalize directly
                        app.set_step_status(&StepKind::DownloadManifest, StepStatus::Done);
                        if let Some(dl) = app.pending_download.take() {
                            finalize_downgrade_with_steps(&mut app, dl);
                        }
                    } else if let Some(ref dl) = app.pending_download {
                        let (tx_d, rx_d) = mpsc::channel(64);
                        app.progress_rx = Some(rx_d);

                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            filelist_path.as_deref(),
                            tx_d,
                        )
                        .await
                        {
                            Ok((stdin, kill_tx)) => {
                                app.depot_stdin = Some(stdin);
                                app.depot_kill = Some(kill_tx);
                            }
                            Err(e) => {
                                app.set_step_status(
                                    &StepKind::DownloadManifest,
                                    StepStatus::Failed(e.to_string()),
                                );
                                app.wizard_state.error =
                                    Some(format!("Failed to start download: {}", e));
                                app.wizard_state.is_downloading = false;
                            }
                        }
                    }
                }
```

- [ ] **Step 5: Replace `apply_downloaded` in `finalize_downgrade_with_steps`**

In `finalize_downgrade_with_steps`, the `current_cache` variable is computed but `apply_downloaded` no longer handles the full flow. We replace that call with:
1. Parse the manifest txt (already on disk from the manifest-only run)
2. Backup current game files to `current_cache`
3. Intern newly downloaded files into `.objects/`
4. Link all manifest entries in `target_cache`
5. Repoint game dir symlinks (existing `repoint_symlinks` call)

Find (around line 1224):

```rust
    let target_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.manifest_id,
    );
    let current_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.current_manifest_id,
    );

    // Step 4: Backup + Step 5: Link
    app.set_step_status(&StepKind::BackupFiles, StepStatus::InProgress);
    if let Err(e) =
        rewind_core::cache::apply_downloaded(&dl.game_install_path, &target_cache, &current_cache)
    {
        app.set_step_status(&StepKind::BackupFiles, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to apply files: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::BackupFiles, StepStatus::Done);
    app.set_step_status(&StepKind::LinkFiles, StepStatus::Done);
```

Replace with:

```rust
    let target_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.manifest_id,
    );
    let current_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.current_manifest_id,
    );
    let depot_dir = cache_root
        .join(dl.app_id.to_string())
        .join(dl.depot_id.to_string());

    // Parse manifest txt (produced by the manifest-only run in start_download)
    let manifest_txt = target_cache
        .join(format!("manifest_{}_{}.txt", dl.depot_id, dl.manifest_id));
    let entries = rewind_core::cache::parse_manifest_txt(&manifest_txt).unwrap_or_default();

    // Step 4: Backup current game files → current_cache (real file copies)
    app.set_step_status(&StepKind::BackupFiles, StepStatus::InProgress);
    if let Err(e) = std::fs::create_dir_all(&current_cache) {
        app.set_step_status(&StepKind::BackupFiles, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to create backup dir: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    for entry in &entries {
        let game_file = dl.game_install_path.join(&entry.name);
        let backup = current_cache.join(&entry.name);
        if let Some(parent) = backup.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if game_file.exists() {
            if let Err(e) = std::fs::copy(&game_file, &backup) {
                app.set_step_status(&StepKind::BackupFiles, StepStatus::Failed(e.to_string()));
                app.wizard_state.error = Some(format!("Failed to backup file: {}", e));
                app.wizard_state.is_downloading = false;
                return;
            }
        }
    }
    app.set_step_status(&StepKind::BackupFiles, StepStatus::Done);

    // Step 5: Intern newly downloaded files + link all entries in target_cache
    app.set_step_status(&StepKind::LinkFiles, StepStatus::InProgress);
    for entry in &entries {
        let file_in_cache = target_cache.join(&entry.name);
        // Intern if this is a real file (just downloaded — not already a symlink to .objects)
        if file_in_cache.exists() && !file_in_cache.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            if let Err(e) = rewind_core::cache::intern_object(&depot_dir, &file_in_cache, &entry.sha1) {
                app.set_step_status(&StepKind::LinkFiles, StepStatus::Failed(e.to_string()));
                app.wizard_state.error = Some(format!("Failed to intern file: {}", e));
                app.wizard_state.is_downloading = false;
                return;
            }
        }
        // Create symlink in target_cache pointing to .objects/<sha1>
        if let Err(e) = rewind_core::cache::link_object(&depot_dir, &entry.sha1, &target_cache, &entry.name) {
            app.set_step_status(&StepKind::LinkFiles, StepStatus::Failed(e.to_string()));
            app.wizard_state.error = Some(format!("Failed to link file: {}", e));
            app.wizard_state.is_downloading = false;
            return;
        }
    }

    // Repoint game dir symlinks to the new manifest's objects
    if let Err(e) = rewind_core::cache::repoint_symlinks(&dl.game_install_path, &target_cache) {
        app.set_step_status(&StepKind::LinkFiles, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to link game files: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::LinkFiles, StepStatus::Done);
```

- [ ] **Step 6: Verify it compiles and all tests pass**

```
cargo check && cargo test
```

Expected: no compile errors, all tests pass

- [ ] **Step 7: Commit**

```bash
git add rewind-cli/src/main.rs
git commit -m "feat(cli): wire up two-phase dedup download flow"
```
