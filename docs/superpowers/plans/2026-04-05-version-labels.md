# Version Labels for Cached Manifests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to attach short text labels to cached manifest IDs, stored in a new `manifests.toml` metadata database and displayed in the version picker.

**Architecture:** A new `ManifestDb` type in `rewind-core` owns a `HashMap<String, ManifestMeta>` keyed by manifest ID, persisted to `~/.local/share/rewind/manifests.toml`. `App` holds a `ManifestDb` loaded at startup. The version picker renders labels inline and opens a bottom-bar text editor on `E`.

**Tech Stack:** Rust, serde + toml (already in use), ratatui (already in use)

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `rewind-core/src/manifest_db.rs` | `ManifestDb`, `ManifestMeta`, load/save |
| Modify | `rewind-core/src/lib.rs` | expose `pub mod manifest_db` |
| Modify | `rewind-cli/src/app.rs` | add `VersionPickerMode`, `mode` field on `VersionPickerState`, `manifest_db` on `App`, update `App::new` |
| Modify | `rewind-cli/src/main.rs` | load manifest DB at startup, handle `E` + edit-mode keys |
| Modify | `rewind-cli/src/ui/version_picker.rs` | render user labels, inline editor bar, updated help text |

---

## Task 1: ManifestDb — core types and TOML I/O

**Files:**
- Create: `rewind-core/src/manifest_db.rs`
- Modify: `rewind-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add to the bottom of `rewind-core/src/manifest_db.rs` (create the file with just the tests first):

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::config::ConfigError;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestDb {
    #[serde(default)]
    pub manifests: HashMap<String, ManifestMeta>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestMeta {
    pub label: Option<String>,
}

impl ManifestDb {
    pub fn get_label(&self, manifest_id: &str) -> Option<&str> {
        self.manifests.get(manifest_id)?.label.as_deref()
    }

    pub fn set_label(&mut self, manifest_id: &str, label: String) {
        self.manifests
            .entry(manifest_id.to_string())
            .or_default()
            .label = Some(label);
    }

    pub fn clear_label(&mut self, manifest_id: &str) {
        if let Some(meta) = self.manifests.get_mut(manifest_id) {
            meta.label = None;
        }
    }
}

pub fn load_manifest_db() -> Result<ManifestDb, ConfigError> {
    let path = crate::config::data_dir()?.join("manifests.toml");
    if !path.exists() {
        return Ok(ManifestDb::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_manifest_db(db: &ManifestDb) -> Result<(), ConfigError> {
    let path = crate::config::data_dir()?.join("manifests.toml");
    let content = toml::to_string_pretty(db)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_label_returns_none_when_no_entry() {
        let db = ManifestDb::default();
        assert!(db.get_label("12345").is_none());
    }

    #[test]
    fn set_and_get_label_roundtrip() {
        let mut db = ManifestDb::default();
        db.set_label("12345", "pre-nerf".to_string());
        assert_eq!(db.get_label("12345"), Some("pre-nerf"));
    }

    #[test]
    fn clear_label_removes_it() {
        let mut db = ManifestDb::default();
        db.set_label("12345", "pre-nerf".to_string());
        db.clear_label("12345");
        assert!(db.get_label("12345").is_none());
    }

    #[test]
    fn clear_label_on_missing_entry_is_noop() {
        let mut db = ManifestDb::default();
        db.clear_label("nonexistent"); // must not panic
    }

    #[test]
    fn toml_roundtrip_preserves_labels() {
        let mut db = ManifestDb::default();
        db.set_label("7291048563840537431", "pre-nerf".to_string());
        db.set_label("8812034512345678901", "1.04".to_string());

        let serialized = toml::to_string_pretty(&db).unwrap();
        let parsed: ManifestDb = toml::from_str(&serialized).unwrap();

        assert_eq!(parsed.get_label("7291048563840537431"), Some("pre-nerf"));
        assert_eq!(parsed.get_label("8812034512345678901"), Some("1.04"));
    }

    #[test]
    fn missing_file_returns_default() {
        // load_manifest_db uses data_dir() which we can't easily redirect in tests,
        // but we can verify that an empty TOML string deserializes to default.
        let db: ManifestDb = toml::from_str("").unwrap();
        assert!(db.manifests.is_empty());
    }
}
```

- [ ] **Step 2: Expose the module from lib.rs**

In `rewind-core/src/lib.rs`, add one line:

```rust
pub mod manifest_db;
```

(Insert it alphabetically between `pub mod image_cache;` and `pub mod immutability;`.)

- [ ] **Step 3: Run tests to verify they pass**

```bash
cargo test -p rewind-core manifest_db
```

