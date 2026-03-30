---
trigger: "Steam path detection + game listing — detect Steam installation across platforms, parse appmanifest ACF files using the VDF parser, and expose a list_games Tauri IPC command that returns installed games to the frontend."
type: feat
branch: feat/game-listing
base-branch: main
created: 2026-03-30
version-bump: minor
---

## Related Files
- src-tauri/src/infrastructure/mod.rs (infrastructure layer — Steam path detection and filesystem reads go here)
- src-tauri/src/domain/mod.rs (domain layer — may need new types for game listing)
- src-tauri/src/domain/vdf/acf.rs (AppState struct — already parses ACF into typed data)
- src-tauri/src/domain/vdf/parser.rs (VDF parser — used to parse appmanifest files)
- src-tauri/src/lib.rs (Tauri command registration — new IPC command goes here)
- src-tauri/src/error.rs (RewindError — infrastructure errors for path detection failures)
- src-tauri/Cargo.toml (may need dirs or home crate for cross-platform path resolution)

## Relevant Docs
- docs/domain/steam-internals.md (ACF format, Steam paths per platform, libraryfolders.vdf)
- docs/domain/platform-differences.md (platform-specific Steam paths and behaviors)
- docs/domain/downgrade-process.md (Steps 1-2: detect Steam, identify installed games)
- docs/decisions/layered-architecture.md (infrastructure implements domain interfaces, IPC boundary)
- docs/specs/mvp-scope.md (core flow starts with detect Steam → list installed games)

## Related Issues
None — no related issues found.

## Scope
Implement Steam path detection and game listing as the first full-stack feature. This covers Steps 1 and 2 of the downgrade process.

### Infrastructure layer (src-tauri/src/infrastructure/)
- **Steam path detection module**: Find the default steamapps directory per platform:
  - Linux: ~/.local/share/Steam/steamapps/
  - macOS: ~/Library/Application Support/Steam/steamapps/
  - Windows: C:\Program Files (x86)\Steam\steamapps\
- **Library folders detection**: Parse steamapps/libraryfolders.vdf to discover additional Steam library locations (games installed on secondary drives)
- **Appmanifest scanner**: Read all appmanifest_<appid>.acf files from each steamapps directory, parse them using the existing VDF parser + AppState struct
- Error handling: Return meaningful errors when Steam is not installed or paths are inaccessible

### Domain layer (src-tauri/src/domain/)
- **GameInfo struct**: A serde-serializable struct representing a game for the frontend:
  - appid, name, buildid, installdir
  - depots (list of depot IDs with their current manifest IDs)
  - install_path (full path to the game directory)
- Conversion from AppState → GameInfo

### Tauri IPC command
- **list_games command**: Async Tauri command that:
  1. Detects Steam installation paths
  2. Scans for appmanifest files
  3. Parses each into AppState → GameInfo
  4. Returns Vec<GameInfo> to the frontend
- Register in lib.rs invoke_handler

### Constraints
- All paths must be cross-platform compatible
- No hardcoded paths — detect or make configurable
- Use async Rust (tokio) for filesystem reads
- Gracefully handle missing Steam installation (return empty list or descriptive error, don't panic)
- GameInfo must derive Serialize for Tauri IPC serialization
