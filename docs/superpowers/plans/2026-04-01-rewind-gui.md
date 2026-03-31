# rewind-gui Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `rewind-gui` crate to the workspace — a standalone `egui`/`eframe` desktop app that exposes the same rewind functionality as the TUI, targeting users who prefer a graphical interface.

**Architecture:** `rewind-gui` is a new Cargo workspace member that depends only on `rewind-core` and `eframe`/`egui`. All business logic (scanning, downloading, patching) lives in `rewind-core`; the GUI crate owns only state management and rendering. Background operations run on a `tokio` runtime held by `RewindApp`; progress is communicated to the render loop via `tokio::sync::mpsc` channels polled with `try_recv()` each frame.

**Tech Stack:** `eframe 0.31`, `egui 0.31`, `tokio 1` (full), `open 5`, `rewind-core` (local)

---

## File Map

| File | Role |
|------|------|
| `Cargo.toml` (workspace root) | Add `"rewind-gui"` to `members` |
| `rewind-gui/Cargo.toml` | Crate manifest with all deps |
| `rewind-gui/src/main.rs` | `eframe::run_native` entry point |
| `rewind-gui/src/app.rs` | `RewindApp` struct, `eframe::App` impl, all state types |
| `rewind-gui/src/views/mod.rs` | Re-exports all view modules |
| `rewind-gui/src/views/first_run.rs` | Full-window first-run setup screen |
| `rewind-gui/src/views/game_list.rs` | Left panel — scrollable game list |
| `rewind-gui/src/views/game_detail.rs` | Right panel — selected game info + action buttons |
| `rewind-gui/src/views/downgrade.rs` | Downgrade wizard modal (step 1: manifest input, step 2: progress) |
| `rewind-gui/src/views/settings.rs` | Settings modal overlay |
| `rewind-gui/src/views/status_bar.rs` | Bottom panel — download progress or idle |

---

## Task 1: Workspace setup

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `rewind-gui/Cargo.toml`
- Create: `rewind-gui/src/main.rs`

- [ ] **Step 1: Add rewind-gui to workspace members**

In `/home/yanek/Projects/rewind-cli/Cargo.toml`:
```toml
[workspace]
members = ["rewind-core", "rewind-cli", "rewind-gui"]
resolver = "2"
```

- [ ] **Step 2: Create rewind-gui/Cargo.toml**

```toml
[package]
name = "rewind-gui"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rewind-gui"
path = "src/main.rs"

[dependencies]
rewind-core = { path = "../rewind-core" }
eframe = "0.31"
egui = "0.31"
tokio = { version = "1", features = ["full"] }
open = "5"
anyhow = "1"
```

- [ ] **Step 3: Create a stub main.rs**

```rust
fn main() {
    println!("rewind-gui stub");
}
```

- [ ] **Step 4: Verify the workspace compiles**

Run: `cargo check -p rewind-gui`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rewind-gui/
git commit -m "feat(gui): add rewind-gui crate to workspace"
```

---

## Task 2: App state types

**Files:**
- Create: `rewind-gui/src/app.rs`

- [ ] **Step 1: Write failing tests for state logic**

Create `rewind-gui/src/app.rs`:
```rust
use std::time::Instant;
use rewind_core::config::{Config, GameEntry};
use rewind_core::scanner::InstalledGame;
use std::path::PathBuf;
use tokio::sync::mpsc;
use rewind_core::depot::DepotProgress;

// ── View state ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum View {
    FirstRun,
    Main,
    DowngradeWizard { app_id: u32, step: WizardStep },
    Settings,
}

#[derive(Debug, PartialEq)]
pub enum WizardStep {
    PickManifest,
    Downloading,
}

// ── Download state ────────────────────────────────────────────────────────────

pub struct DownloadState {
    pub log_lines: Vec<String>,
    pub done: bool,
    pub error: Option<String>,
    pub rx: mpsc::Receiver<DepotProgress>,
}

impl DownloadState {
    pub fn new(rx: mpsc::Receiver<DepotProgress>) -> Self {
        Self { log_lines: Vec::new(), done: false, error: None, rx }
    }

