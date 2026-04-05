# Version Labels for Cached Manifests — Design Spec

**Issue:** [#32](https://github.com/yanekyuk/rewind/issues/32)  
**Milestone:** 0.7.0 — Cache and manifest management

---

## Problem

The version picker and detail panel show raw manifest IDs (e.g. `7291048563840537431`), which are meaningless at a glance. Users maintaining multiple cached versions can't tell them apart without cross-referencing SteamDB.

---

## Goals

- Allow users to attach a short label to any cached manifest (e.g. "pre-nerf", "speedrun patch", "1.04")
- Display labels alongside manifest IDs in the version picker
- Persist labels in a way that can grow into a broader manifest metadata database over time

---

## Non-Goals

- Labels do not affect version switching logic
- No community/shared label import in this iteration
- No label on `active_manifest_id` or `latest_manifest_id` fields directly (those are in `games.toml`; labels live in the separate DB)

---

## Architecture

### New file: `manifests.toml`

A new file at `~/.local/share/rewind/manifests.toml` serves as a manifest metadata database, keyed globally by manifest ID.

```toml
[manifests.7291048563840537431]
label = "pre-nerf"

[manifests.8812034512345678901]
label = "1.04"
```

**Why separate from `games.toml`:**
- Zero migration cost — `games.toml` and its `cached_manifest_ids: Vec<String>` are untouched
- Manifest IDs are global (a depot manifest doesn't belong to one game entry)
- Future fields (`size_bytes`, `downloaded_at`, `source`) can be added to `ManifestMeta` without touching game config
- Enables future seeding from SteamDB data, community CSVs, or download history

### Core types (`rewind-core/src/manifest_db.rs`)

```rust
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestDb {
    #[serde(default)]
    pub manifests: HashMap<String, ManifestMeta>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestMeta {
    pub label: Option<String>,
    // future: pub size_bytes: Option<u64>,
    // future: pub downloaded_at: Option<DateTime<Utc>>,
    // future: pub source: Option<String>,
}
```

### Core functions (`rewind-core/src/manifest_db.rs`)

- `load_manifest_db() -> Result<ManifestDb, ConfigError>` — reads `manifests.toml`, returns default if missing
- `save_manifest_db(db: &ManifestDb) -> Result<(), ConfigError>` — writes `manifests.toml`
- `ManifestDb::get_label(&self, manifest_id: &str) -> Option<&str>`
- `ManifestDb::set_label(&mut self, manifest_id: &str, label: String)`
- `ManifestDb::clear_label(&mut self, manifest_id: &str)`

`ManifestDb` is exposed from `rewind-core`'s public API alongside `GameEntry` and `GamesConfig`.

---

## UI Changes (`rewind-cli`)

### Version picker rendering

When rendering each entry in the cached manifest list, the version picker reads from `AppState.manifest_db` and looks up each manifest ID:

- **With label:** `● pre-nerf  7291048563840537431  (installed)`
- **Without label:** `●  7291048563840537431  (installed)`

Label is shown first, left-aligned, in a distinct style (e.g. bold or accent color). Manifest ID follows in a dimmed style.

### Inline label editor

Keybinding `E` on a selected manifest opens an inline input bar at the bottom of the version picker screen (same visual register as a vim `:` command line):

```
Label: [pre-nerf_             ]   Enter to confirm · Esc to cancel
```

- Pre-populated with the existing label if one exists, empty otherwise
- `Enter` saves to `ManifestDb` and closes the bar
- `Esc` cancels with no change
- Empty input + `Enter` clears the label

### App state

`VersionPickerState` gains a `mode` field to track the inline editing mode:

```rust
pub enum VersionPickerMode {
    Browse,
    EditingLabel { input: String },
}
```

`ManifestDb` is loaded into `AppState` at startup (alongside `GamesConfig`) and written on label save.

---

## Data Flow

1. App starts → `load_manifest_db()` → stored in `AppState`
2. Version picker renders → looks up each manifest ID in `ManifestDb`
3. User presses `E` → `VersionPickerMode::EditingLabel` with pre-populated input
4. User confirms → `ManifestDb::set_label(...)` → `save_manifest_db(...)` → mode back to `Browse`

---

## Testing

- Unit tests in `rewind-core`: roundtrip serialize/deserialize of `ManifestDb`, `get_label`/`set_label`/`clear_label` behavior, missing file returns default
- UI logic: label edit mode transitions (browse → editing → browse), empty input clears label
- No integration tests needed for this feature (pure metadata, no filesystem side effects beyond the TOML file)
