---
title: "SteamKit2 Sidecar Migration"
type: spec
tags: [steamkit, sidecar, tauri, infrastructure, auth, manifest, depot, steam]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Replace the DepotDownloader sidecar with a custom .NET console application built on SteamKit2. The new sidecar communicates via newline-delimited JSON (NDJSON) on stdout, eliminating fragile text parsing.

### 1. .NET Sidecar Application (SteamKitSidecar)

A new .NET 8 console app in `sidecar/` at the project root. It uses SteamKit2 (LGPL v2.1) to interact with Steam directly.

**Commands** (passed as CLI subcommands):

- `login` -- Authenticate with Steam using username/password. Handles Steam Guard 2FA (email + mobile authenticator) via JSON prompt/response on stdin/stdout. Persists session tokens to a local file for reuse.
- `list-manifests --app <id> --depot <id>` -- Enumerate historical manifest IDs for a given app/depot. Returns JSON array of manifest entries with ID and date.
- `get-manifest --app <id> --depot <id> --manifest <id>` -- Download and parse a specific manifest. Returns JSON with file listings (name, SHA, size, chunks).
- `download --app <id> --depot <id> --manifest <id> [--filelist <path>]` -- Download depot files for a specific manifest via Steam CDN. Reports progress via JSON events.

**Output protocol** (NDJSON on stdout):

```json
{"type":"log","level":"info","message":"Connected to Steam"}
{"type":"guard_prompt","method":"email","hint":"t***@example.com"}
{"type":"auth_success","session_file":"/path/to/session"}
{"type":"manifest_list","manifests":[{"id":"123456","date":"2026-03-22T16:01:45Z"}]}
{"type":"manifest","depot_id":3321461,"manifest_id":"123456","files":[...]}
{"type":"progress","percent":45.2,"bytes_downloaded":1024000,"bytes_total":2048000}
{"type":"error","code":"AUTH_FAILED","message":"Invalid credentials"}
{"type":"done","success":true}
```

Each line is a complete JSON object. The Rust infrastructure layer reads lines and deserializes by `type` field.

### 2. Rust Infrastructure Layer Changes

- **`infrastructure/sidecar.rs`** -- Replace DepotDownloader spawning with SteamKitSidecar spawning. The sidecar binary name changes from `DepotDownloader` to `SteamKitSidecar`. Update the `SIDECAR_NAME` constant and spawn function.
- **`infrastructure/depot_downloader.rs`** -- Rename to `infrastructure/steam_operations.rs`. Replace text parsing with JSON deserialization of NDJSON output. Each operation (list_manifests, get_manifest, download) reads lines from stdout and deserializes them into typed Rust structs.
- Remove `is_guard_prompt`, `write_guard_code`, and `build_authenticated_args` -- auth is handled by the sidecar natively (login command + session persistence).

### 3. Domain Layer Changes

- **`domain/auth.rs`** -- Remove `to_depot_args()` method from `Credentials`. The domain layer should not know about CLI argument formatting. Add a method or type for JSON-serializable credential submission instead.
- **`domain/manifest/list_parser.rs`** -- Remove entirely. Manifest lists are now JSON-deserialized, not text-parsed.
- **`domain/manifest/parser.rs`** -- Remove entirely. Manifests are now JSON-deserialized.
- **`domain/manifest/mod.rs`** -- Update types to work with JSON deserialization. Add `Deserialize` derive to `ManifestListEntry`, `DepotManifest`, `ManifestEntry`. The types themselves remain the same (they represent Steam concepts, not DepotDownloader output).
- **`domain/manifest/diff.rs`** -- No changes needed. Diffing operates on domain types.

### 4. Domain Sidecar Protocol Types

Add a new `domain/sidecar.rs` module defining the NDJSON message types:

```rust
enum SidecarMessage {
    Log { level: String, message: String },
    GuardPrompt { method: String, hint: Option<String> },
    AuthSuccess { session_file: String },
    ManifestList { manifests: Vec<ManifestListEntry> },
    Manifest { depot_id: u64, manifest_id: String, files: Vec<ManifestEntry> },
    Progress { percent: f64, bytes_downloaded: u64, bytes_total: u64 },
    Error { code: String, message: String },
    Done { success: bool },
}
```

### 5. Build System Changes

- **`scripts/download-sidecar.sh`** -- Replace with `scripts/build-sidecar.sh` that builds the .NET project and copies the output to `src-tauri/binaries/`.
- **`package.json`** -- Update `ensure-sidecar` script to call the new build script.
- **`tauri.conf.json`** -- Change `externalBin` from `DepotDownloader` to `SteamKitSidecar`.
- **`.gitignore`** -- Keep `src-tauri/binaries/` ignored. Add .NET build output directories (`sidecar/bin/`, `sidecar/obj/`).

### 6. Licensing Changes

- Remove `src-tauri/resources/DEPOTDOWNLOADER-LICENSE` (GPL-2.0).
- Remove `resources` entry from `tauri.conf.json`.
- Add SteamKit2 LGPL v2.1 attribution to appropriate location.

### 7. Frontend Changes

- **`src/hooks/useManifestList.ts`** -- No changes needed. The IPC interface (`list_manifests` command) remains the same; only the backend implementation changes.
- **`src/hooks/useAuth.ts`** -- No changes needed. Auth flow remains: submit credentials via IPC, backend stores and uses them.
- **`src/types/manifest.ts`** -- No changes needed. `ManifestListEntry` type is unchanged.

### 8. IPC Command Changes

- **`list_manifests`** -- Internal implementation switches from spawning DepotDownloader + text parsing to spawning SteamKitSidecar + JSON deserialization. The command signature (app_id, depot_id) and return type (Vec<ManifestListEntry>) remain identical.

## Constraints

- The sidecar binary must be self-contained (include .NET runtime) for each target platform.
- The sidecar binary must not be committed to git (each is ~60-80 MB).
- All sidecar communication must use NDJSON on stdout. No text parsing.
- Credentials must never be logged or persisted insecurely. Session tokens are stored in a platform-appropriate location.
- Infrastructure layer owns all sidecar interaction (per layered architecture).
- Domain types must not import from infrastructure or application layers.
- Cross-platform: Linux (x86_64), macOS (x86_64, arm64), Windows (x86_64).
- The .NET sidecar project must be buildable with `dotnet publish` for self-contained deployment.

## Acceptance Criteria

- [ ] `sidecar/` contains a .NET 8 console app with SteamKit2 dependency
- [ ] Sidecar supports `login`, `list-manifests`, `get-manifest`, and `download` commands
- [ ] All sidecar output is NDJSON (one JSON object per line)
- [ ] Steam Guard 2FA flow works via JSON prompt/response
- [ ] `domain/sidecar.rs` defines typed message types for the NDJSON protocol
- [ ] `domain/manifest/list_parser.rs` and `domain/manifest/parser.rs` are removed
- [ ] `domain/auth.rs` no longer has `to_depot_args()` method
- [ ] `infrastructure/sidecar.rs` spawns SteamKitSidecar instead of DepotDownloader
- [ ] `infrastructure/steam_operations.rs` replaces `depot_downloader.rs` with JSON-based operations
- [ ] `tauri.conf.json` references SteamKitSidecar in `externalBin`
- [ ] `scripts/build-sidecar.sh` builds the .NET project for the current platform
- [ ] `package.json` `ensure-sidecar` calls the new build script
- [ ] DepotDownloader GPL-2.0 license file is removed
- [ ] All existing Rust tests pass (updated or replaced as needed)
- [ ] `list_manifests` IPC command works end-to-end with the new sidecar
- [ ] Frontend hooks require no changes (backward-compatible IPC interface)