    /// Drain all currently-available messages from the channel.
    /// Returns true if the download finished (done or error) this call.
    pub fn poll(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(DepotProgress::Line(line)) => {
                    self.log_lines.push(line);
                    if self.log_lines.len() > 100 {
                        self.log_lines.remove(0);
                    }
                }
                Ok(DepotProgress::Done) => { self.done = true; return true; }
                Ok(DepotProgress::Error(e)) => { self.error = Some(e); return true; }
                Err(_) => return false,
            }
        }
    }
}

// ── Toast ─────────────────────────────────────────────────────────────────────

pub struct Toast {
    pub message: String,
    pub expires_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            expires_at: Instant::now() + std::time::Duration::from_secs(3),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

// ── DisplayGame (merged scanner + rewind tracking) ────────────────────────────

#[derive(Debug, Clone)]
pub struct DisplayGame {
    pub app_id: u32,
    pub name: String,
    pub depot_id: u32,
    pub install_path: PathBuf,
    pub manifest_id: String,
    pub entry: Option<GameEntry>,
}

impl DisplayGame {
    pub fn status_label(&self) -> &'static str {
        match &self.entry {
            None => "Native",
            Some(e) if e.acf_locked => "Downgraded",
            Some(_) => "Managed",
        }
    }
}

/// Merge a list of scanned games with a list of rewind-tracked entries.
pub fn merge_games(installed: Vec<InstalledGame>, tracked: Vec<GameEntry>) -> Vec<DisplayGame> {
    installed
        .into_iter()
        .map(|g| {
            let entry = tracked.iter().find(|e| e.app_id == g.app_id).cloned();
            DisplayGame {
                app_id: g.app_id,
                name: g.name,
                depot_id: g.depot_id,
                install_path: g.install_path,
                manifest_id: g.manifest_id,
                entry,
            }
        })
        .collect()
}

// ── WizardState (manifest input buffer) ──────────────────────────────────────

#[derive(Default)]
pub struct WizardState {
    pub manifest_input: String,
    pub download: Option<DownloadState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use rewind_core::config::GameEntry;
    use rewind_core::scanner::InstalledGame;

    fn make_installed(app_id: u32, name: &str) -> InstalledGame {
        InstalledGame {
            app_id,
            name: name.into(),
            depot_id: app_id + 1,
            manifest_id: "m1".into(),
            install_path: PathBuf::from("/games"),
            acf_path: PathBuf::from("/games/app.acf"),
            state_flags: 4,
        }
    }

    fn make_entry(app_id: u32, locked: bool) -> GameEntry {
        GameEntry {
            name: "game".into(),
            app_id,
            depot_id: app_id + 1,
            install_path: PathBuf::from("/games"),
            active_manifest_id: "m1".into(),
            latest_manifest_id: "m2".into(),
            cached_manifest_ids: vec![],
            acf_locked: locked,
        }
    }

