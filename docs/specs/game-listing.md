---
title: "Game Listing"
type: spec
tags: [steam, game, listing, infrastructure, domain, ipc, acf]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Detect Steam installation paths across platforms, scan for installed games by parsing appmanifest ACF files, and expose the results to the frontend via a Tauri IPC command.

### Steam Path Detection (Infrastructure)

- Detect the default `steamapps/` directory per platform:
  - Linux: `~/.local/share/Steam/steamapps/`
  - macOS: `~/Library/Application Support/Steam/steamapps/`
  - Windows: `C:\Program Files (x86)\Steam\steamapps\`
- Parse `steamapps/libraryfolders.vdf` to discover additional Steam library folders (games installed on secondary drives).
- Return all discovered `steamapps/` paths as a list.

### Appmanifest Scanning (Infrastructure)

- For each discovered `steamapps/` directory, find all files matching `appmanifest_*.acf`.
- Read each file and parse it using the existing VDF parser + `AppState::from_vdf`.
- Skip files that fail to parse (log a warning, do not abort the scan).

### GameInfo Type (Domain)

- A serde-serializable struct representing a game for the frontend:
  - `appid: String` -- Steam application identifier
  - `name: String` -- human-readable game name
  - `buildid: String` -- currently installed build number
  - `installdir: String` -- game folder name under `steamapps/common/`
  - `depots: Vec<DepotInfo>` -- list of installed depots with manifest IDs
  - `install_path: String` -- full absolute path to the game directory
- `DepotInfo` struct: `depot_id: String`, `manifest: String`, `size: String`
- Conversion from `AppState` to `GameInfo` given a `steamapps/` base path.

### list_games IPC Command

- Async Tauri command registered in the invoke handler.
- Calls infrastructure to detect Steam paths and scan appmanifests.
- Converts each `AppState` to `GameInfo`.
- Returns `Vec<GameInfo>` serialized via serde.
- On failure (Steam not installed), returns an empty list or a descriptive error.

## Constraints

- All filesystem paths must be cross-platform compatible (Linux, macOS, Windows).
- No hardcoded paths -- detect using platform-specific home directory resolution.
- Use async Rust (tokio) for filesystem reads.
- Domain layer (`GameInfo`, `DepotInfo`) must not import from infrastructure.
- Infrastructure layer performs all I/O and implements scanning logic.
- Gracefully handle missing Steam installation -- do not panic.
- `GameInfo` and `DepotInfo` must derive `Serialize` for Tauri IPC serialization.

## Acceptance Criteria

1. `GameInfo` struct exists in the domain layer with all specified fields and derives `Serialize` + `Clone` + `Debug`.
2. `AppState` can be converted to `GameInfo` given a steamapps base path.
3. Steam path detection returns the correct default path for the current platform.
4. `libraryfolders.vdf` is parsed to discover additional library folders.
5. Appmanifest scanner finds and parses all `appmanifest_*.acf` files in a given directory.
6. Malformed ACF files are skipped without aborting the scan.
7. `list_games` Tauri command is registered and returns `Vec<GameInfo>`.
8. All tests pass with `cargo test` from the `src-tauri/` directory.
