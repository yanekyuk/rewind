---
trigger: "Manifest diffing — domain-layer logic to compare two DepotManifest structs by SHA hash and produce a filelist of changed files for selective download. This is step 5 of the downgrade process."
type: feat
branch: feat/manifest-diff
base-branch: main
created: 2026-03-30
---

## Related Files
- src-tauri/src/domain/manifest/mod.rs — DepotManifest and ManifestEntry types (diff inputs)
- src-tauri/src/domain/manifest/parser.rs — manifest parser (reference for test patterns)
- src-tauri/src/domain/mod.rs — domain layer module declarations

## Relevant Docs
- docs/domain/downgrade-process.md — step 5 defines the diffing algorithm: compare files by SHA hash, generate filelist of changed files
- docs/domain/depotdownloader.md — DepotDownloader CLI uses -filelist flag with the diff output
- docs/specs/mvp-scope.md — "Manifest diffing" feature: fetch both manifests, parse, diff by SHA, generate filelist

## Related Issues
None — no related issues found.

## Scope

### Domain logic (src-tauri/src/domain/manifest/)
- Add a `diff` module (or `diff_manifests` function in mod.rs) that takes two `&DepotManifest` references (current and target)
- Compare entries by file name, using SHA hash to detect changes
- Produce a `ManifestDiff` result struct containing:
  - `changed`: files present in both manifests but with different SHA hashes (need re-download)
  - `added`: files in target manifest but not in current (new files to download)
  - `removed`: files in current manifest but not in target (files to delete during apply step)
- Generate a filelist (Vec<String> of file paths) suitable for DepotDownloader's `-filelist` flag — this should include `changed` + `added` files
- Use a HashMap keyed by file name for O(n) comparison rather than O(n²)

### Tests
- Identical manifests → empty diff
- Completely different manifests → all files changed
- Mixed scenario: some files same SHA, some different, some added, some removed
- Files with same name but different SHA → detected as changed
- Empty manifests → empty diff
- One empty manifest, one non-empty → all files added or removed