    #[test]
    fn merge_games_untracked_has_no_entry() {
        let installed = vec![make_installed(100, "Game A")];
        let merged = merge_games(installed, vec![]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].entry.is_none());
        assert_eq!(merged[0].status_label(), "Native");
    }

    #[test]
    fn merge_games_tracked_and_locked() {
        let installed = vec![make_installed(100, "Game A")];
        let tracked = vec![make_entry(100, true)];
        let merged = merge_games(installed, tracked);
        assert!(merged[0].entry.is_some());
        assert_eq!(merged[0].status_label(), "Downgraded");
    }

    #[test]
    fn merge_games_tracked_not_locked() {
        let installed = vec![make_installed(100, "Game A")];
        let tracked = vec![make_entry(100, false)];
        let merged = merge_games(installed, tracked);
        assert_eq!(merged[0].status_label(), "Managed");
    }

    #[test]
    fn merge_games_mixed() {
        let installed = vec![make_installed(1, "A"), make_installed(2, "B")];
        let tracked = vec![make_entry(1, false)];
        let merged = merge_games(installed, tracked);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().find(|g| g.app_id == 1).unwrap().entry.is_some());
        assert!(merged.iter().find(|g| g.app_id == 2).unwrap().entry.is_none());
    }

    #[test]
    fn toast_expires() {
        let mut toast = Toast::new("hello");
        toast.expires_at = Instant::now() - std::time::Duration::from_secs(1);
        assert!(toast.is_expired());
    }

    #[test]
    fn toast_not_expired() {
        let toast = Toast::new("hello");
        assert!(!toast.is_expired());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p rewind-gui`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add rewind-gui/src/app.rs
git commit -m "feat(gui): add app state types and merge_games logic"
```

---

## Task 3: RewindApp struct and eframe entry point

**Files:**
- Modify: `rewind-gui/src/app.rs` — add `RewindApp` struct + `eframe::App` impl
- Modify: `rewind-gui/src/main.rs` — wire up `eframe::run_native`

- [ ] **Step 1: Write test for initial view selection**

Add to the `#[cfg(test)]` block in `rewind-gui/src/app.rs`:
```rust
    // Note: RewindApp::new() will be tested after it's defined in Step 2.
    // These tests cover the view-selection logic directly.

    #[test]
    fn initial_view_is_first_run_when_no_libraries() {
        let config = Config::default();
        let view = initial_view(&config);
        assert_eq!(view, View::FirstRun);
    }

    #[test]
    fn initial_view_is_main_when_libraries_configured() {
        let config = Config {
            steam_username: Some("user".into()),
            libraries: vec![rewind_core::config::Library { path: "/tmp".into() }],
        };
        let view = initial_view(&config);
        assert_eq!(view, View::Main);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rewind-gui`
Expected: FAIL — `initial_view` not defined

- [ ] **Step 3: Add `initial_view` helper and `RewindApp` struct to app.rs**

Add after the `WizardState` definition (before `#[cfg(test)]`):
```rust
/// Determines the starting view based on whether any libraries are configured.
pub fn initial_view(config: &Config) -> View {
    if config.libraries.is_empty() {
        View::FirstRun
    } else {
        View::Main
    }
}

pub struct RewindApp {
    pub config: Config,
    pub games: Vec<DisplayGame>,
    pub selected: Option<usize>,
    pub view: View,
    pub wizard: WizardState,
    pub toast: Option<Toast>,
    pub rt: tokio::runtime::Runtime,
    // Transient UI state
    pub settings_username_buf: String,
    pub settings_lib_buf: String,
    pub first_run_lib_buf: String,
}

impl RewindApp {
    pub fn new() -> Self {
        let config = rewind_core::config::load_config().unwrap_or_default();
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let view = initial_view(&config);
        let settings_username_buf = config.steam_username.clone().unwrap_or_default();
        Self {
            config,
            games: Vec::new(),
            selected: None,
            view,
            wizard: WizardState::default(),
            toast: None,
            rt,
            settings_username_buf,
            settings_lib_buf: String::new(),
            first_run_lib_buf: String::new(),
        }
    }
}
```

- [ ] **Step 4: Add eframe::App impl stub to app.rs**

Add at the top of app.rs after imports:
```rust
use eframe::egui;
```

Add after `RewindApp`:
```rust
impl eframe::App for RewindApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Expire toast
        if let Some(t) = &self.toast {
            if t.is_expired() {
                self.toast = None;
            }
        }

        match self.view {
            View::FirstRun => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("First run placeholder");
                });
            }
            _ => {
                egui::TopBottomPanel::top("header").show(ctx, |ui| {
                    ui.label("rewind");
                });
                egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
                    ui.label("Ready");
                });
                egui::SidePanel::left("games")
                    .min_width(180.0)
                    .show(ctx, |ui| {
                        ui.label("Games panel placeholder");
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Detail panel placeholder");
                });
            }
        }
    }
}
```

- [ ] **Step 5: Replace main.rs**

```rust
mod app;
mod views;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 540.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("rewind"),
        ..Default::default()
    };
    eframe::run_native(
        "rewind",
        options,
        Box::new(|_cc| Ok(Box::new(app::RewindApp::new()))),
    )
}
```

- [ ] **Step 6: Create views/mod.rs stub**

Create `rewind-gui/src/views/mod.rs`:
```rust
pub mod downgrade;
pub mod first_run;
pub mod game_detail;
pub mod game_list;
pub mod settings;
pub mod status_bar;
```

Create each stub (all identical pattern):

`rewind-gui/src/views/first_run.rs`:
```rust
pub fn show(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

`rewind-gui/src/views/game_list.rs`:
```rust
pub fn show_panel(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

`rewind-gui/src/views/game_detail.rs`:
```rust
pub fn show_panel(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

`rewind-gui/src/views/downgrade.rs`:
```rust
pub fn show(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

`rewind-gui/src/views/settings.rs`:
```rust
pub fn show(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

`rewind-gui/src/views/status_bar.rs`:
```rust
pub fn show(_app: &mut crate::app::RewindApp, _ctx: &eframe::egui::Context) {}
```

- [ ] **Step 7: Run tests and verify window opens**

Run: `cargo test -p rewind-gui`
Expected: all tests pass

Run: `cargo run -p rewind-gui`
Expected: a window opens with placeholder labels; close it

- [ ] **Step 8: Commit**

```bash
git add rewind-gui/src/
git commit -m "feat(gui): wire up eframe entry point with placeholder panels"
```

---

## Task 4: Header panel

**Files:**
- Create: (no new file — implement in app.rs's `update` inline, or extract to a small helper)
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Replace the header placeholder in `update()`**

In the `_ =>` branch of `update()`, replace the header panel:
```rust
egui::TopBottomPanel::top("header").show(ctx, |ui| {
    ui.horizontal(|ui| {
        ui.heading("rewind");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("?").on_hover_text("Help").clicked() {
                let _ = open::that("https://github.com/your-org/rewind");
            }
            if ui.button("Settings").clicked() {
                self.view = View::Settings;
                self.settings_username_buf = self.config.steam_username
                    .clone()
                    .unwrap_or_default();
            }
        });
    });
});
```

- [ ] **Step 2: Run and verify header renders**

Run: `cargo run -p rewind-gui`
Expected: top bar shows "rewind" on the left, "Settings" and "?" buttons on the right

- [ ] **Step 3: Commit**

```bash
git add rewind-gui/src/app.rs
git commit -m "feat(gui): implement header panel with settings and help buttons"
```

---

## Task 5: Game list panel

**Files:**
- Modify: `rewind-gui/src/views/game_list.rs`
- Modify: `rewind-gui/src/app.rs` (wire up the call)

- [ ] **Step 1: Implement `show_panel` in game_list.rs**

```rust
use crate::app::RewindApp;
use eframe::egui;

pub fn show_panel(app: &mut RewindApp, ctx: &egui::Context) {
    egui::SidePanel::left("games")
        .min_width(180.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Games");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, game) in app.games.iter().enumerate() {
                    let selected = app.selected == Some(i);
                    let label = egui::SelectableLabel::new(selected, &game.name);
                    if ui.add(label).clicked() {
                        app.selected = Some(i);
                    }
                }
                if app.games.is_empty() {
                    ui.label(egui::RichText::new("No games found").weak());
                }
            });
        });
}
```

- [ ] **Step 2: Wire game_list::show_panel into update()**

In `app.rs`, in the `_ =>` branch of `update()`, replace the SidePanel placeholder:
```rust
crate::views::game_list::show_panel(self, ctx);
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: left panel shows "Games" heading and "No games found" (since no libraries scanned yet)

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/game_list.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement game list panel"
```

---

## Task 6: Game detail panel

**Files:**
- Modify: `rewind-gui/src/views/game_detail.rs`
- Modify: `rewind-gui/src/app.rs` (wire up, add View transitions)

- [ ] **Step 1: Implement `show_panel` in game_detail.rs**

```rust
use crate::app::{RewindApp, View, WizardStep, WizardState};
use eframe::egui;
use rewind_core::steamdb;

