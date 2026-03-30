---
title: "Downgrade Pipeline"
type: spec
tags: [downgrade, pipeline, manifest, download, acf, lock, sidecar, ipc]
created: 2026-03-31
updated: 2026-03-31
---

## Behavior

Orchestrate the core downgrade pipeline (steps 5-8 of the 9-step workflow) as a single `start_downgrade` Tauri IPC command that emits progress events to the frontend. The pipeline fetches manifests, diffs them, downloads changed files, applies the downgrade, patches the ACF manifest, and locks it.

### Pipeline phases

1. **Comparing** -- Fetch current and target manifests via the SteamKit sidecar's `get-manifest` command. Parse the JSON output into `DepotManifest` structs. Diff them using `diff_manifests`. Generate a filelist of changed + added files.

2. **Downloading** -- Write the filelist to a temp file. Spawn the sidecar's `download` command with the filelist and target manifest ID. Stream NDJSON progress events (`percent`, `bytes_downloaded`, `bytes_total`) to the frontend via Tauri event emission.

3. **Applying** -- Copy downloaded files from the temp download directory over the game's install directory. Delete files classified as "removed" in the diff. Patch the ACF manifest file per the rules in docs/domain/downgrade-process.md. Lock the ACF file using platform-specific immutability (chattr/chflags/readonly).

### Progress events

The backend emits Tauri events on the `downgrade-progress` channel:

| Event payload `phase` | Description |
|----------------------|-------------|
| `comparing` | Manifest fetch + diff in progress |
| `downloading` | File download in progress; includes `percent`, `bytes_downloaded`, `bytes_total` |
| `applying` | File copy, ACF patch, and lock in progress |
| `complete` | Pipeline finished successfully |
| `error` | Pipeline failed; includes `message` |

### Infrastructure functions

- `get_manifest(app, app_id, depot_id, manifest_id, credentials)` -- Spawn sidecar `get-manifest`, collect JSON output, parse into `DepotManifest`.
- `download(app, app_id, depot_id, manifest_id, dir, filelist_path, credentials, event_emitter)` -- Spawn sidecar `download`, stream progress events.
- `apply_downgrade(install_path, download_dir, removed_files)` -- Copy files, delete removed files.
- `patch_acf(acf_path, latest_buildid, latest_manifest, latest_size)` -- Read ACF, modify fields, write back.
- `lock_acf(acf_path)` -- Platform-specific immutability.

### JSON manifest parser

The sidecar's `get-manifest` command outputs a JSON object with `type: "manifest"`. Parse this into the existing `DepotManifest` struct. This is a new parser function (`parse_manifest_json`) separate from the existing text-based `parse_manifest`.

## Constraints

- Domain layer must not import from infrastructure or application layers
- Application layer must not import from infrastructure layer directly
- Orchestration happens in `lib.rs` Tauri command handlers (composition root)
- All filesystem paths must be cross-platform (Linux, macOS, Windows)
- Subprocess calls must handle non-zero exit codes and stderr output
- ACF patching sets buildid and manifest to LATEST values (not target), StateFlags=4, TargetBuildID=0, BytesToDownload=0
- Manifest locking: Linux uses `pkexec chattr +i`, macOS uses `chflags uchg` via osascript, Windows uses read-only attribute
- The `FullValidateAfterNextUpdate` field must be set to `0` if present

## Acceptance Criteria

1. `get_manifest` spawns sidecar with correct args and returns a parsed `DepotManifest`
2. `parse_manifest_json` correctly parses the sidecar's JSON manifest output into `DepotManifest`
3. `download` spawns sidecar with filelist and streams progress events
4. `apply_downgrade` copies files to install dir and deletes removed files
5. `patch_acf` modifies ACF fields correctly per the domain rules
6. `lock_acf` uses the correct platform-specific command
7. `start_downgrade` IPC command orchestrates all phases and emits progress events
8. Frontend displays progress for comparing, downloading, and applying phases
9. Frontend shows completion message with update preference reminder
10. Error in any phase produces an error event with a descriptive message
