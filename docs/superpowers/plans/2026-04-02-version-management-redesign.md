# Version Management Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign version management to separate download from switch, add SteamDB guidance, show explicit manifest state, and auto-manage locking based on version.

**Architecture:** Rework the D/U key split so D always opens the download wizard with patches/manifests guidance, and U opens a version picker that transitions into a progress overlay. Remove manual lock toggle. Status display changes from "Downgraded (locked)" to "Updates enabled/disabled" with explicit Installed/Spoofed labels.

**Tech Stack:** Rust, ratatui, crossterm, rewind-core

---

### File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rewind-core/src/steamdb.rs` | Modify | Add `app_patchnotes_url()` function |
| `rewind-cli/src/app.rs` | Modify | Add `SwitchOverlay` screen, `SwitchOverlayState` struct, new `StepKind` variants |
| `rewind-cli/src/ui/mod.rs` | Modify | Add routing for `SwitchOverlay` screen |
| `rewind-cli/src/ui/main_screen.rs` | Modify | Update status display and key labels |
| `rewind-cli/src/ui/downgrade_wizard.rs` | Modify | Replace SteamDB step with P/M guidance |
| `rewind-cli/src/ui/version_picker.rs` | Modify | Add `(latest)` label, rename to `(installed)` |
| `rewind-cli/src/ui/switch_overlay.rs` | Create | Progress overlay for version switching |
| `rewind-cli/src/main.rs` | Modify | Rework D/U handlers, switch logic, remove L handler |

---

### Task 1: Add `app_patchnotes_url` to steamdb module

**Files:**
- Modify: `rewind-core/src/steamdb.rs:1-27`

- [ ] **Step 1: Write the failing test**

Add to the existing test module in `rewind-core/src/steamdb.rs`:

```rust
#[test]
fn app_patchnotes_url_test() {
    let url = app_patchnotes_url(3321460);
    assert_eq!(url, "https://www.steamdb.info/app/3321460/patchnotes/");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rewind-core steamdb::tests::app_patchnotes_url_test`
Expected: FAIL — `app_patchnotes_url` not found

- [ ] **Step 3: Write the implementation**

Add after `app_url` (line 10) in `rewind-core/src/steamdb.rs`:

```rust
/// Returns the SteamDB patch notes page URL for a given app ID.
pub fn app_patchnotes_url(app_id: u32) -> String {
    format!("https://www.steamdb.info/app/{}/patchnotes/", app_id)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rewind-core steamdb::tests::app_patchnotes_url_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/steamdb.rs
git commit -m "feat: add app_patchnotes_url to steamdb module"
```

---

### Task 2: Add SwitchOverlay screen and state to App

**Files:**
- Modify: `rewind-cli/src/app.rs:10-80`

- [ ] **Step 1: Add `SwitchOverlay` to the `Screen` enum**

In `rewind-cli/src/app.rs`, add `SwitchOverlay` to the `Screen` enum (line 43-50):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    FirstRun,
    Main,
    DowngradeWizard,
    VersionPicker,
    SwitchOverlay,
    Settings,
}
```

- [ ] **Step 2: Add new StepKind variants for switch overlay**

Add these variants to the `StepKind` enum (line 18-27) and their labels:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    CheckDotnet,
    DownloadDepot,
    DownloadManifest,
    BackupFiles,
    LinkFiles,
    PatchManifest,
    LockManifest,
    // Switch overlay steps
    RepointSymlinks,
    PatchAcf,
    LockAcf,
}

impl StepKind {
    pub fn label(&self) -> &'static str {
        match self {
            StepKind::CheckDotnet => "Checking .NET runtime",
            StepKind::DownloadDepot => "Downloading DepotDownloader",
            StepKind::DownloadManifest => "Downloading manifest files",
            StepKind::BackupFiles => "Backing up current files",
            StepKind::LinkFiles => "Linking manifest files to game directory",
            StepKind::PatchManifest => "Patching Steam manifest",
            StepKind::LockManifest => "Locking manifest file",
            StepKind::RepointSymlinks => "Repointing symlinks",
            StepKind::PatchAcf => "Patching ACF",
            StepKind::LockAcf => "Locking ACF",
        }
    }
}
```