pub fn show_panel(app: &mut RewindApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let Some(idx) = app.selected else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Select a game").weak());
            });
            return;
        };

        let game = &app.games[idx];

        ui.heading(&game.name);
        ui.separator();

        egui::Grid::new("detail_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("App ID:");
                ui.label(game.app_id.to_string());
                ui.end_row();

                ui.label("Status:");
                ui.label(game.status_label());
                ui.end_row();

                ui.label("Current manifest:");
                ui.label(&game.manifest_id);
                ui.end_row();

                if let Some(entry) = &game.entry {
                    ui.label("Active:");
                    ui.label(&entry.active_manifest_id);
                    ui.end_row();

                    ui.label("Latest:");
                    ui.label(&entry.latest_manifest_id);
                    ui.end_row();

                    if !entry.cached_manifest_ids.is_empty() {
                        ui.label("Cached:");
                        ui.label(entry.cached_manifest_ids.join(", "));
                        ui.end_row();
                    }
                }
            });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Downgrade").clicked() {
                app.wizard = WizardState::default();
                app.view = View::DowngradeWizard {
                    app_id: game.app_id,
                    step: WizardStep::PickManifest,
                };
            }
            if let Some(entry) = &game.entry {
                if entry.acf_locked && ui.button("Restore Latest").clicked() {
                    app.toast = Some(crate::app::Toast::new(
                        "Restore not yet available — rewind-core cache module pending.",
                    ));
                }
            }
            let steamdb_url = steamdb::app_url(game.app_id);
            if ui.button("Open SteamDB").clicked() {
                let _ = open::that(&steamdb_url);
            }
        });
    });
}
```

- [ ] **Step 2: Wire game_detail::show_panel into update()**

In `app.rs`, replace the CentralPanel placeholder in the `_ =>` branch:
```rust
crate::views::game_detail::show_panel(self, ctx);
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: right panel shows "Select a game" when nothing is selected

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/game_detail.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement game detail panel"
```

---

## Task 7: Status bar

**Files:**
- Modify: `rewind-gui/src/views/status_bar.rs`
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Implement `show` in status_bar.rs**

```rust
use crate::app::{RewindApp, View};
use eframe::egui;

