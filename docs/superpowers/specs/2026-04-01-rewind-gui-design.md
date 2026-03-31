# rewind-gui Design Spec
Date: 2026-04-01

## Overview

`rewind-gui` is a standalone graphical front-end for rewind, built with `egui`/`eframe`. It shares `rewind-core` for all business logic and the same data directory as `rewind-cli`, but ships as an independent binary targeted at users who prefer a graphical interface. It runs on Windows, Linux, and macOS.

---

## Workspace Structure

`rewind-gui` is a new crate added to the existing Cargo workspace:

```
rewind-cli/              ← workspace root
  Cargo.toml             ← "rewind-gui" added to members
  rewind-core/           ← unchanged
  rewind-cli/            ← unchanged TUI binary
  rewind-gui/            ← new standalone GUI binary
    Cargo.toml
    src/
      main.rs            ← eframe::run_native entry point
      app.rs             ← RewindApp struct, eframe::App impl
      views/
        game_list.rs     ← left panel / game list
        game_detail.rs   ← right panel / selected game info
        downgrade.rs     ← downgrade wizard overlay
        settings.rs      ← settings screen
        first_run.rs     ← first-run setup screen
```

Both `rewind` (TUI) and `rewind-gui` share `~/.local/share/rewind/` and can coexist.

---

## Dependencies

```toml
[dependencies]
rewind-core = { path = "../rewind-core" }
eframe = "0.31"
egui = "0.31"
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
open = "5"
anyhow = "1"
```

---

## Application State

`RewindApp` is the central state struct held by `eframe`. Rendering is immediate-mode — every frame reads from this struct.

```rust
struct RewindApp {
    config: Config,
    games: Vec<GameEntry>,
    selected: Option<usize>,
    view: View,
    background_task: Option<BackgroundTask>,
    toast: Option<Toast>,
}

enum View {
    Main,
    DowngradeWizard { app_id: u32, step: WizardStep },
    Settings,
    FirstRun,
}

enum BackgroundTask {
    Downloading { progress: Arc<Mutex<DownloadProgress>> },
}
```

Background work (DepotDownloader invocations) runs on a `tokio` runtime spawned at startup. Progress is communicated back to the UI via `Arc<Mutex<DownloadProgress>>` — the render loop polls it each frame. `rewind-core` async functions are called via `tokio::spawn`; on completion they update shared state and egui picks up the change on the next frame.

---

## UI Layout

Two-panel layout with a status bar:

```
┌─────────────────────────────────────────────────────┐
│  rewind                              [Settings] [?]  │
├──────────────────┬──────────────────────────────────┤
│ Games            │  Crimson Desert                  │
│                  │  App ID: 3321460                 │
│ > Crimson Desert │  Status: ▼ Downgraded            │
│   Elden Ring     │  Active:  manifest abc123        │
│   Dark Souls III │  Latest:  manifest def456        │
│                  │  Cached:  abc123, def456         │
│                  │                                  │
│                  │  [Downgrade]  [Restore Latest]   │
│                  │  [Open SteamDB]                  │
├──────────────────┴──────────────────────────────────┤
│ Downloading Crimson Desert... ████░░░░░░  42%        │
└─────────────────────────────────────────────────────┘
```

- **Left panel** — scrollable game list; click to select
- **Right panel** — detail view for selected game; buttons trigger actions or open overlays
- **Status bar** — shows active background task progress, or empty when idle
- **Overlays** — downgrade wizard and settings render as egui modal windows
- **First-run** — full-window replacement of main layout until setup is complete

Default window size: 800×540, resizable, minimum 600×400. No menubar.

---

## Downgrade Wizard

Modal overlay with two steps:

**Step 1 — Pick a manifest:**
- SteamDB depot manifests URL rendered as a clickable link (opens via `open` crate)
- Text input for manifest ID
- "Next" disabled until input is non-empty

**Step 2 — Download & apply:**
- Progress bar polling `Arc<Mutex<DownloadProgress>>`
- Scrollable log area showing last 100 lines of DepotDownloader stdout/stderr
- "Cancel" sends a `CancellationToken` signal to the tokio task
- On success: closes wizard, refreshes game detail panel, shows toast notification

**Version picker** (shown instead of Step 1 when cached versions exist):
- List of cached manifest IDs with radio-button selection
- "Switch" applies instantly via symlink repoint — no Step 2 needed
- "Download new version" drops into Step 1

---

## Distribution

Built via GitHub Actions. Binary named `rewind-gui` to avoid collision with `rewind` (TUI).

| Platform | Artifact | Tool |
|----------|----------|------|
| Windows  | `.msi` installer | `cargo-wix` |
| macOS    | `.dmg` + `.app` bundle | `cargo-bundle` |
| Linux    | `.tar.gz` + `.AppImage` | `cargo-bundle` / AppImage tooling |

No auto-update mechanism in v1. Users download new releases from GitHub manually.

Window icon and app metadata set via `eframe::NativeOptions` and `cargo-bundle` metadata in `Cargo.toml`.

---

## Out of Scope (v1)

- Auto-update
- Dark/light theme toggle (egui default theme only)
- Keyboard shortcuts beyond standard OS conventions
- File browser for Steam library path input (text field only)