- [ ] **Step 3: Add `SwitchOverlayState` struct**

Add after `VersionPickerState` (line 77-80):

```rust
#[derive(Debug, Default)]
pub struct SwitchOverlayState {
    /// High-level process steps with their status.
    pub steps: Vec<(StepKind, StepStatus)>,
    /// The manifest ID being switched to.
    pub target_manifest: String,
    /// Whether the switch is complete (user can dismiss with Esc).
    pub done: bool,
}
```

- [ ] **Step 4: Add `switch_overlay_state` to App struct**

In the `App` struct (line 103-126), add:

```rust
pub switch_overlay_state: SwitchOverlayState,
```

And initialize it in `App::new()` (line 129-150):

```rust
switch_overlay_state: SwitchOverlayState::default(),
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS (with warnings about unused fields — that's fine)

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/app.rs
git commit -m "feat: add SwitchOverlay screen and state structs"
```

---

### Task 3: Create switch overlay UI

**Files:**
- Create: `rewind-cli/src/ui/switch_overlay.rs`
- Modify: `rewind-cli/src/ui/mod.rs:1-46`

- [ ] **Step 1: Create `switch_overlay.rs`**

Create `rewind-cli/src/ui/switch_overlay.rs`:

```rust
use crate::app::{App, StepStatus};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(50, 40, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Switch Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "Switching to ..." label
            Constraint::Length(1), // spacer
            Constraint::Min(0),   // steps
            Constraint::Length(1), // help line
        ])
        .split(content);

    // Header
    let header = Paragraph::new(format!(
        "Switching to {}...",
        app.switch_overlay_state.target_manifest
    ))
    .style(theme::text());
    f.render_widget(header, layout[0]);

    // Steps
    let step_items: Vec<ListItem> = app
        .switch_overlay_state
        .steps
        .iter()
        .map(|(kind, status)| {
            let (icon, style) = match status {
                StepStatus::Pending => ("[ ]", theme::text_secondary()),
                StepStatus::InProgress => ("[\u{2026}]", theme::status_warning()),
                StepStatus::Done => ("[\u{2713}]", theme::status_success()),
                StepStatus::Failed(_) => ("[\u{2717}]", theme::status_error()),
            };
            let label = if matches!(kind, crate::app::StepKind::LockAcf) {
                if let StepStatus::Done = status {
                    kind.label().to_string()
                } else if matches!(status, StepStatus::Pending) {
                    // Check if this is a latest-version switch (will be skipped)
                    kind.label().to_string()
                } else {
                    kind.label().to_string()
                }
            } else {
                kind.label().to_string()
            };
            ListItem::new(format!(" {} {}", icon, label)).style(style)
        })
        .collect();
    let step_list = List::new(step_items);
    f.render_widget(step_list, layout[2]);

    // Help line
    let help_text = if app.switch_overlay_state.done {
        " Done! [Esc] close "
    } else {
        " Switching... "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
```

- [ ] **Step 2: Register the module in `mod.rs`**

In `rewind-cli/src/ui/mod.rs`, add the module declaration (after line 6):

```rust
pub mod switch_overlay;
```

- [ ] **Step 3: Add routing for `SwitchOverlay` screen**

In `rewind-cli/src/ui/mod.rs`, update the `draw` function (lines 32-46) to add the new screen:

```rust
pub fn draw(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::FirstRun => first_run::draw(f, app),
        Screen::Main => main_screen::draw(f, app),
        Screen::DowngradeWizard => {
            main_screen::draw(f, app);
            downgrade_wizard::draw(f, app);
        }
        Screen::VersionPicker => {
            main_screen::draw(f, app);
            version_picker::draw(f, app);
        }
        Screen::SwitchOverlay => {
            main_screen::draw(f, app);
            switch_overlay::draw(f, app);
        }
        Screen::Settings => settings::draw(f, app),
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/ui/switch_overlay.rs rewind-cli/src/ui/mod.rs
git commit -m "feat: add switch overlay UI component"
```

---