pub fn show(app: &mut RewindApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Poll download progress if a wizard step 2 is active
            if let View::DowngradeWizard { ref step, .. } = app.view {
                if *step == crate::app::WizardStep::Downloading {
                    if let Some(dl) = app.wizard.download.as_mut() {
                        let finished = dl.poll();
                        let pct = (dl.log_lines.len() % 20) as f32 / 20.0; // indeterminate proxy
                        ui.add(egui::ProgressBar::new(pct).animate(true));
                        if let Some(last) = dl.log_lines.last() {
                            ui.label(
                                egui::RichText::new(last).small().weak(),
                            );
                        }
                        if finished {
                            ctx.request_repaint();
                        } else {
                            ctx.request_repaint_after(
                                std::time::Duration::from_millis(100),
                            );
                        }
                    }
                    return;
                }
            }

            // Toast
            if let Some(toast) = &app.toast {
                ui.label(&toast.message);
                return;
            }

            ui.label(egui::RichText::new("Ready").weak());
        });
    });
}
```

- [ ] **Step 2: Wire status_bar::show into update()**

In `app.rs`, replace the bottom panel placeholder in `update()`:
```rust
crate::views::status_bar::show(self, ctx);
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: bottom bar shows "Ready"

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/status_bar.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement status bar with download progress and toast"
```

---

## Task 8: First-run screen

**Files:**
- Modify: `rewind-gui/src/views/first_run.rs`
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Implement `show` in first_run.rs**

```rust
use crate::app::{RewindApp, View};
use eframe::egui;
use rewind_core::config::{Library, save_config};
use rewind_core::scanner::find_steam_libraries;

pub fn show(app: &mut RewindApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.heading("Welcome to rewind");
            ui.add_space(8.0);
            ui.label("rewind manages Steam game version downgrades.");
            ui.add_space(24.0);

            if ui.button("Auto-detect Steam libraries").clicked() {
                match find_steam_libraries() {
                    Ok(paths) => {
                        app.config.libraries = paths
                            .into_iter()
                            .map(|p| Library { path: p })
                            .collect();
                        if app.config.libraries.is_empty() {
                            app.toast = Some(crate::app::Toast::new(
                                "No Steam libraries found. Add one manually.",
                            ));
                        }
                    }
                    Err(_) => {
                        app.toast = Some(crate::app::Toast::new(
                            "Steam not found. Add a library path manually.",
                        ));
                    }
                }
            }

            ui.add_space(12.0);
            ui.label("Or add a library path manually:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.first_run_lib_buf);
                if ui.button("Add").clicked() && !app.first_run_lib_buf.is_empty() {
                    let path = std::path::PathBuf::from(&app.first_run_lib_buf);
                    app.config.libraries.push(Library { path });
                    app.first_run_lib_buf.clear();
                }
            });

            if !app.config.libraries.is_empty() {
                ui.add_space(12.0);
                ui.label("Libraries to add:");
                for lib in &app.config.libraries {
                    ui.label(lib.path.display().to_string());
                }

                ui.add_space(12.0);
                if ui.button("Continue").clicked() {
                    if let Err(e) = save_config(&app.config) {
                        app.toast = Some(crate::app::Toast::new(format!("Save failed: {e}")));
                    } else {
                        app.view = View::Main;
                    }
                }
            }

            // Show toast inside this panel
            if let Some(toast) = &app.toast {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&toast.message).color(egui::Color32::YELLOW));
            }
        });
    });
}
```

- [ ] **Step 2: Wire first_run::show into update()**

In `app.rs`, replace the `View::FirstRun` branch placeholder:
```rust
View::FirstRun => {
    crate::views::first_run::show(self, ctx);
}
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`

If no libraries are configured (fresh state), the first-run screen should appear. If `~/.local/share/rewind/config.toml` already has libraries, delete it first:
```bash
rm -f ~/.local/share/rewind/config.toml
cargo run -p rewind-gui
```
Expected: first-run screen with auto-detect and manual path buttons

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/first_run.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement first-run setup screen"
```

