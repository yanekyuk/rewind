# TUI Styling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the rewind TUI with Steam's color palette and display inline game hero images in the detail panel with disk caching and graceful terminal fallback.

**Architecture:** A centralized `theme.rs` module defines all colors/styles used across the 5 screen files. A new `image_cache.rs` module in `rewind-core` handles fetching hero images from Steam CDN and caching them to disk. The `ratatui-image` crate auto-detects terminal image protocol support (Kitty/Sixel/iTerm2) and falls back to not rendering images when unsupported.

**Tech Stack:** Rust, ratatui 0.29, ratatui-image, image, reqwest (already in rewind-core), dirs (already in rewind-core), tokio

---

### Task 1: Add dependencies

**Files:**
- Modify: `rewind-cli/Cargo.toml`

- [ ] **Step 1: Add ratatui-image and image to rewind-cli**

Add these dependencies to `rewind-cli/Cargo.toml`:

```toml
[dependencies]
rewind-core = { path = "../rewind-core" }
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
open = "5"
anyhow = "1"
ratatui-image = "6"
image = "0.25"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/Cargo.toml Cargo.lock
git commit -m "chore: add ratatui-image and image dependencies"
```

---

### Task 2: Create theme module

**Files:**
- Create: `rewind-cli/src/ui/theme.rs`
- Modify: `rewind-cli/src/ui/mod.rs`

- [ ] **Step 1: Create theme.rs with Steam color palette**

Create `rewind-cli/src/ui/theme.rs`:

```rust
use ratatui::style::{Color, Modifier, Style};

// Steam color palette
pub const BASE_BG: Color = Color::Rgb(27, 40, 56);       // #1b2838
pub const PANEL_BG: Color = Color::Rgb(42, 71, 94);      // #2a475e
pub const ACCENT: Color = Color::Rgb(102, 192, 244);      // #66c0f4
pub const TEXT_PRIMARY: Color = Color::Rgb(199, 213, 224); // #c7d5e0
pub const TEXT_SECONDARY: Color = Color::Rgb(143, 152, 160); // #8f98a0
pub const SUCCESS: Color = Color::Rgb(91, 163, 43);       // #5ba32b
pub const WARNING: Color = Color::Rgb(229, 160, 13);      // #e5a00d
pub const ERROR: Color = Color::Rgb(195, 60, 60);         // #c33c3c
pub const SELECTED_BG: Color = Color::Rgb(61, 108, 142);  // #3d6c8e

// Pre-built styles
pub fn title() -> Style {
    Style::default()
        .fg(ACCENT)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn text() -> Style {
    Style::default().fg(TEXT_PRIMARY).bg(BASE_BG)
}

pub fn text_secondary() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn border() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn border_accent() -> Style {
    Style::default().fg(ACCENT).bg(BASE_BG)
}

pub fn border_focused() -> Style {
    Style::default()
        .fg(WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn selected() -> Style {
    Style::default().fg(TEXT_PRIMARY).bg(SELECTED_BG)
}

pub fn status_success() -> Style {
    Style::default().fg(SUCCESS).bg(BASE_BG)
}

pub fn status_warning() -> Style {
    Style::default()
        .fg(WARNING)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn status_error() -> Style {
    Style::default().fg(ERROR).bg(BASE_BG)
}

pub fn input_active() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .bg(BASE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn input_inactive() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

pub fn help_bar() -> Style {
    Style::default().fg(TEXT_SECONDARY).bg(BASE_BG)
}

/// Background fill style for areas that should show the base background.
pub fn base_bg() -> Style {
    Style::default().bg(BASE_BG)
}

/// Background fill style for panel areas.
pub fn panel_bg() -> Style {
    Style::default().bg(PANEL_BG)
}
```

- [ ] **Step 2: Register theme module in mod.rs**

In `rewind-cli/src/ui/mod.rs`, add this line after the existing module declarations:

```rust
pub mod theme;
```

So the top of the file becomes:

```rust
pub mod downgrade_wizard;
pub mod first_run;
pub mod main_screen;
pub mod settings;
pub mod theme;
pub mod version_picker;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors (theme module is defined but not yet used)

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/ui/theme.rs rewind-cli/src/ui/mod.rs
git commit -m "feat: add centralized Steam color theme module"
```

---

### Task 3: Apply theme to main_screen.rs

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs`

- [ ] **Step 1: Replace all hardcoded colors with theme references**

Replace the full contents of `rewind-cli/src/ui/main_screen.rs` with:

```rust
use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use rewind_core::steamdb;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fill entire area with base background
    f.render_widget(ratatui::widgets::Clear, area);
    let bg = Paragraph::new("").style(theme::base_bg());
    f.render_widget(bg, area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Title bar
    let title = Paragraph::new(" rewind — Steam Version Manager ")
        .style(theme::title());
    f.render_widget(title, outer[0]);

    // Content
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    draw_game_list(f, app, content[0]);
    draw_detail_panel(f, app, content[1]);

    // Status bar
    let status = Paragraph::new(
        " [↑↓/jk] navigate  [D] downgrade  [U] upgrade  [L] lock  [O] SteamDB  [S] settings  [Q] quit ",
    )
    .style(theme::help_bar());
    f.render_widget(status, outer[2]);
}

fn draw_game_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .installed_games
        .iter()
        .enumerate()
        .map(|(i, game)| {
            let entry = app.games_config.games.iter().find(|e| e.app_id == game.app_id);
            let indicator = match entry {
                Some(e) if e.active_manifest_id != e.latest_manifest_id => "▼ ",
                Some(_) => "✓ ",
                None => "  ",
            };

            let style = if i == app.selected_game_index {
                theme::selected()
            } else {
                theme::text()
            };

            ListItem::new(format!("{}{}", indicator, game.name)).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" GAMES ")
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border())
        .style(theme::base_bg());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(game) = app.selected_game() else {
        let msg = Paragraph::new(
            "No games found.\nPress [S] to add a Steam library.",
        )
        .style(theme::text_secondary());
        f.render_widget(msg, inner);
        return;
    };

    let entry = app.games_config.games.iter().find(|e| e.app_id == game.app_id);

    let status_line = match entry {
        Some(e) if e.acf_locked && e.active_manifest_id != e.latest_manifest_id => {
            "▼ Downgraded (locked)"
        }
        Some(e) if e.acf_locked => "✓ Up to date (locked)",
        Some(_) => "  Managed",
        None => "  Unmanaged",
    };

    let active_manifest = entry
        .map(|e| e.active_manifest_id.as_str())
        .unwrap_or(game.manifest_id.as_str());

    let cached_list = entry
        .map(|e| e.cached_manifest_ids.join(", "))
        .unwrap_or_else(|| "none".into());

    let steamdb_url = steamdb::depot_manifests_url(game.depot_id);

    let text = format!(
        "  {name}\n  App ID:  {app_id}\n  Depot:   {depot_id}\n\n  Status:  {status}\n  Active:  {active}\n  Cached:  {cached}\n\n  SteamDB: {url}\n\n  [D] Downgrade / switch version\n  [U] Upgrade / switch version\n  [L] Toggle ACF lock\n  [O] Open app on SteamDB",
        name = game.name,
        app_id = game.app_id,
        depot_id = game.depot_id,
        status = status_line,
        active = active_manifest,
        cached = cached_list,
        url = steamdb_url,
    );

    let para = Paragraph::new(text)
        .style(theme::text())
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/main_screen.rs
git commit -m "style: apply Steam theme to main screen"
```

---

### Task 4: Apply theme to first_run.rs

**Files:**
- Modify: `rewind-cli/src/ui/first_run.rs`

- [ ] **Step 1: Replace all hardcoded colors with theme references**

Replace the full contents of `rewind-cli/src/ui/first_run.rs` with:

```rust
use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, _app: &App) {
    // Fill background
    let bg = Paragraph::new("").style(theme::base_bg());
    f.render_widget(bg, f.area());

    let area = centered_rect(60, 14, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Welcome to rewind ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = "rewind manages your Steam game versions.\n\n\
        It uses DepotDownloader to fetch previous versions\n\
        and switches between them instantly via symlinks.\n\n\
        Press [Enter] to configure your Steam libraries.\n\
        Press [Q] to quit.";

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(theme::text());

    f.render_widget(paragraph, inner.inner(Margin { horizontal: 1, vertical: 1 }));
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/first_run.rs
git commit -m "style: apply Steam theme to first run screen"
```

---

### Task 5: Apply theme to downgrade_wizard.rs

**Files:**
- Modify: `rewind-cli/src/ui/downgrade_wizard.rs`

- [ ] **Step 1: Replace all hardcoded colors with theme references**

Replace the full contents of `rewind-cli/src/ui/downgrade_wizard.rs` with:

```rust
use crate::app::{App, StepStatus};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(70, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Downgrade Game ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_focused())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    if app.wizard_state.is_downloading || !app.wizard_state.steps.is_empty() {
        draw_download_view(f, app, content);
    } else {
        draw_input_view(f, app, content);
    }
}

/// The initial view: SteamDB URL, manifest input, output log, help line.
fn draw_input_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // SteamDB URL
    let url_block = Block::default()
        .title(" 1. Open this URL in your browser to find the manifest ID ")
        .borders(Borders::ALL)
        .border_style(theme::border())
        .style(theme::base_bg());
    let url_para = Paragraph::new(app.wizard_state.steamdb_url.as_str())
        .style(Style::default().fg(theme::ACCENT).bg(theme::BASE_BG))
        .block(url_block);
    f.render_widget(url_para, layout[0]);

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
        .title(" 2. Enter target manifest ID then press [Enter] ")
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
        vec![ListItem::new(err.as_str()).style(theme::status_error())]
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
        " [O] open SteamDB in browser   [Esc] cancel   [Ctrl+C] quit "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}