### Task 4: Update main screen status display

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs:42-132`

- [ ] **Step 1: Update status bar key labels**

In `rewind-cli/src/ui/main_screen.rs`, replace the status bar text (line 42-46):

```rust
    let status = Paragraph::new(
        " [↑↓/jk] navigate  [D] download  [U] switch version  [O] SteamDB  [S] settings  [Q] quit ",
    )
    .style(theme::help_bar());
    f.render_widget(status, outer[2]);
```

- [ ] **Step 2: Update game list indicators**

No change needed — `▼` and `✓` already map correctly. `▼` = updates disabled, `✓` = updates enabled.

- [ ] **Step 3: Update detail panel status line**

Replace the `status_line` logic (lines 104-111):

```rust
    let status_line = match entry {
        Some(e) if e.active_manifest_id != e.latest_manifest_id => "▼ Updates disabled",
        Some(e) if e.acf_locked => "✓ Updates disabled",
        Some(_) => "✓ Updates enabled",
        None => "  Updates enabled",
    };
```

- [ ] **Step 4: Update detail panel text with Installed/Spoofed labels**

Replace the detail text formatting (lines 123-132):

```rust
    let spoofed_line = match entry {
        Some(e) if e.active_manifest_id != e.latest_manifest_id => {
            format!("\n  Spoofed as: {}", e.latest_manifest_id)
        }
        _ => String::new(),
    };

    let text = format!(
        "  {name}\n  App ID:  {app_id}\n  Depot:   {depot_id}\n\n  Status:     {status}\n  Installed:  {active}{spoofed}\n  Cached:     {cached}\n\n  [D] Download new version\n  [U] Switch version\n  [O] Open app on SteamDB",
        name = game.name,
        app_id = game.app_id,
        depot_id = game.depot_id,
        status = status_line,
        active = active_manifest,
        spoofed = spoofed_line,
        cached = cached_list,
    );
```

Note: The SteamDB depot manifests URL line and `[L] Toggle ACF lock` are removed from the detail panel.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/ui/main_screen.rs
git commit -m "feat: update status display to show updates enabled/disabled"
```

---

### Task 5: Redesign download wizard input view

**Files:**
- Modify: `rewind-cli/src/ui/downgrade_wizard.rs:14-111`
- Modify: `rewind-cli/src/app.rs` (add `app_id` to `DowngradeWizardState`)

- [ ] **Step 1: Add `app_id` and `depot_id` to `DowngradeWizardState`**

In `rewind-cli/src/app.rs`, update `DowngradeWizardState` (lines 52-68):

```rust
#[derive(Debug, Default)]
pub struct DowngradeWizardState {
    pub manifest_input: String,
    pub steamdb_url: String,
    pub app_id: u32,
    pub depot_id: u32,
    pub is_downloading: bool,
    pub error: Option<String>,
    pub error_url: Option<String>,
    pub steps: Vec<(StepKind, StepStatus)>,
    pub depot_lines: Vec<String>,
    pub prompt_input: Option<String>,
    pub prompt_label: Option<String>,
}
```

- [ ] **Step 2: Update wizard title and input view**

Replace `draw_input_view` in `rewind-cli/src/ui/downgrade_wizard.rs` (lines 38-111):

```rust
fn draw_input_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // guidance text
            Constraint::Length(3), // manifest input
            Constraint::Min(0),   // error/output log
            Constraint::Length(1), // help line
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
        .border_style(theme::border_focused())
        .style(theme::base_bg());
    let input_para =
        Paragraph::new(format!("{}{}", app.wizard_state.manifest_input, cursor))
            .style(input_style)
            .block(input_block);
    f.render_widget(input_para, layout[1]);

    // Error / output log
    let (log_title, log_border_style) = if app.wizard_state.error.is_some() {
        (" Error ", Style::default().fg(theme::ERROR).bg(theme::BASE_BG))
    } else {
        (" Output ", theme::border())
    };

    let log_items: Vec<ListItem> = if let Some(err) = &app.wizard_state.error {
        vec![ListItem::new(err.as_str()).style(Style::default().fg(theme::ERROR).bg(theme::BASE_BG))]
    } else {
        vec![]
    };

    let log_block = Block::default()
        .title(log_title)
        .borders(Borders::ALL)
        .border_style(log_border_style)
        .style(theme::base_bg());
    let log_list = List::new(log_items).block(log_block);
    f.render_widget(log_list, layout[2]);

    // Help line
    let help_text = if app.wizard_state.error_url.is_some() {
        " [O] open download page   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [P] patches   [M] manifests   [Enter] download   [Esc] cancel "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
```