---

## Task 9: Settings overlay

**Files:**
- Modify: `rewind-gui/src/views/settings.rs`
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Implement `show` in settings.rs**

```rust
use crate::app::{RewindApp, View};
use eframe::egui;
use rewind_core::config::{Library, save_config};

pub fn show(app: &mut RewindApp, ctx: &egui::Context) {
    let mut open = matches!(app.view, View::Settings);

    egui::Window::new("Settings")
        .open(&mut open)
        .resizable(false)
        .min_width(360.0)
        .show(ctx, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Steam username:");
                    ui.text_edit_singleline(&mut app.settings_username_buf);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label("Steam library paths:");
            ui.separator();

            let mut to_remove: Option<usize> = None;
            for (i, lib) in app.config.libraries.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(lib.path.display().to_string());
                    if ui.small_button("Remove").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(i) = to_remove {
                app.config.libraries.remove(i);
            }

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.settings_lib_buf);
                if ui.button("Add").clicked() && !app.settings_lib_buf.is_empty() {
                    app.config.libraries.push(Library {
                        path: std::path::PathBuf::from(&app.settings_lib_buf),
                    });
                    app.settings_lib_buf.clear();
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    app.config.steam_username = if app.settings_username_buf.is_empty() {
                        None
                    } else {
                        Some(app.settings_username_buf.clone())
                    };
                    match save_config(&app.config) {
                        Ok(_) => {
                            app.toast = Some(crate::app::Toast::new("Settings saved."));
                            app.view = View::Main;
                        }
                        Err(e) => {
                            app.toast = Some(crate::app::Toast::new(format!("Save failed: {e}")));
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    app.view = View::Main;
                }
            });
        });

    if !open {
        app.view = View::Main;
    }
}
```

- [ ] **Step 2: Wire settings::show into update()**

In `app.rs`, in the `_ =>` branch of `update()`, after the panels, add:
```rust
if matches!(self.view, View::Settings) {
    crate::views::settings::show(self, ctx);
}
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: clicking "Settings" opens a modal window; "Cancel" closes it; "Save" persists the config

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/settings.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement settings overlay"
```

---

## Task 10: Downgrade wizard — Step 1 (manifest input + version picker)

**Files:**
- Modify: `rewind-gui/src/views/downgrade.rs`
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Implement `show` in downgrade.rs (step 1 + version picker)**

```rust
use crate::app::{RewindApp, View, WizardStep, WizardState, DownloadState};
use eframe::egui;
use rewind_core::steamdb;

pub fn show(app: &mut RewindApp, ctx: &egui::Context) {
    let View::DowngradeWizard { app_id, ref step } = app.view else { return };
    let app_id = app_id; // copy out of borrowed view

    let game = app
        .games
        .iter()
        .find(|g| g.app_id == app_id)
        .cloned();
    let Some(game) = game else { return };

    let mut open = true;
    egui::Window::new(format!("Downgrade — {}", game.name))
        .open(&mut open)
        .resizable(false)
        .min_width(440.0)
        .show(ctx, |ui| {
            match step {
                WizardStep::PickManifest => {
                    // Version picker: show cached versions if any
                    if let Some(entry) = &game.entry {
                        if !entry.cached_manifest_ids.is_empty() {
                            ui.label("Switch to a cached version (instant):");
                            let cached = entry.cached_manifest_ids.clone();
                            for manifest in &cached {
                                if ui.button(manifest).clicked() {
                                    // Instant switch — symlink repoint (Task 13)
                                    app.toast = Some(crate::app::Toast::new(
                                        format!("Switched to {manifest} (not yet implemented)")
                                    ));
                                    app.view = View::Main;
                                    return;
                                }
                            }
                            ui.separator();
                        }
                    }

                    // SteamDB link
                    let depot_id = game.depot_id;
                    let url = steamdb::depot_manifests_url(depot_id);
                    ui.label("Find the target manifest ID on SteamDB:");
                    if ui.link(&url).clicked() {
                        let _ = open::that(&url);
                    }

                    ui.add_space(8.0);
                    ui.label("Paste manifest ID:");
                    ui.text_edit_singleline(&mut app.wizard.manifest_input);

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let can_proceed = !app.wizard.manifest_input.trim().is_empty();
                        if ui.add_enabled(can_proceed, egui::Button::new("Download")).clicked() {
                            // Transition to step 2 — Task 11 wires the actual download
                            app.view = View::DowngradeWizard {
                                app_id,
                                step: WizardStep::Downloading,
                            };
                        }
                        if ui.button("Cancel").clicked() {
                            app.view = View::Main;
                        }
                    });
                }
                WizardStep::Downloading => {
                    // Rendered in Task 11
                    ui.label("Downloading…");
                }
            }
        });

    if !open {
        app.view = View::Main;
    }
}
```