Expected: all 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rewind-core/src/manifest_db.rs rewind-core/src/lib.rs
git commit -m "feat(core): add ManifestDb type with TOML persistence"
```

---

## Task 2: Wire ManifestDb into App state

**Files:**
- Modify: `rewind-cli/src/app.rs`
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Add VersionPickerMode to app.rs**

In `rewind-cli/src/app.rs`, add this enum above `VersionPickerState`:

```rust
#[derive(Debug, Default, PartialEq)]
pub enum VersionPickerMode {
    #[default]
    Browse,
    EditingLabel { input: String },
}
```

- [ ] **Step 2: Add mode field to VersionPickerState**

Replace the existing `VersionPickerState` struct:

```rust
#[derive(Debug, Default)]
pub struct VersionPickerState {
    pub selected_index: usize,
    /// Set when Steam is detected running on screen open.
    pub steam_warning: bool,
    /// Set when an operation is blocked (e.g. Steam still running).
    pub error: Option<String>,
    pub mode: VersionPickerMode,
}
```

- [ ] **Step 3: Add manifest_db field to App and update App::new**

At the top of `rewind-cli/src/app.rs`, add the import:

```rust
use rewind_core::manifest_db::ManifestDb;
```

In the `App` struct, add the field after `games_config`:

```rust
pub manifest_db: ManifestDb,
```

Update `App::new` signature and body — replace:

```rust
pub fn new(config: Config, games_config: GamesConfig) -> Self {
    let first_run = config.steam_username.is_none() && config.libraries.is_empty();
    App {
        screen: if first_run { Screen::FirstRun } else { Screen::Main },
        config,
        games_config,
```

with:

```rust
pub fn new(config: Config, games_config: GamesConfig, manifest_db: ManifestDb) -> Self {
    let first_run = config.steam_username.is_none() && config.libraries.is_empty();
    App {
        screen: if first_run { Screen::FirstRun } else { Screen::Main },
        config,
        games_config,
        manifest_db,
```

- [ ] **Step 4: Load manifest_db at startup in main.rs**

In `rewind-cli/src/main.rs`, replace:

```rust
let cfg = config::load_config().unwrap_or_default();
let games_cfg = config::load_games().unwrap_or_default();
run(cfg, games_cfg).await
```

with:

```rust
let cfg = config::load_config().unwrap_or_default();
let games_cfg = config::load_games().unwrap_or_default();
let manifest_db = rewind_core::manifest_db::load_manifest_db().unwrap_or_default();
run(cfg, games_cfg, manifest_db).await
```

And update the `run` function signature — replace:

```rust
async fn run(
    cfg: rewind_core::config::Config,
    games_cfg: rewind_core::config::GamesConfig,
) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(cfg, games_cfg);
```

with:

```rust
async fn run(
    cfg: rewind_core::config::Config,
    games_cfg: rewind_core::config::GamesConfig,
    manifest_db: rewind_core::manifest_db::ManifestDb,
) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(cfg, games_cfg, manifest_db);
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p rewind-cli
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/app.rs rewind-cli/src/main.rs
git commit -m "feat(cli): wire ManifestDb into App state"
```

---

## Task 3: Render labels and inline editor in version picker

**Files:**
- Modify: `rewind-cli/src/ui/version_picker.rs`

- [ ] **Step 1: Update layout to include editor row**

Replace the existing layout block (the `let layout = Layout::default()...` section) with:

```rust
let editing = matches!(
    app.version_picker_state.mode,
    app::VersionPickerMode::EditingLabel { .. }
);
let editor_height: u16 = if editing { 1 } else { 0 };

let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(info_height), // warning / error line
        Constraint::Min(0),              // version list
        Constraint::Length(editor_height), // inline label editor
        Constraint::Length(1),           // help bar
    ])
    .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));
```

- [ ] **Step 2: Update list item rendering to show user labels**

Replace the `let items: Vec<ListItem> = cached.iter().enumerate().map(...)` block with:

```rust
let items: Vec<ListItem> = cached
    .iter()
    .enumerate()
    .map(|(i, manifest_id)| {
        let is_active = manifest_id == active;
        let is_latest = manifest_id == latest;

        let user_label = app.manifest_db.get_label(manifest_id);
        let display = match user_label {
            Some(lbl) => format!("{lbl}  {manifest_id}"),
            None => manifest_id.clone(),
        };

        let label = match (is_active, is_latest) {
            (true, true) => format!("● {display} (installed) (latest)"),
            (true, false) => format!("● {display} (installed)"),
            (false, true) => format!("  {display} (latest)"),
            (false, false) => format!("  {display}"),
        };

        let style = if i == app.version_picker_state.selected_index {
            theme::selected()
        } else if is_active {
            theme::status_success()
        } else {
            theme::text()
        };

        ListItem::new(label).style(style)
    })
    .collect();