- [ ] **Step 3: Update wizard title**

Replace the block title in `draw` (line 15):

```rust
        .title(" Download New Version ")
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/ui/downgrade_wizard.rs rewind-cli/src/app.rs
git commit -m "feat: redesign wizard with patches/manifests guidance"
```

---

### Task 6: Update version picker to show `(latest)` and `(installed)` labels

**Files:**
- Modify: `rewind-cli/src/ui/version_picker.rs:40-66`

- [ ] **Step 1: Update version list labels**

Replace the item rendering loop in `rewind-cli/src/ui/version_picker.rs` (lines 40-66):

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/version_picker.rs
git commit -m "feat: add (latest) and (installed) labels in version picker"
```

---

### Task 7: Rework key handlers in main.rs

**Files:**
- Modify: `rewind-cli/src/main.rs:316-513`

- [ ] **Step 1: Update `handle_main` — rework D key**

Replace the D key handler (lines 321-337) so it always opens the wizard:

```rust
        KeyCode::Char('d') => {
            if let Some(g) = app.selected_game() {
                let url = rewind_core::steamdb::depot_manifests_url(g.depot_id);
                app.wizard_state = DowngradeWizardState {
                    steamdb_url: url,
                    app_id: g.app_id,
                    depot_id: g.depot_id,
                    ..Default::default()
                };
                app.screen = Screen::DowngradeWizard;
            }
        }
```

- [ ] **Step 2: Update `handle_main` — rework U key**

Replace the U key handler (lines 339-345):

```rust
        KeyCode::Char('u') => {
            if app.selected_game_entry().map(|e| e.cached_manifest_ids.len() > 1).unwrap_or(false) {
                app.version_picker_state.selected_index = 0;
                app.screen = Screen::VersionPicker;
            }
        }
```

(This stays the same — version picker still opens first, switch overlay triggers on Enter.)

- [ ] **Step 3: Remove L key handler**

Remove the entire `KeyCode::Char('l')` block (lines 346-366).

- [ ] **Step 4: Update `handle_wizard` — replace [O] with [P] and [M]**

In `handle_wizard` (lines 430-480), replace the `KeyCode::Char('o')` handler (lines 440-447):

```rust
        KeyCode::Char('p') => {
            if !app.wizard_state.is_downloading {
                let url = rewind_core::steamdb::app_patchnotes_url(app.wizard_state.app_id);
                let _ = open::that(url);
            }
        }
        KeyCode::Char('m') => {
            if !app.wizard_state.is_downloading {
                let url = rewind_core::steamdb::depot_manifests_url(app.wizard_state.depot_id);
                let _ = open::that(url);
            }
        }
```

- [ ] **Step 5: Update `handle_version_picker` — transition to SwitchOverlay on Enter**

Replace the `KeyCode::Enter` handler in `handle_version_picker` (lines 500-510):

```rust
        KeyCode::Enter => {
            let target_manifest = app
                .selected_game_entry()
                .and_then(|e| e.cached_manifest_ids.get(app.version_picker_state.selected_index))
                .cloned();

            if let Some(manifest_id) = target_manifest {
                // Check if this is the currently installed manifest
                let is_current = app
                    .selected_game_entry()
                    .map(|e| e.active_manifest_id == manifest_id)
                    .unwrap_or(false);

                if is_current {
                    // Already on this version, just go back
                    app.screen = Screen::Main;
                    return;
                }

                let is_latest = app
                    .selected_game_entry()
                    .map(|e| e.latest_manifest_id == manifest_id)
                    .unwrap_or(false);

                // Initialize switch overlay steps
                let mut steps = vec![
                    (StepKind::RepointSymlinks, StepStatus::Pending),
                    (StepKind::PatchAcf, StepStatus::Pending),
                ];
                if is_latest {
                    steps.push((StepKind::LockAcf, StepStatus::Done));
                } else {
                    steps.push((StepKind::LockAcf, StepStatus::Pending));
                }

                app.switch_overlay_state = SwitchOverlayState {
                    steps,
                    target_manifest: manifest_id.clone(),
                    done: false,
                };
                app.screen = Screen::SwitchOverlay;

                // Perform the switch
                switch_to_cached_version(app, manifest_id, is_latest);
            }
        }
