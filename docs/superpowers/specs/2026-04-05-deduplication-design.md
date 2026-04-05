# Content-Addressed Cache Deduplication — Design Spec

**Issue:** [#46](https://github.com/yanekyuk/rewind/issues/46)  
**Milestone:** 0.7.0 — Cache and manifest management

---

## Problem

When multiple versions are cached for a game, files that are identical across versions are stored multiple times on disk. There is no mechanism to detect or eliminate these duplicates, and no way to avoid re-downloading files that are already cached in a different manifest.

---

## Goals

- Eliminate redundant file storage: identical files across manifest versions are stored once
- Avoid downloading files already in the cache: pre-check SHA1 against the object store before downloading
- Keep the existing symlink-based game directory mechanism intact

---

## Non-Goals

- Deletion of cached versions (#34) — handled in 0.8.0 once the object store is in place
- Disk usage display (#33) — handled in 0.8.0
- Cross-game or cross-depot deduplication (depot boundaries are the unit of deduplication)

---

## Architecture

### Content-addressed object store

A `.objects/` directory inside each depot's cache dir holds the canonical file content, keyed by Steam's per-file SHA1 hash:

```
cache/<app_id>/<depot_id>/
├── .objects/
│   ├── 8a11847b3e22b2fb909b57787ed94d1bb139bcb2   ← real file
│   ├── 3e6800918fef5f8880cf601e5b60bff031465e60   ← real file
│   └── ...
├── <manifest_id_1>/
│   ├── 0000/0.pamt   → ../../.objects/8a11847b...   ← symlink
│   ├── 0000/0.paz    → ../../.objects/3e680091...   ← symlink
│   └── ...
└── <manifest_id_2>/
    ├── 0000/0.pamt   → ../../.objects/8a11847b...   ← same object, no duplicate
    └── ...
```

Every file in a manifest directory is a symlink. Real bytes live in `.objects/` exactly once per unique SHA1.

### Why SHA1 (not BLAKE3 or SHA256)

DepotDownloader's `-manifest-only` flag produces a human-readable `manifest_<depot>_<manifest>.txt` with a per-file SHA1. Using Steam's SHA1 directly as the object key means:

- No hashing step required on our side — the hash is known before downloading
- Object existence check is a single `Path::exists()` call
- No mismatch between our hash and DepotDownloader's hash

### Manifest txt format

```
Content Manifest for Depot 3321461

Manifest ID / date     : 3559081655545104676 / 03/22/2026 16:01:45
Total number of files  : 257
Total number of chunks : 130874
Total bytes on disk    : 133352312992
Total bytes compressed : 100116131120


          Size Chunks File SHA                                 Flags Name
       6740755      7 8a11847b3e22b2fb909b57787ed94d1bb139bcb2     0 0000/0.pamt
     912261088    896 3e6800918fef5f8880cf601e5b60bff031465e60     0 0000/0.paz
```

Fields (whitespace-split): `size chunks sha1 flags name`

The header block (lines before the column header) is skipped during parsing.

---

## Core Types (`rewind-core/src/cache.rs`)

```rust
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub name: String,       // relative path, e.g. "0000/0.paz"
    pub sha1: String,       // 40-char lowercase hex
    pub size_bytes: u64,
}
```

---

## Core Functions (`rewind-core/src/cache.rs`)

### `parse_manifest_txt`

```rust
pub fn parse_manifest_txt(path: &Path) -> Result<Vec<ManifestEntry>, CacheError>
```

Parses the human-readable manifest txt produced by `-manifest-only`. Skips all header lines (before the data rows). Each data row is whitespace-split: fields are `[size, chunks, sha1, flags, name]`.

### `missing_entries`

```rust
pub fn missing_entries<'a>(
    depot_dir: &Path,
    entries: &'a [ManifestEntry],
) -> Vec<&'a ManifestEntry>
```

Returns entries whose SHA1 is not present in `depot_dir/.objects/<sha1>`.

### `intern_object`

```rust
pub fn intern_object(
    depot_dir: &Path,
    src: &Path,
    sha1: &str,
) -> Result<PathBuf, CacheError>
```

Moves `src` into `depot_dir/.objects/<sha1>`. If the object already exists (concurrent or retry scenario), the source file is discarded. Returns the object path.

### `link_object`

```rust
pub fn link_object(
    depot_dir: &Path,
    sha1: &str,
    manifest_dir: &Path,
    name: &str,
) -> Result<(), CacheError>
```

Creates a symlink at `manifest_dir/<name>` pointing to the absolute path of `depot_dir/.objects/<sha1>`. Creates parent directories as needed. Overwrites an existing symlink at the same path. Absolute paths are used (consistent with the existing `create_symlink` behavior in `cache.rs`) to avoid depth-dependent relative path calculations for files in subdirectories.

---

## Download Flow Changes (`rewind-cli/src/main.rs`)

The existing `apply_downloaded()` call is **replaced** by a two-phase flow:

### Phase 1 — Pre-download

Both phases use the manifest cache dir (`cache/<app_id>/<depot_id>/<manifest_id>/`) as the `-dir` argument to DepotDownloader.

1. Run DepotDownloader with `-manifest-only -dir <manifest_cache_dir>` → produces `manifest_<depot>_<manifest>.txt` in that dir (no game files downloaded)
2. Parse with `parse_manifest_txt()`
3. Call `missing_entries()` against the depot's `.objects/`
4. **If nothing missing:** skip the full download entirely — proceed directly to Phase 2 step 2
5. **If some missing:** write their paths to a temp filelist, pass `-filelist <path> -dir <manifest_cache_dir>` to the real DepotDownloader invocation

### Phase 2 — Post-download

1. For each file in the filelist (downloaded into `<manifest_cache_dir>`): call `intern_object()` → moves it to `.objects/<sha1>`
2. For every entry in the manifest: call `link_object()` → creates symlink in manifest cache dir (whether the object was just interned or was already present)
3. The manifest txt is already saved at `<manifest_cache_dir>/manifest_<depot>_<manifest>.txt` from Phase 1 — no extra copy needed

### Unchanged

- `repoint_symlinks()` — unchanged; operates on manifest dir symlinks, which now point to `.objects/` instead of real files
- `restore_from_cache()` — unchanged; `std::fs::copy` follows symlinks, so content is correctly restored
- `cached_manifest_ids` tracking in `games.toml` — unchanged

---

## Testing

All tests in `rewind-core/src/cache.rs` using `tempfile::TempDir`.

### `parse_manifest_txt`
- Correctly formatted input → correct `ManifestEntry` fields (name, sha1, size)
- Header lines and blank lines are skipped
- Empty file → empty vec

### `missing_entries`
- All entries missing → returns all
- All entries present in `.objects/` → returns none
- Mixed → returns only missing ones

### `intern_object`
- Moves file to `.objects/<sha1>`, original path no longer exists
- If object already exists, does not error (idempotent)
- Returns the correct object path

### `link_object`
- Creates symlink at `manifest_dir/name` pointing to correct object
- Symlink target is readable (resolves correctly)
- Overwrites an existing symlink without error
- Creates intermediate directories for nested paths (e.g. `0000/0.paz`)

### Integration: full download flow simulation
- All files missing → all entries in filelist, all interned and linked
- All files present → empty filelist, symlinks created without any download
- Partial overlap → only missing files in filelist, existing ones linked directly

### Regression
- `repoint_symlinks` works when manifest dir contains symlinks into `.objects/`
- `restore_from_cache` correctly copies file content through the symlink chain