/// The download-in-progress view: steps on top, DepotDownloader output below.
fn draw_download_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let step_count = app.wizard_state.steps.len() as u16;

    let prompt_height = if app.wizard_state.prompt_input.is_some() {
        3u16
    } else {
        0
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(step_count + 1), // steps + top margin
            Constraint::Min(5),                 // DepotDownloader pane
            Constraint::Length(prompt_height),   // credential input (0 if hidden)
            Constraint::Length(1),               // help line
        ])
        .split(area);

    // --- Steps ---
    let step_items: Vec<ListItem> = app
        .wizard_state
        .steps
        .iter()
        .map(|(kind, status)| {
            let (icon, style) = match status {
                StepStatus::Pending => (
                    "[ ]",
                    theme::text_secondary(),
                ),
                StepStatus::InProgress => (
                    "[\u{2026}]",
                    theme::status_warning(),
                ),
                StepStatus::Done => (
                    "[\u{2713}]",
                    theme::status_success(),
                ),
                StepStatus::Failed(_) => (
                    "[\u{2717}]",
                    theme::status_error(),
                ),
            };
            ListItem::new(format!(" {} {}", icon, kind.label())).style(style)
        })
        .collect();
    let step_list = List::new(step_items);
    f.render_widget(step_list, layout[0]);

    // --- DepotDownloader output pane ---
    let depot_block = Block::default()
        .title(" DepotDownloader ")
        .borders(Borders::ALL)
        .border_style(theme::border())
        .style(theme::base_bg());

    let depot_inner_height = depot_block.inner(layout[1]).height as usize;
    let depot_items: Vec<ListItem> = app
        .wizard_state
        .depot_lines
        .iter()
        .rev()
        .take(depot_inner_height)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|l| ListItem::new(l.as_str()).style(theme::text_secondary()))
        .collect();

    let depot_list = List::new(depot_items).block(depot_block);
    f.render_widget(depot_list, layout[1]);

    // --- Credential prompt input (if active) ---
    if let Some(ref input) = app.wizard_state.prompt_input {
        let label = app
            .wizard_state
            .prompt_label
            .as_deref()
            .unwrap_or("Input required:");
        let is_password = label.to_lowercase().contains("password");
        let display_text = if is_password {
            format!("{}\u{2588}", "*".repeat(input.len()))
        } else {
            format!("{}\u{2588}", input)
        };
        let prompt_block = Block::default()
            .title(format!(" {} ", label))
            .borders(Borders::ALL)
            .border_style(theme::border_accent())
            .style(theme::base_bg());
        let prompt_para = Paragraph::new(display_text)
            .style(theme::input_active())
            .block(prompt_block);
        f.render_widget(prompt_para, layout[2]);
    }

    // --- Help line ---
    let help_text = if app
        .wizard_state
        .error
        .as_ref()
        .map(|e| e.contains("[R]"))
        .unwrap_or(false)
    {
        " [R] restart in terminal   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [Esc] cancel   [Ctrl+C] quit "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/downgrade_wizard.rs
git commit -m "style: apply Steam theme to downgrade wizard"
```

---

### Task 6: Apply theme to version_picker.rs

**Files:**
- Modify: `rewind-cli/src/ui/version_picker.rs`

- [ ] **Step 1: Replace all hardcoded colors with theme references**

Replace the full contents of `rewind-cli/src/ui/version_picker.rs` with:

```rust
use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));

    let cached = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.as_slice())
        .unwrap_or(&[]);

    if cached.is_empty() {
        let msg = Paragraph::new("No cached versions found.\nUse [D] to downgrade first.")
            .alignment(Alignment::Center)
            .style(theme::text_secondary());
        f.render_widget(msg, layout[0]);
    } else {
        let active = app
            .selected_game_entry()
            .map(|e| e.active_manifest_id.as_str())
            .unwrap_or("");

        let items: Vec<ListItem> = cached
            .iter()
            .enumerate()
            .map(|(i, manifest_id)| {
                let is_active = manifest_id == active;
                let label = if is_active {
                    format!("● {} (current)", manifest_id)
                } else {
                    format!("  {}", manifest_id)
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
        f.render_widget(list, layout[0]);
    }

    let help = Paragraph::new(" [↑↓] select   [Enter] switch   [Esc] cancel ")
        .style(theme::help_bar());
    f.render_widget(help, layout[1]);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/version_picker.rs
git commit -m "style: apply Steam theme to version picker"
```

---

### Task 7: Apply theme to settings.rs

**Files:**
- Modify: `rewind-cli/src/ui/settings.rs`

- [ ] **Step 1: Replace all hardcoded colors with theme references**

Replace the full contents of `rewind-cli/src/ui/settings.rs` with:

```rust
use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fill background
    let bg = Paragraph::new("").style(theme::base_bg());
    f.render_widget(bg, area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(" Settings ")
        .alignment(Alignment::Center)
        .style(theme::title());
    f.render_widget(title, outer[0]);

    let content = outer[1].inner(Margin { horizontal: 2, vertical: 1 });

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(content);

    // Username input
    let username_focused = app.settings_state.focused_field == 0;
    let username_border_style = if username_focused {
        theme::border_focused()
    } else {
        theme::border()
    };
    let cursor = if username_focused { "█" } else { "" };
    let username_block = Block::default()
        .title(" Steam Username ")
        .borders(Borders::ALL)
        .border_style(username_border_style)
        .style(theme::base_bg());
    let username_para =
        Paragraph::new(format!("{}{}", app.settings_state.username_input, cursor))
            .style(if username_focused { theme::input_active() } else { theme::input_inactive() })
            .block(username_block);
    f.render_widget(username_para, sections[0]);

    // Library path input
    let library_focused = app.settings_state.focused_field == 1;
    let library_border_style = if library_focused {
        theme::border_focused()
    } else {
        theme::border()
    };
    let lib_cursor = if library_focused { "█" } else { "" };
    let library_block = Block::default()
        .title(" Add Steam Library Path (Enter to add) ")
        .borders(Borders::ALL)
        .border_style(library_border_style)
        .style(theme::base_bg());
    let library_para =
        Paragraph::new(format!("{}{}", app.settings_state.library_input, lib_cursor))
            .style(if library_focused { theme::input_active() } else { theme::input_inactive() })
            .block(library_block);
    f.render_widget(library_para, sections[2]);

    // Library list
    let lib_items: Vec<ListItem> = app
        .config
        .libraries
        .iter()
        .map(|l| ListItem::new(format!("  {}", l.path.display())).style(theme::text()))
        .collect();

    let lib_list_block = Block::default()
        .title(" Configured Libraries ")
        .borders(Borders::ALL)
        .border_style(theme::border())
        .style(theme::base_bg());

    if lib_items.is_empty() {
        let msg = Paragraph::new("  No libraries configured yet.")
            .style(theme::text_secondary())
            .block(lib_list_block);
        f.render_widget(msg, sections[3]);
    } else {
        let list = List::new(lib_items).block(lib_list_block);
        f.render_widget(list, sections[3]);
    }

    // Status bar
    let help = Paragraph::new(
        " [Tab] switch field   [Enter] save/add   [Esc] back ",
    )
    .style(theme::help_bar());
    f.render_widget(help, outer[2]);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/settings.rs
git commit -m "style: apply Steam theme to settings screen"
```

---

### Task 8: Create image cache module in rewind-core

**Files:**
- Create: `rewind-core/src/image_cache.rs`
- Modify: `rewind-core/src/lib.rs`

- [ ] **Step 1: Write test for image cache path generation and disk cache logic**

Create `rewind-core/tests/image_cache_test.rs`:

```rust
use std::path::PathBuf;
use rewind_core::image_cache;

#[test]
fn cache_path_uses_appid() {
    let dir = PathBuf::from("/tmp/rewind-test-images");
    let path = image_cache::hero_cache_path(&dir, 12345);
    assert_eq!(path, dir.join("12345_hero.jpg"));
}

#[tokio::test]
async fn load_cached_returns_none_when_missing() {
    let dir = PathBuf::from("/tmp/rewind-test-nonexistent-dir");
    let result = image_cache::load_cached_hero(&dir, 99999);
    assert!(result.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rewind-core --test image_cache_test`
Expected: FAIL — `image_cache` module does not exist yet

- [ ] **Step 3: Create image_cache.rs**

Create `rewind-core/src/image_cache.rs`:

```rust
use std::path::{Path, PathBuf};

const STEAM_CDN_BASE: &str = "https://cdn.akamai.steamstatic.com/steam/apps";

/// Returns the expected cache file path for a game's hero image.
pub fn hero_cache_path(cache_dir: &Path, app_id: u32) -> PathBuf {
    cache_dir.join(format!("{}_hero.jpg", app_id))
}

/// Returns the Steam CDN URL for a game's library hero image.
pub fn hero_url(app_id: u32) -> String {
    format!("{}/{}/library_hero.jpg", STEAM_CDN_BASE, app_id)
}

/// Returns the image directory inside the rewind data dir, creating it if needed.
pub fn images_dir() -> Result<PathBuf, crate::config::ConfigError> {
    let dir = crate::config::data_dir()?.join("images");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Loads a cached hero image from disk, returning the raw bytes if present.
pub fn load_cached_hero(cache_dir: &Path, app_id: u32) -> Option<Vec<u8>> {
    let path = hero_cache_path(cache_dir, app_id);
    std::fs::read(&path).ok()
}

/// Fetches the hero image from Steam CDN and saves it to the cache directory.
/// Returns the raw image bytes on success.
pub async fn fetch_and_cache_hero(
    cache_dir: &Path,
    app_id: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let url = hero_url(app_id);
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()).into());
    }

    let bytes = response.bytes().await?.to_vec();
    let path = hero_cache_path(cache_dir, app_id);
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(&path, &bytes)?;
    Ok(bytes)
}
```

- [ ] **Step 4: Register module in lib.rs**

In `rewind-core/src/lib.rs`, add:

```rust
pub mod image_cache;
```

So the file becomes:

```rust
pub mod cache;
pub mod config;
pub mod depot;
pub mod image_cache;
pub mod immutability;
pub mod patcher;
pub mod scanner;
pub mod steamdb;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p rewind-core --test image_cache_test`
Expected: PASS — both tests pass

- [ ] **Step 6: Commit**

```bash
git add rewind-core/src/image_cache.rs rewind-core/src/lib.rs rewind-core/tests/image_cache_test.rs
git commit -m "feat: add image cache module for Steam CDN hero images"
```

---

### Task 9: Add image state to App and integrate with main event loop

**Files:**
- Modify: `rewind-cli/src/app.rs`
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Add image state to App struct**

In `rewind-cli/src/app.rs`, add these imports at the top:

```rust
use std::collections::HashMap;
```

Add a new struct before the `App` struct:

```rust
#[derive(Debug, Default)]
pub struct ImageState {
    /// Loaded hero images keyed by app_id. Value is the decoded DynamicImage.
    pub loaded_images: HashMap<u32, image::DynamicImage>,
    /// App IDs currently being fetched (to avoid duplicate requests).
    pub pending_fetches: std::collections::HashSet<u32>,
}
```

Add the field to the `App` struct (after `depot_kill`):

```rust
    pub image_state: ImageState,
```

In `App::new()`, add initialization:

```rust
            image_state: ImageState::default(),
```

- [ ] **Step 2: Add image dependency to app.rs imports**

The `image` crate is already in Cargo.toml from Task 1. The `image::DynamicImage` type is used in `ImageState`. Add at the top of `app.rs`:

```rust
use std::collections::HashMap;
```

Note: `image` crate types are referenced via full path (`image::DynamicImage`) so no additional `use` is needed beyond `HashMap`.

- [ ] **Step 3: Add image fetching to the event loop in main.rs**

In `rewind-cli/src/main.rs`, add an import at the top:

```rust
use std::collections::HashSet;
```

In the `run` function, after the line `repair_stale_locks(&mut app);` and before the `loop {` line, add image protocol detection and the image channel:

```rust
    // Image loading channel
    let (image_tx, mut image_rx) = mpsc::channel::<(u32, Option<image::DynamicImage>)>(16);

    // Detect terminal image protocol support
    let image_protocol = ratatui_image::picker::Picker::from_query_stdio();
```

Inside the `loop { ... }`, right after the progress message polling block (after the `for msg in progress_msgs { ... }` block ends), add image reception:

```rust
        // Receive loaded images
        while let Ok((app_id, maybe_img)) = image_rx.try_recv() {
            app.image_state.pending_fetches.remove(&app_id);
            if let Some(img) = maybe_img {
                app.image_state.loaded_images.insert(app_id, img);
            }
        }

        // Trigger image fetch for selected game if needed
        if let Some(game) = app.selected_game() {
            let app_id = game.app_id;
            if !app.image_state.loaded_images.contains_key(&app_id)
                && !app.image_state.pending_fetches.contains(&app_id)
            {
                app.image_state.pending_fetches.insert(app_id);
                let tx = image_tx.clone();
                tokio::spawn(async move {
                    let result = async {
                        let images_dir = rewind_core::image_cache::images_dir()?;
                        let bytes = match rewind_core::image_cache::load_cached_hero(&images_dir, app_id) {
                            Some(b) => b,
                            None => rewind_core::image_cache::fetch_and_cache_hero(&images_dir, app_id).await?,
                        };
                        let img = image::load_from_memory(&bytes)?;
                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(img)
                    }
                    .await;
                    let _ = tx.send((app_id, result.ok())).await;
                });
            }
        }
```

- [ ] **Step 4: Store the picker in App**

Add to `App` struct in `app.rs`:

```rust
    pub image_picker: Option<ratatui_image::picker::Picker>,
```

In `App::new()`, add:

```rust
            image_picker: None,
```

Then in `main.rs`, after creating `image_protocol`, set it on the app:

```rust
    app.image_picker = image_protocol.ok();
```

Remove the standalone `let image_protocol = ...` variable (it's now stored in `app.image_picker`).

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors (warnings about unused `image_picker` in draw are fine for now)

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/app.rs rewind-cli/src/main.rs
git commit -m "feat: add image state and async image loading to app"
```

---

### Task 10: Render hero image in detail panel

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs`

- [ ] **Step 1: Update draw_detail_panel to render the hero image**

In `rewind-cli/src/ui/main_screen.rs`, update the imports at the top:

```rust
use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use rewind_core::steamdb;
```

Replace the `draw_detail_panel` function with this version that includes image rendering:

```rust
fn draw_detail_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(game) = app.selected_game() else {
        let msg = Paragraph::new(
            "No games found.\nPress [S] to add a Steam library.",
        )
        .style(theme::text_secondary());
        f.render_widget(msg, inner);
        return;
    };

    // Determine if we have an image to show
    let has_image = app.image_picker.is_some()
        && app.image_state.loaded_images.contains_key(&game.app_id);

    let (image_area, text_area) = if has_image {
        // Split: top ~40% for image, bottom for text
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner);
        (Some(layout[0]), layout[1])
    } else {
        (None, inner)
    };

    // Render image if available
    if let Some(img_area) = image_area {
        if let Some(img) = app.image_state.loaded_images.get(&game.app_id) {
            if let Some(ref picker) = app.image_picker {
                let image_widget = ratatui_image::StatefulImage::new(None);
                let mut state = picker.new_resize_protocol(img.clone());
                f.render_stateful_widget(image_widget, img_area, &mut state);
            }
        }
    }

    // Text info
    let entry = app.games_config.games.iter().find(|e| e.app_id == game.app_id);

    let status_line = match entry {
        Some(e) if e.acf_locked && e.active_manifest_id != e.latest_manifest_id => {
            "▼ Downgraded (locked)"
        }
        Some(e) if e.acf_locked => "✓ Up to date (locked)",
        Some(_) => "  Managed",
        None => "  Unmanaged",
    };

    let active_manifest = entry
        .map(|e| e.active_manifest_id.as_str())
        .unwrap_or(game.manifest_id.as_str());

    let cached_list = entry
        .map(|e| e.cached_manifest_ids.join(", "))
        .unwrap_or_else(|| "none".into());

    let steamdb_url = steamdb::depot_manifests_url(game.depot_id);

    let text = format!(
        "  {name}\n  App ID:  {app_id}\n  Depot:   {depot_id}\n\n  Status:  {status}\n  Active:  {active}\n  Cached:  {cached}\n\n  SteamDB: {url}\n\n  [D] Downgrade / switch version\n  [U] Upgrade / switch version\n  [L] Toggle ACF lock\n  [O] Open app on SteamDB",
        name = game.name,
        app_id = game.app_id,
        depot_id = game.depot_id,
        status = status_line,
        active = active_manifest,
        cached = cached_list,
        url = steamdb_url,
    );

    let para = Paragraph::new(text)
        .style(theme::text())
        .wrap(Wrap { trim: false });
    f.render_widget(para, text_area);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p rewind-cli`
Expected: compiles without errors

- [ ] **Step 3: Test manually**

Run: `cargo run` and navigate between games. Verify:
- Steam dark theme colors appear across all screens
- Hero images load asynchronously in the detail panel (if terminal supports it)
- Detail panel shows text-only layout gracefully when images aren't supported
- All modals (downgrade wizard, version picker, settings) use the new palette

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/ui/main_screen.rs
git commit -m "feat: render hero images in detail panel with fallback"
```

---

### Task 11: Final cleanup and version bump

**Files:**
- Modify: `rewind-cli/Cargo.toml`
- Modify: `rewind-core/Cargo.toml`

- [ ] **Step 1: Bump versions**

In both `rewind-cli/Cargo.toml` and `rewind-core/Cargo.toml`, bump the version from `0.2.0` to `0.3.0`.

- [ ] **Step 2: Verify everything compiles and tests pass**

Run: `cargo build && cargo test`
Expected: builds cleanly, all tests pass (except the known macOS immutability test failures)

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/Cargo.toml rewind-core/Cargo.toml
git commit -m "chore: bump version to 0.3.0"
```
