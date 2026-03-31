---
trigger: "Implement the start_downgrade IPC command and application-layer orchestration that wires together existing domain/infrastructure pieces into a full downgrade pipeline"
type: feat
branch: feat/downgrade-orchestration
base-branch: main
created: 2026-03-31
---

## Related Files
- src-tauri/src/lib.rs (IPC command registration)
- src-tauri/src/application/auth.rs (AuthStore for credentials)
- src-tauri/src/domain/downgrade.rs (DowngradeParams, DowngradeProgress)
- src-tauri/src/domain/manifest/mod.rs (DepotManifest, ManifestEntry)
- src-tauri/src/domain/manifest/diff.rs (diff_manifests, ManifestDiff, filelist())
- src-tauri/src/infrastructure/depot_downloader.rs (get_manifest, download)
- src-tauri/src/infrastructure/downgrade.rs (apply_files, delete_removed_files, patch_acf, lock_acf, is_steam_running)
- src-tauri/src/domain/vdf/acf.rs (AppState, AcfPatchParams, patch_for_downgrade)
- src-tauri/src/error.rs (RewindError)

## Relevant Docs
- docs/domain/downgrade-process.md
- docs/specs/downgrade-pipeline.md
- docs/domain/steamkit-sidecar.md

## Related Issues
None — no related issues found.

## Scope
Create `src-tauri/src/application/downgrade.rs` with orchestration logic and a `start_downgrade` Tauri IPC command. The pipeline has 4 phases:

1. **Comparing** — Emit `DowngradeProgress::Comparing`, fetch target + current manifests via `get_manifest()`, diff them with `diff_manifests()`, generate filelist
2. **Downloading** — Write filelist to temp file, call `download()` (already emits progress events)
3. **Applying** — Emit `DowngradeProgress::Applying`, call `apply_files()`, call `delete_removed_files()`, construct `AcfPatchParams` and call `patch_acf()`, call `lock_acf()`
4. **Complete** — Emit `DowngradeProgress::Complete` or `DowngradeProgress::Error`

Key constraints:
- Application layer must not import infrastructure directly — use the existing function signatures
- ACF path: `{steamapps_path}/appmanifest_{app_id}.acf`
- ACF patching uses LATEST values (not target) to trick Steam
- Use `std::env::temp_dir()` for download staging
- Check `is_steam_running()` before applying phase
- All progress emitted via `app.emit("downgrade-progress", ...)`
- Register command in lib.rs invoke_handler