```

- [ ] **Step 6: Add `handle_switch_overlay` function**

Add a new handler function after `handle_version_picker`:

```rust
fn handle_switch_overlay(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.switch_overlay_state = SwitchOverlayState::default();
            app.screen = Screen::Main;
        }
        _ => {}
    }
}
```

- [ ] **Step 7: Add routing in `handle_key`**

In `handle_key` (line 289-302), add the new screen to the match:

```rust
Screen::SwitchOverlay => handle_switch_overlay(app, key),
```

- [ ] **Step 8: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS (may have warnings)

- [ ] **Step 9: Commit**

```bash
git add rewind-cli/src/main.rs
git commit -m "feat: rework key handlers for D/U split and switch overlay"
```

---

### Task 8: Rework `switch_to_cached_version` with overlay steps and latest-version logic

**Files:**
- Modify: `rewind-cli/src/main.rs:670-717`

- [ ] **Step 1: Update function signature and logic**

Replace `switch_to_cached_version` (lines 670-717):

```rust
fn switch_to_cached_version(app: &mut App, manifest_id: String, is_latest: bool) {
    let Some(game) = app.selected_game().cloned() else {
        return;
    };
    let Ok(cache_root) = config::cache_dir() else { return };

    let new_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &manifest_id,
    );

    // Step 1: Repoint symlinks
    app.set_switch_step_status(&StepKind::RepointSymlinks, StepStatus::InProgress);
    if let Err(e) = rewind_core::cache::repoint_symlinks(&game.install_path, &new_cache) {
        app.set_switch_step_status(
            &StepKind::RepointSymlinks,
            StepStatus::Failed(e.to_string()),
        );
        return;
    }
    app.set_switch_step_status(&StepKind::RepointSymlinks, StepStatus::Done);

    if let Some(entry) = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == game.app_id)
    {
        entry.active_manifest_id = manifest_id.clone();
        let acf_path = entry.acf_path();

        // Step 2: Patch ACF
        app.set_switch_step_status(&StepKind::PatchAcf, StepStatus::InProgress);
        let _ = rewind_core::immutability::unlock_file(&acf_path);

        let (buildid, manifest_for_acf) = if is_latest {
            // Switching to latest: use real values, no spoofing
            (entry.latest_buildid.clone(), entry.latest_manifest_id.clone())
        } else {
            // Switching to non-latest: spoof as latest
            (entry.latest_buildid.clone(), entry.latest_manifest_id.clone())
        };

        if let Err(e) = rewind_core::patcher::patch_acf_file(
            &acf_path,
            &buildid,
            &manifest_for_acf,
            entry.depot_id,
        ) {
            app.set_switch_step_status(&StepKind::PatchAcf, StepStatus::Failed(e.to_string()));
            return;
        }
        app.set_switch_step_status(&StepKind::PatchAcf, StepStatus::Done);

        // Step 3: Lock ACF (only if not switching to latest)
        if is_latest {
            // Don't lock — let Steam manage updates
            entry.acf_locked = false;
            // LockAcf step was already set to Done with a skip indication in the overlay init
        } else {
            app.set_switch_step_status(&StepKind::LockAcf, StepStatus::InProgress);
            if let Err(e) = rewind_core::immutability::lock_file(&acf_path) {
                app.set_switch_step_status(
                    &StepKind::LockAcf,
                    StepStatus::Failed(e.to_string()),
                );
                return;
            }
            app.set_switch_step_status(&StepKind::LockAcf, StepStatus::Done);
            entry.acf_locked = true;
        }
    }

    let _ = config::save_games(&app.games_config);
    app.switch_overlay_state.done = true;
}
```

- [ ] **Step 2: Add `set_switch_step_status` helper to App**

In `rewind-cli/src/app.rs`, add after `set_step_status` (line 152-156):

```rust
    pub fn set_switch_step_status(&mut self, kind: &StepKind, status: StepStatus) {
        if let Some(step) = self.switch_overlay_state.steps.iter_mut().find(|s| s.0 == *kind) {
            step.1 = status;
        }
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/main.rs rewind-cli/src/app.rs
git commit -m "feat: rework switch logic with overlay steps and latest-version handling"
```

---

### Task 9: Update switch overlay UI for skipped lock step

**Files:**
- Modify: `rewind-cli/src/ui/switch_overlay.rs`

- [ ] **Step 1: Update step rendering to show skip message for LockAcf when latest**

In `switch_overlay.rs`, replace the step rendering logic (the `step_items` mapping) with:

```rust
    let step_items: Vec<ListItem> = app
        .switch_overlay_state
        .steps
        .iter()
        .map(|(kind, status)| {
            // Special handling for LockAcf when switching to latest (shown as skipped)
            let is_lock_skipped = matches!(kind, crate::app::StepKind::LockAcf)
                && matches!(status, StepStatus::Done)
                && app
                    .selected_game_entry()
                    .map(|e| e.active_manifest_id == e.latest_manifest_id)
                    .unwrap_or(false);

            if is_lock_skipped {
                let label = format!(" [\u{2014}] {} (skipped \u{2014} updates enabled)", kind.label());
                return ListItem::new(label).style(theme::text_secondary());
            }

            let (icon, style) = match status {
                StepStatus::Pending => ("[ ]", theme::text_secondary()),
                StepStatus::InProgress => ("[\u{2026}]", theme::status_warning()),
                StepStatus::Done => ("[\u{2713}]", theme::status_success()),
                StepStatus::Failed(_) => ("[\u{2717}]", theme::status_error()),
            };
            ListItem::new(format!(" {} {}", icon, kind.label())).style(style)
        })
        .collect();
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/switch_overlay.rs
git commit -m "feat: show skipped lock step when switching to latest version"
```

---

### Task 10: Update startup repair to skip unlocked games

**Files:**
- Modify: `rewind-cli/src/main.rs:234-287`

- [ ] **Step 1: Update `repair_stale_locks`**

The function already filters on `e.acf_locked` (line 239), so games with "updates enabled" (acf_locked = false) are already skipped. No code change needed.

- [ ] **Step 2: Verify by reading the code**

Confirm that line 239 has `.filter(|e| e.acf_locked)` — this correctly skips games where updates are enabled.

- [ ] **Step 3: Commit (skip — no changes)**

No changes needed for this task.

---

### Task 11: Remove unused imports and clean up

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs` (remove `steamdb` import if no longer used)
- Modify: `rewind-cli/src/main.rs` (ensure all new types are imported)

- [ ] **Step 1: Check and fix imports in main_screen.rs**

The `steamdb` import at line 9 (`use rewind_core::steamdb;`) is no longer used since we removed the SteamDB URL from the detail panel. Remove it.

- [ ] **Step 2: Ensure main.rs imports the new types**

Make sure these are imported at the top of `main.rs`:

```rust
use crate::app::{SwitchOverlayState, StepKind, StepStatus};
```

Check existing imports and only add what's missing.

- [ ] **Step 3: Run full build**

Run: `cargo build -p rewind-cli`
Expected: PASS with no errors

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass (except known macOS immutability test failures)

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/ui/main_screen.rs rewind-cli/src/main.rs
git commit -m "chore: clean up imports after version management redesign"
```

---

### Task 12: Final verification

- [ ] **Step 1: Full build**

Run: `cargo build --release`
Expected: PASS

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All pass (except known macOS immutability failures)

- [ ] **Step 3: Manual verification checklist**

Verify these behaviors work correctly:
- Main screen shows "Updates enabled" / "Updates disabled" status
- Main screen shows "Installed:" and "Spoofed as:" (when applicable)
- `[D]` always opens download wizard with P/M guidance
- `[U]` opens version picker with `(installed)` and `(latest)` labels
- Selecting a version in picker opens switch overlay with progress steps
- Switching to latest skips lock step and shows "skipped — updates enabled"
- `[L]` key does nothing (removed)
- `[O]` still opens SteamDB app page

- [ ] **Step 4: Commit any final fixes**
