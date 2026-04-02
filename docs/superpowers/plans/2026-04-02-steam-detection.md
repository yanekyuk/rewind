# Steam Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect if Steam is running and warn users on screen entry and hard-block operations that modify Steam's ACF manifest files.

**Architecture:** A new `steam_guard` module in `rewind-core` exposes `is_steam_running()` using the `sysinfo` crate. The CLI calls this on screen entry (warning only) and again at operation start (hard block + inline error).

**Tech Stack:** `sysinfo` crate (process enumeration), ratatui (inline warning/error rendering), existing `theme::status_warning()` / `theme::status_error()` styles.

---

## File Map

| File | Change |
|------|--------|
| `rewind-core/Cargo.toml` | Add `sysinfo` dependency |
| `rewind-core/src/steam_guard.rs` | **Create** — `is_steam_running()` |
| `rewind-core/src/lib.rs` | Export `steam_guard` module |
| `rewind-cli/src/app.rs` | Add `steam_warning: bool` to `DowngradeWizardState` and `VersionPickerState`; add `error: Option<String>` to `VersionPickerState` |
| `rewind-cli/src/main.rs` | Set warning on screen entry; hard block in `start_download` and `handle_version_picker` Enter |
| `rewind-cli/src/ui/downgrade_wizard.rs` | Render steam_warning line in input view |
| `rewind-cli/src/ui/version_picker.rs` | Render steam_warning and error lines |

---

### Task 1: Implement `is_steam_running()` in rewind-core

**Files:**
- Modify: `rewind-core/Cargo.toml`
- Create: `rewind-core/src/steam_guard.rs`
- Modify: `rewind-core/src/lib.rs`

- [ ] **Step 1: Add `sysinfo` to `rewind-core/Cargo.toml`**

In `rewind-core/Cargo.toml`, add to `[dependencies]`:

```toml
sysinfo = "0.33"
```

- [ ] **Step 2: Write the failing test**

Create `rewind-core/src/steam_guard.rs` with just the test:

```rust
pub fn is_steam_running() -> bool {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_guard_does_not_panic() {
        // Can't mock the process list, but we verify the function
        // completes without panicking and returns a bool.
        let _ = is_steam_running();
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p rewind-core steam_guard
```

Expected: FAIL with a panic from `todo!()`

- [ ] **Step 4: Implement `is_steam_running()`**

Replace the `todo!()` body in `rewind-core/src/steam_guard.rs`:

```rust
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub fn is_steam_running() -> bool {
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new()),
    );
    sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_lowercase();
        // "steam" covers Linux and macOS; "steam.exe" covers Windows
        name == "steam" || name == "steam.exe"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_guard_does_not_panic() {
        let _ = is_steam_running();
    }
}
```

- [ ] **Step 5: Export the module from `rewind-core/src/lib.rs`**

Add one line to `rewind-core/src/lib.rs`:

```rust
pub mod cache;
pub mod config;
pub mod depot;
pub mod image_cache;
pub mod immutability;
pub mod patcher;
pub mod scanner;
pub mod steam_guard;
pub mod steamdb;
```

- [ ] **Step 6: Run test to verify it passes**

```bash
cargo test -p rewind-core steam_guard
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add rewind-core/Cargo.toml rewind-core/src/steam_guard.rs rewind-core/src/lib.rs
git commit -m "feat: add is_steam_running() to rewind-core"
```

---

### Task 2: Extend app state structs

**Files:**
- Modify: `rewind-cli/src/app.rs`

- [ ] **Step 1: Add `steam_warning` to `DowngradeWizardState`**

In `rewind-cli/src/app.rs`, change `DowngradeWizardState`:

```rust
#[derive(Debug, Default)]
pub struct DowngradeWizardState {
    pub manifest_input: String,
    pub app_id: u32,
    pub depot_id: u32,
    pub is_downloading: bool,
    pub error: Option<String>,
    pub error_url: Option<String>,
    pub steps: Vec<(StepKind, StepStatus)>,
    pub depot_lines: Vec<String>,
    pub prompt_input: Option<String>,
    pub prompt_label: Option<String>,
    /// Set when Steam is detected running on wizard open.
    pub steam_warning: bool,
}
```

- [ ] **Step 2: Add `steam_warning` and `error` to `VersionPickerState`**

In `rewind-cli/src/app.rs`, change `VersionPickerState`:

```rust
#[derive(Debug, Default)]
pub struct VersionPickerState {
    pub selected_index: usize,
    /// Set when Steam is detected running on screen open.
    pub steam_warning: bool,
    /// Set when an operation is blocked (e.g. Steam still running).
    pub error: Option<String>,
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check -p rewind-cli
```

