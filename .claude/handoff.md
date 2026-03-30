---
trigger: "Implement the core downgrade pipeline: manifest diff, download changed files via sidecar, apply downgrade (patch ACF manifest, lock updates). Wire up the comparing, downloading, applying, and complete steps end-to-end."
type: feat
branch: feat/downgrade-pipeline
base-branch: main
created: 2026-03-31
version-bump: minor
---

## Related Files
- src-tauri/src/lib.rs — Tauri IPC commands (needs new commands for get-manifest, download, apply, lock)
- src-tauri/src/domain/manifest/diff.rs — ManifestDiff and diff_manifests (already implemented)
- src-tauri/src/domain/manifest/mod.rs — DepotManifest, ManifestEntry types
- src-tauri/src/domain/manifest/parser.rs — Manifest parsing from sidecar JSON output
- src-tauri/src/domain/vdf/acf.rs — AppState parsing and serialization (to_vdf for ACF patching)
- src-tauri/src/infrastructure/depot_downloader.rs — Sidecar interaction (login, list_manifests exist; needs get_manifest, download)
- src-tauri/src/infrastructure/sidecar.rs — spawn_sidecar helper
- src-tauri/src/infrastructure/steam.rs — Steam path detection, scan_appmanifests
- src-tauri/src/error.rs — RewindError enum
- sidecar/SteamKitSidecar/Commands/GetManifestCommand.cs — Sidecar get-manifest command (already implemented)
- sidecar/SteamKitSidecar/Commands/DownloadCommand.cs — Sidecar download command (already implemented)
- src/steps.ts — Step definitions (comparing, downloading, applying, complete steps)
- src/App.tsx — Step rendering (needs wiring for new steps)

## Relevant Docs
- docs/domain/downgrade-process.md — Full 9-step workflow with ACF patching rules
- docs/specs/manifest-diff.md — Manifest diffing algorithm and filelist generation
- docs/specs/mvp-scope.md — MVP feature set including download, apply, ACF patch, manifest lock
- docs/domain/steam-internals.md — Steam depot/manifest/ACF internals
- docs/domain/platform-differences.md — Platform-specific manifest locking (chattr, chflags, SetFileAttributes)

## Related Issues
None — no related issues found.

## Scope
Wire up the core downgrade pipeline (steps 5-8 of the 9-step workflow) end-to-end:

1. **Get manifest via sidecar** — Add `get_manifest` function in the infrastructure layer that spawns the sidecar with the `get-manifest` command, collects NDJSON output, and parses it into a `DepotManifest`. Needs two calls: one for the current manifest (from the installed game's depot info) and one for the target manifest (user-selected).

2. **Diff manifests** — Already implemented in `domain/manifest/diff.rs`. The orchestration layer needs to call `get_manifest` twice, then `diff_manifests`, then generate the filelist.

3. **Download changed files** — Add `download` function in the infrastructure layer that spawns the sidecar with the `download` command, a filelist file, and the target manifest ID. Must stream progress events (the sidecar emits NDJSON progress lines) back to the frontend via Tauri events.

4. **Apply downgrade** — Copy downloaded files over the game's install directory. Delete files classified as "removed" in the diff. Must verify Steam is not running before applying.

5. **Patch ACF manifest** — Edit `appmanifest_<appid>.acf` per the rules in docs/domain/downgrade-process.md: set buildid and manifest to the LATEST values (not target), set StateFlags=4, TargetBuildID=0, BytesToDownload=0. Use the existing `AppState::to_vdf()` for serialization.

6. **Lock ACF manifest** — Make the ACF file immutable using platform-specific methods:
   - Linux: `chattr +i` (requires privilege escalation)
   - macOS: `chflags uchg`
   - Windows: read-only attribute
   See docs/decisions/privilege-escalation.md for approach.

7. **Tauri IPC commands** — Expose the pipeline as commands: `start_downgrade` (orchestrates the full pipeline, emitting progress events) or granular commands for each step. The frontend needs to show progress for comparing, downloading, and applying phases.

8. **Frontend wiring** — Connect the comparing/downloading/applying/complete step views to the new backend commands. Show real-time download progress. Display completion message with the "set update preference" reminder.

Key constraint: The sidecar already handles authentication and CDN interaction. The Rust backend orchestrates the sidecar calls, file operations, and ACF patching. The frontend tracks progress via Tauri events.