- [ ] **Step 2: Wire downgrade::show into update()**

In `app.rs`, in the `_ =>` branch of `update()`, after the panels, add:
```rust
if matches!(self.view, View::DowngradeWizard { .. }) {
    crate::views::downgrade::show(self, ctx);
}
```

- [ ] **Step 3: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: clicking "Downgrade" on a selected game opens the wizard modal with a SteamDB link and a manifest input field; "Cancel" closes it; "Download" button is disabled when input is empty

- [ ] **Step 4: Commit**

```bash
git add rewind-gui/src/views/downgrade.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement downgrade wizard step 1 with version picker"
```

---

## Task 11: Downgrade wizard — Step 2 (download + progress)

**Files:**
- Modify: `rewind-gui/src/views/downgrade.rs`
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Add `start_download` helper to app.rs**

Add below `RewindApp::new()`:
```rust
impl RewindApp {
    /// Spawn a background DepotDownloader task and wire up the mpsc channel.
    pub fn start_download(&mut self, app_id: u32, depot_id: u32, manifest_id: String) {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        self.wizard.download = Some(DownloadState::new(rx));

        let username = self.config.steam_username.clone().unwrap_or_default();
        let bin_dir = rewind_core::config::bin_dir().expect("bin_dir");
        let cache_dir = rewind_core::config::cache_dir()
            .expect("cache_dir")
            .join(app_id.to_string())
            .join(depot_id.to_string())
            .join(&manifest_id);

        self.rt.spawn(async move {
            use rewind_core::depot::{ensure_depot_downloader, run_depot_downloader, DepotProgress};

            let binary = match ensure_depot_downloader(&bin_dir).await {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(DepotProgress::Error(format!("Setup failed: {e}"))).await;
                    return;
                }
            };

            if let Err(e) = run_depot_downloader(
                &binary,
                app_id,
                depot_id,
                &manifest_id,
                &username,
                &cache_dir,
                tx.clone(),
            ).await {
                let _ = tx.send(DepotProgress::Error(e.to_string())).await;
            }
        });
    }
}
```

- [ ] **Step 2: Trigger download when entering WizardStep::Downloading**

In `app.rs`'s `update()`, before rendering, detect the transition into `Downloading` and call `start_download`:
```rust
// Kick off download on first frame of Downloading step
if let View::DowngradeWizard { app_id, step: WizardStep::Downloading } = self.view {
    if self.wizard.download.is_none() {
        let game = self.games.iter().find(|g| g.app_id == app_id).cloned();
        if let Some(game) = game {
            let manifest = self.wizard.manifest_input.trim().to_string();
            self.start_download(app_id, game.depot_id, manifest);
        }
    }
}
```

- [ ] **Step 3: Implement step 2 rendering in downgrade.rs**

Replace the `WizardStep::Downloading` arm in `downgrade.rs`:
```rust
WizardStep::Downloading => {
    if let Some(dl) = app.wizard.download.as_ref() {
        if let Some(err) = &dl.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {err}"));
            if ui.button("Close").clicked() {
                app.view = View::Main;
            }
            return;
        }
        if dl.done {
            ui.colored_label(egui::Color32::GREEN, "Download complete!");
            app.toast = Some(crate::app::Toast::new("Downgrade complete."));
            if ui.button("Done").clicked() {
                app.view = View::Main;
            }
            return;
        }
        ui.label("Downloading…");
        ui.add(egui::ProgressBar::new(0.0).animate(true));
        let log_scroll_height = 160.0;
        egui::ScrollArea::vertical()
            .max_height(log_scroll_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &dl.log_lines {
                    ui.label(egui::RichText::new(line).monospace().small());
                }
            });
        if ui.button("Cancel").clicked() {
            app.wizard.download = None;
            app.view = View::Main;
        }
    }
}
```