```

- [ ] **Step 3: Render inline editor bar**

After the `f.render_widget(list, layout[1]);` line, add:

```rust
if let app::VersionPickerMode::EditingLabel { input } = &app.version_picker_state.mode {
    let bar = Paragraph::new(format!(" Label: {}█", input))
        .style(theme::text());
    f.render_widget(bar, layout[2]);
}
```

- [ ] **Step 4: Update help bar text and index**

Replace the help bar line (it was `layout[2]`, now it's `layout[3]`):

```rust
let help_text = if editing {
    " [Enter] confirm   [Esc] cancel "
} else {
    " [↑↓] select   [Enter] switch   [E] label   [Esc] cancel "
};
let help = Paragraph::new(help_text).style(theme::help_bar());
f.render_widget(help, layout[3]);
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p rewind-cli
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/ui/version_picker.rs
git commit -m "feat(cli): render version labels and inline editor in version picker"
```

---

## Task 4: Key handling for label editing

**Files:**
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Replace handle_version_picker with mode-aware version**

Replace the entire `fn handle_version_picker(app: &mut App, key: KeyCode)` function with:

```rust
fn handle_version_picker(app: &mut App, key: KeyCode) {
    // --- Editing mode: intercept all keys ---
    if matches!(app.version_picker_state.mode, app::VersionPickerMode::EditingLabel { .. }) {
        match key {
            KeyCode::Esc => {
                app.version_picker_state.mode = app::VersionPickerMode::Browse;
            }
            KeyCode::Enter => {
                let input = match &app.version_picker_state.mode {
                    app::VersionPickerMode::EditingLabel { input } => input.trim().to_string(),
                    _ => unreachable!(),
                };
                let manifest_id = app
                    .selected_game_entry()
                    .and_then(|e| {
                        e.cached_manifest_ids.get(app.version_picker_state.selected_index)
                    })
                    .cloned();
                if let Some(id) = manifest_id {
                    if input.is_empty() {
                        app.manifest_db.clear_label(&id);
                    } else {
                        app.manifest_db.set_label(&id, input);
                    }
                    let _ = rewind_core::manifest_db::save_manifest_db(&app.manifest_db);
                }
                app.version_picker_state.mode = app::VersionPickerMode::Browse;
            }
            KeyCode::Backspace => {
                if let app::VersionPickerMode::EditingLabel { input } =
                    &mut app.version_picker_state.mode
                {
                    input.pop();
                }
            }
            KeyCode::Char(c) => {
                if let app::VersionPickerMode::EditingLabel { input } =
                    &mut app.version_picker_state.mode
                {
                    input.push(c);
                }
            }
            _ => {}
        }
        return;
    }

    // --- Browse mode ---
    let cached_len = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.len())
        .unwrap_or(0);

    match key {
        KeyCode::Esc => app.screen = Screen::Main,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.version_picker_state.selected_index > 0 {
                app.version_picker_state.selected_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.version_picker_state.selected_index + 1 < cached_len {
                app.version_picker_state.selected_index += 1;
            }
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let existing = app
                .selected_game_entry()
                .and_then(|e| {
                    e.cached_manifest_ids.get(app.version_picker_state.selected_index)
                })
                .and_then(|id| app.manifest_db.get_label(id))
                .unwrap_or("")
                .to_string();
            app.version_picker_state.mode =
                app::VersionPickerMode::EditingLabel { input: existing };
        }
        KeyCode::Enter => {
            let target_manifest = app
                .selected_game_entry()
                .and_then(|e| e.cached_manifest_ids.get(app.version_picker_state.selected_index))
                .cloned();

            if let Some(manifest_id) = target_manifest {
                let is_current = app
                    .selected_game_entry()
                    .map(|e| e.active_manifest_id == manifest_id)
                    .unwrap_or(false);

                if is_current {
                    app.screen = Screen::Main;
                    return;
                }

                if rewind_core::steam_guard::is_steam_running() {
                    app.version_picker_state.error =
                        Some("Steam is running. Quit Steam before switching versions.".into());
                    return;
                }

                let is_latest = app
                    .selected_game_entry()
                    .map(|e| e.latest_manifest_id == manifest_id)
                    .unwrap_or(false);

                let mut steps = vec![
                    (app::StepKind::RepointSymlinks, app::StepStatus::Pending),
                    (app::StepKind::PatchAcf, app::StepStatus::Pending),
                ];
                if is_latest {
                    steps.push((app::StepKind::LockAcf, app::StepStatus::Done));
                } else {
                    steps.push((app::StepKind::LockAcf, app::StepStatus::Pending));
                }

                app.switch_overlay_state = app::SwitchOverlayState {
                    steps,
                    target_manifest: manifest_id.clone(),
                    done: false,
                    lock_skipped: is_latest,
                };
                app.screen = Screen::SwitchOverlay;

                switch_to_cached_version(app, manifest_id, is_latest);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check -p rewind-cli
```

Expected: no errors.

- [ ] **Step 3: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Manual smoke test**

```bash
cargo run
```

- Open the version picker on a game with cached manifests (`V` key)
- Press `E` — confirm the label bar appears at the bottom
- Type a label (e.g. `pre-nerf`) — confirm characters appear
- Press `Enter` — confirm bar closes and the label is shown before the manifest ID
- Press `E` again — confirm the bar is pre-populated with `pre-nerf`
- Clear the input and press `Enter` — confirm the label disappears
- Press `Esc` while editing — confirm bar closes with no change

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/main.rs
git commit -m "feat(cli): handle E key for version label editing"
```
