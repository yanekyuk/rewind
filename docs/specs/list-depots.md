---
title: "List All Depots for an App"
type: spec
tags: [depot, sidecar, steamkit, pics, ipc, frontend]
created: 2026-03-31
updated: 2026-03-31
---

## Behavior

Add the ability to enumerate ALL depots for a Steam app (not just locally installed ones) by querying Steam's PICS API. This enables the UI to show non-installed depots alongside installed ones, giving users visibility into the full content structure of a game.

### 1. Sidecar: `list-depots` command

A new sidecar command that:
1. Authenticates with Steam (reuses existing session pattern)
2. Calls `PICSGetProductInfo()` for the given app ID
3. Iterates `depots.Children` to enumerate ALL depots (not just a single depot)
4. Returns each depot's ID and available metadata (name, max size, DLC app ID if present)

**Arguments:**
- `--username <user>` -- Steam username
- `--password <pass>` (optional) -- Steam password; falls back to saved session
- `--app <id>` -- App ID
- `--guard-code <code>` (optional) -- Steam Guard code if needed

**Output:**
```json
{"type":"depot_list","depots":[
  {"depot_id":3321461,"name":"Crimson Desert Content","max_size":133575233011,"dlc_app_id":null},
  {"depot_id":3321462,"name":"Crimson Desert DLC","max_size":5000000000,"dlc_app_id":3321470}
]}
```

### 2. Rust domain: `SteamDepotInfo` type

A new domain type representing a depot as reported by Steam (distinct from `DepotInfo` which represents a locally installed depot from ACF data):

```rust
pub struct SteamDepotInfo {
    pub depot_id: String,
    pub name: Option<String>,
    pub max_size: Option<u64>,
    pub dlc_app_id: Option<String>,
}
```

### 3. Rust infrastructure: `list_depots` function

New function in `infrastructure/depot_downloader.rs` that:
1. Spawns the sidecar with `list-depots --app <id>` and credentials
2. Collects NDJSON stdout
3. Parses the `depot_list` message type
4. Returns `Vec<SteamDepotInfo>`

### 4. Rust IPC: `list_depots` command

New Tauri command `list_depots(app_id: String)` that:
1. Gets credentials from AuthStore (or falls back to saved session)
2. Calls the infrastructure `list_depots` function
3. Returns `Vec<SteamDepotInfo>` to the frontend

### 5. Frontend: depot display in GameDetail

Update `GameDetail.tsx` to:
1. Call `list_depots` when the component mounts (with the game's app ID)
2. Merge installed depots (from `game.depots`) with all depots (from `list_depots`)
3. Display installed depots with full details (manifest, size) -- existing behavior
4. Display non-installed depots with different styling (greyed out, no manifest info)
5. All depots remain selectable for version browsing via `onChangeVersion`

### 6. Frontend types

New TypeScript type mirroring `SteamDepotInfo`:
```typescript
interface SteamDepotInfo {
  depot_id: string;
  name: string | null;
  max_size: number | null;
  dlc_app_id: string | null;
}
```

## Constraints

- The `list-depots` command follows the same NDJSON protocol as existing commands
- The new `SteamDepotInfo` domain type must not import from infrastructure or application layers
- Credentials handling must follow the same pattern as `list_manifests` (full creds or saved session fallback)
- `AUTH_REQUIRED` error must be detected and surfaced as `RewindError::AuthRequired`
- The sidecar must filter out non-content depots (e.g., workshop depots, SDK depots) based on depot metadata where possible
- Cross-platform: all changes must work on Linux, macOS, and Windows
- The frontend must gracefully handle the case where `list_depots` fails (show only installed depots)

## Acceptance Criteria

- [ ] Sidecar `list-depots` command enumerates all depots for a given app ID
- [ ] Sidecar output is valid NDJSON with `depot_list` message type
- [ ] `SteamDepotInfo` domain type defined in Rust with Serialize derive
- [ ] `list_depots` infrastructure function spawns sidecar and parses output
- [ ] `list_depots` IPC command available to frontend
- [ ] `SteamDepotInfo` TypeScript type defined in frontend
- [ ] `GameDetail` displays both installed and non-installed depots
- [ ] Non-installed depots have visually distinct styling
- [ ] All depots are selectable for version browsing
- [ ] Graceful fallback when `list_depots` call fails (shows installed depots only)
- [ ] Existing Rust tests pass
- [ ] New tests cover depot list parsing and domain type construction
