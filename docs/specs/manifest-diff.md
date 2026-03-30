---
title: "Manifest Diff"
type: spec
tags: [manifest, diff, domain, filelist, steamkit]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Compare two `DepotManifest` structs (current and target) by file name and SHA hash to identify which files need downloading, adding, or removing during a downgrade. Produce a `ManifestDiff` result and generate a filelist suitable for the SteamKit sidecar's `download` command with the `--filelist` option.

### Diffing algorithm

1. Build a `HashMap<&str, &ManifestEntry>` keyed by file name from the current manifest's entries
2. Iterate over the target manifest's entries:
   - If a file name exists in the current map with a **different SHA**, classify as **changed**
   - If a file name does **not** exist in the current map, classify as **added**
   - Remove matched entries from the current map
3. Remaining entries in the current map (not matched by any target entry) are **removed**

This yields O(n + m) comparison where n and m are the entry counts of the two manifests.

### Filelist generation

Produce a `Vec<String>` of file paths containing all **changed** and **added** files. These are the files the SteamKit sidecar needs to download via its `--filelist` option. **Removed** files are not included (they are handled during the apply step by deleting them from the game directory).

### Public API

```rust
pub fn diff_manifests(current: &DepotManifest, target: &DepotManifest) -> ManifestDiff

impl ManifestDiff {
    pub fn filelist(&self) -> Vec<String>
}
```

## Constraints

- Domain layer only -- no filesystem I/O, no infrastructure imports
- Operates on `&DepotManifest` references; caller handles manifest fetching and parsing
- Uses `std::collections::HashMap` for O(n) lookup -- no O(n^2) nested iteration
- File name comparison is exact (case-sensitive, full relative path)
- The `ManifestDiff` struct stores `ManifestEntry` clones for each category

## Acceptance Criteria

1. Identical manifests produce an empty diff (no changed, added, or removed files)
2. Completely different manifests (no shared file names) classify all current files as removed and all target files as added
3. Mixed scenario: files with same SHA are excluded, different SHA are changed, missing in current are added, missing in target are removed
4. Files with the same name but different SHA hashes are detected as changed
5. Two empty manifests produce an empty diff
6. One empty manifest and one non-empty manifest produce all-added or all-removed results
7. `filelist()` returns changed + added file names, excludes removed files
8. No filesystem I/O or infrastructure layer imports in the module