- [ ] **Step 4: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: entering a manifest ID and clicking "Download" transitions to step 2 and shows the spinner + log area; output from DepotDownloader appears in the log

Note: The actual DepotDownloader run will fail unless `.NET` is installed and Steam credentials are set up — that is expected in development. Verify the UI renders correctly and the cancel button works.

- [ ] **Step 5: Commit**

```bash
git add rewind-gui/src/views/downgrade.rs rewind-gui/src/app.rs
git commit -m "feat(gui): implement downgrade wizard step 2 with tokio download"
```

---

## Task 12: Load games on startup

**Files:**
- Modify: `rewind-gui/src/app.rs`

- [ ] **Step 1: Write test for `merge_games` with scanner output**

The merge_games tests already cover this in Task 2. No new test needed.

- [ ] **Step 2: Add `reload_games` to RewindApp**

Add to the `impl RewindApp` block:
```rust
    /// Synchronously scan all configured libraries and update self.games.
    pub fn reload_games(&mut self) {
        let tracked = rewind_core::config::load_games()
            .map(|gc| gc.games)
            .unwrap_or_default();

        let mut installed = Vec::new();
        for lib in &self.config.libraries {
            let steamapps = lib.path.join("steamapps");
            if steamapps.exists() {
                if let Ok(games) = rewind_core::scanner::scan_library(&steamapps) {
                    installed.extend(games);
                }
            }
        }

        self.games = crate::app::merge_games(installed, tracked);
        // Preserve selection if still valid
        if let Some(i) = self.selected {
            if i >= self.games.len() {
                self.selected = None;
            }
        }
    }
```

- [ ] **Step 3: Call reload_games in new() when view is Main**

In `RewindApp::new()`, after setting `view`:
```rust
let mut app = Self {
    config,
    games: Vec::new(),
    selected: None,
    view,
    wizard: WizardState::default(),
    toast: None,
    rt,
    settings_username_buf,
    settings_lib_buf: String::new(),
    first_run_lib_buf: String::new(),
};
if !matches!(app.view, View::FirstRun) {
    app.reload_games();
}
app
```

Also call `reload_games` after transitioning from first-run to main in `first_run.rs` — replace the `app.view = View::Main;` line:
```rust
app.view = View::Main;
app.reload_games();
```

And after saving settings in `settings.rs` — replace `app.view = View::Main;`:
```rust
app.view = View::Main;
app.reload_games();
```

- [ ] **Step 4: Run and verify**

Run: `cargo run -p rewind-gui`
Expected: if Steam is installed and libraries are configured, the game list on the left populates with installed games

- [ ] **Step 5: Commit**

```bash
git add rewind-gui/src/app.rs rewind-gui/src/views/first_run.rs rewind-gui/src/views/settings.rs
git commit -m "feat(gui): load games from scanner on startup and after config changes"
```

---

## Task 13: Final wiring — full compile and smoke test

**Files:**
- Modify: as needed to resolve any remaining compile errors

- [ ] **Step 1: Full build**

Run: `cargo build -p rewind-gui`
Expected: builds with zero errors (warnings are acceptable)

Fix any type mismatches or missing imports that surface here.

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 3: Smoke-test the binary**

Run: `cargo run -p rewind-gui`

Walk through:
1. If first-run shows: click "Auto-detect Steam libraries", verify libraries appear, click "Continue"
2. Main screen: game list on left, "Select a game" on right
3. Click a game: detail panel shows name, app ID, status, manifest
4. Click "Open SteamDB": browser opens to `https://www.steamdb.info/app/<id>/`
5. Click "Settings": modal opens, close with "Cancel"
6. Click "Downgrade": wizard opens with SteamDB link and manifest input; input disabled "Download" button until text entered; "Cancel" closes it

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "feat(gui): complete rewind-gui MVP — game list, detail, wizard, settings"
```