Expected: no errors (all usages of these structs use named fields or `..Default::default()`, so new `bool`/`Option` fields default to `false`/`None`)

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/app.rs
git commit -m "feat: add steam_warning and error fields to wizard and picker state"
```

---

### Task 3: Wire checks into main.rs logic

**Files:**
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Check Steam on wizard screen entry (handle_main `d` key)**

In `rewind-cli/src/main.rs`, find the `KeyCode::Char('d')` arm in `handle_main` (around line 322). Replace it:

```rust
KeyCode::Char('d') => {
    if let Some(g) = app.selected_game() {
        let steam_running = rewind_core::steam_guard::is_steam_running();
        app.wizard_state = DowngradeWizardState {
            app_id: g.app_id,
            depot_id: g.depot_id,
            steam_warning: steam_running,
            ..Default::default()
        };
        app.screen = Screen::DowngradeWizard;
    }
}
```

- [ ] **Step 2: Check Steam on version picker screen entry (handle_main `u` key)**

Find the `KeyCode::Char('u')` arm in `handle_main` (around line 332). Replace it:

```rust
KeyCode::Char('u') => {
    if app.selected_game_entry().map(|e| e.cached_manifest_ids.len() > 1).unwrap_or(false) {
        let steam_running = rewind_core::steam_guard::is_steam_running();
        app.version_picker_state = app::VersionPickerState {
            selected_index: 0,
            steam_warning: steam_running,
            error: None,
        };
        app.screen = Screen::VersionPicker;
    }
}
```

- [ ] **Step 3: Hard block in `start_download`**

Find `fn start_download(app: &mut App)` (around line 594). Insert the Steam check immediately after the `steam_username` check:

```rust
fn start_download(app: &mut App) {
    use crate::app::{StepKind, StepStatus};

    if app.config.steam_username.is_none() {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    if rewind_core::steam_guard::is_steam_running() {
        app.wizard_state.error = Some("Steam is running. Quit Steam before downloading.".into());
        return;
    }

    // ... rest of function unchanged
```

- [ ] **Step 4: Hard block in `handle_version_picker` Enter**

Find the `KeyCode::Enter` arm in `handle_version_picker` (around line 481). Insert the Steam check before the overlay is built — immediately after the `is_current` early-return:

```rust
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

        // ... rest unchanged (steps setup, screen transition, switch_to_cached_version call)
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p rewind-cli
```

Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/main.rs
git commit -m "feat: check Steam running on screen entry and block operations"
```

---

### Task 4: Render warnings and errors in the UI

**Files:**
- Modify: `rewind-cli/src/ui/downgrade_wizard.rs`
- Modify: `rewind-cli/src/ui/version_picker.rs`

- [ ] **Step 1: Add steam warning line to the downgrade wizard input view**

In `rewind-cli/src/ui/downgrade_wizard.rs`, find `fn draw_input_view` (around line 38). Replace the layout definition to add a conditional warning row between guidance and the input box:

```rust
fn draw_input_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let warn_height: u16 = if app.wizard_state.steam_warning { 1 } else { 0 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),           // guidance text
            Constraint::Length(warn_height), // steam warning (0 if not needed)
            Constraint::Length(3),           // manifest input
            Constraint::Min(0),              // error/output log
            Constraint::Length(1),           // help line
        ])
        .split(area);

    // Guidance text
    let guidance = Paragraph::new(
        " Find your target version:\n \
         [P] Patches  \u{2014} browse patch notes to find the version you want,\n \
                        note its date\n \
         [M] Manifests \u{2014} match the date to find the corresponding\n \
                        manifest ID",
    )
    .style(theme::text());
    f.render_widget(guidance, layout[0]);

    // Steam warning
    if app.wizard_state.steam_warning {
        let warn = Paragraph::new(" \u{26a0} Steam is running. Quit Steam before downloading.")
            .style(theme::status_warning());
        f.render_widget(warn, layout[1]);
    }

    // Manifest ID input
    let cursor = if !app.wizard_state.is_downloading {
        "\u{2588}"
    } else {
        ""
    };
    let input_style = if app.wizard_state.is_downloading {
        theme::input_inactive()
    } else {
        theme::input_active()
    };
    let input_block = Block::default()
        .title(" Manifest ID ")
        .borders(Borders::ALL)
        .border_style(theme::border_focused());
    let input_para =
        Paragraph::new(format!("{}{}", app.wizard_state.manifest_input, cursor))
            .style(input_style)
            .block(input_block);
    f.render_widget(input_para, layout[2]);

    // Error / output log
    let (log_title, log_border_style) = if app.wizard_state.error.is_some() {
        (" Error ", Style::default().fg(theme::ERROR))
    } else {
        (" Output ", theme::border())
    };

    let log_items: Vec<ListItem> = if let Some(err) = &app.wizard_state.error {
        vec![ListItem::new(err.as_str()).style(Style::default().fg(theme::ERROR))]
    } else {
        vec![]
    };

    let log_block = Block::default()
        .title(log_title)
        .borders(Borders::ALL)
        .border_style(log_border_style);
    let log_list = List::new(log_items).block(log_block);
    f.render_widget(log_list, layout[3]);

    // Help line
    let help_text = if app.wizard_state.error_url.is_some() {
        " [O] open download page   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [P] patches   [M] manifests   [Enter] download   [Esc] cancel "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[4]);
}
```

- [ ] **Step 2: Add steam warning and error rendering to the version picker**

In `rewind-cli/src/ui/version_picker.rs`, replace the layout and rendering (full `draw` function body after the block setup):

```rust
pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let has_info = app.version_picker_state.steam_warning
        || app.version_picker_state.error.is_some();
    let info_height: u16 = if has_info { 1 } else { 0 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(info_height), // warning / error line
            Constraint::Min(0),              // version list
            Constraint::Length(1),           // help bar
        ])
        .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));

    // Warning / error line
    if app.version_picker_state.steam_warning {
        let warn = Paragraph::new(" \u{26a0} Steam is running. Quit Steam before switching.")
            .style(theme::status_warning());
        f.render_widget(warn, layout[0]);
    } else if let Some(err) = &app.version_picker_state.error {
        let err_para = Paragraph::new(format!(" {}", err))
            .style(theme::status_error());
        f.render_widget(err_para, layout[0]);
    }

    let cached = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.as_slice())
        .unwrap_or(&[]);

    if cached.is_empty() {
        let msg = Paragraph::new("No cached versions found.\nUse [D] to downgrade first.")
            .alignment(Alignment::Center)
            .style(theme::text_secondary());
        f.render_widget(msg, layout[1]);
    } else {
        let active = app
            .selected_game_entry()
            .map(|e| e.active_manifest_id.as_str())
            .unwrap_or("");

        let latest = app
            .selected_game_entry()
            .map(|e| e.latest_manifest_id.as_str())
            .unwrap_or("");

        let items: Vec<ListItem> = cached
            .iter()
            .enumerate()
            .map(|(i, manifest_id)| {
                let is_active = manifest_id == active;
                let is_latest = manifest_id == latest;
                let label = match (is_active, is_latest) {
                    (true, true) => format!("● {} (installed) (latest)", manifest_id),
                    (true, false) => format!("● {} (installed)", manifest_id),
                    (false, true) => format!("  {} (latest)", manifest_id),
                    (false, false) => format!("  {}", manifest_id),
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

        let list = List::new(items);
        f.render_widget(list, layout[1]);
    }

    let help = Paragraph::new(" [↑↓] select   [Enter] switch   [Esc] cancel ")
        .style(theme::help_bar());
    f.render_widget(help, layout[2]);
}
```

- [ ] **Step 3: Build and verify**

```bash
cargo build -p rewind-cli
```

Expected: compiles cleanly with no warnings about unused imports or dead code

- [ ] **Step 4: Manual smoke test**

Run `cargo run` (or the binary). With Steam open:
- Press `d` on a game → wizard opens with yellow warning line
- Press Enter → "Steam is running. Quit Steam before downloading." appears in the Error pane
- Press `u` on a game with cached versions → picker opens with yellow warning line
- Press Enter on a version → "Steam is running. Quit Steam before switching versions." appears

Close Steam:
- Repeat both flows → no warning, operations proceed normally

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/ui/downgrade_wizard.rs rewind-cli/src/ui/version_picker.rs
git commit -m "feat: render Steam running warning and error in wizard and version picker UI"
```

---

### Task 5: Version bump and cleanup

- [ ] **Step 1: Bump versions**

In `rewind-core/Cargo.toml`, bump `version` from `0.4.0` to `0.4.1`.  
In `rewind-cli/Cargo.toml`, bump `version` from `0.4.0` to `0.4.1`.

- [ ] **Step 2: Run full test suite**

```bash
cargo test
```

Expected: all tests pass (the 2 immutability tests may fail on macOS — this is a known issue per CLAUDE.md)

- [ ] **Step 3: Commit**

```bash
git add rewind-core/Cargo.toml rewind-cli/Cargo.toml
git commit -m "chore: bump version to 0.4.1"
```
