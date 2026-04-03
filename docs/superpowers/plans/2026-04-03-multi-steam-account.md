# Multi-Steam Account Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow Rewind to detect multiple Steam accounts, let the user pick a preferred one (stored as SteamID64 in `config.toml`), and use it when reading per-user data like launch options.

**Architecture:** New `SteamAccount` type and two new functions in `rewind-core/src/scanner.rs` handle account detection and userdata directory resolution. `Config` gains a `preferred_steam_account: Option<u64>` field. The CLI grows a first-run AccountPicker step and a new Settings field for post-first-run switching.

**Tech Stack:** Rust, ratatui/crossterm (TUI), serde/toml (config), steamlocate (Steam directory detection), existing VDF line-parser pattern already in scanner.rs.

---

## File Map

| File | Change |
|------|--------|
| `rewind-core/src/scanner.rs` | Add `SteamAccount`, `read_steam_accounts()`, `userdata_dir_for_account()`, `parse_loginusers_vdf()`; update `read_launch_options()` and `find_launch_options()` signatures |
| `rewind-core/src/config.rs` | Add `preferred_steam_account: Option<u64>` to `Config` |
| `rewind-cli/src/app.rs` | Add `FirstRunStep`, `FirstRunState`; update `SettingsState`; add `is_first_run`, `first_run_state` to `App` |
| `rewind-cli/src/ui/first_run.rs` | Add `AccountPicker` step rendering |
| `rewind-cli/src/ui/settings.rs` | Add Steam Account field (field index 2) |
| `rewind-cli/src/main.rs` | Update `handle_first_run`, `handle_main` (open settings), `handle_settings`; add `open_settings()` helper and `current_account_index()` helper |
| `rewind-cli/src/ui/main_screen.rs` | Update `find_launch_options` call to pass `app.config.preferred_steam_account` |

---

## Task 1: SteamAccount type + read_steam_accounts()

**Files:**
- Modify: `rewind-core/src/scanner.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `rewind-core/src/scanner.rs`:

```rust
#[test]
fn read_steam_accounts_parses_loginusers_vdf() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();
    let vdf = "\"users\"\n{\n\t\"76561198858787719\"\n\t{\n\t\t\"AccountName\"\t\t\"yanekyuk\"\n\t\t\"PersonaName\"\t\t\"yanekeke\"\n\t}\n\t\"76561199258820835\"\n\t{\n\t\t\"AccountName\"\t\t\"chwantt\"\n\t\t\"PersonaName\"\t\t\"chwantt\"\n\t}\n}";
    fs::write(config_dir.join("loginusers.vdf"), vdf).unwrap();
    let accounts = read_steam_accounts(tmp.path());
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].id, 76561198858787719u64);
    assert_eq!(accounts[0].account_name, "yanekyuk");
    assert_eq!(accounts[0].persona_name, "yanekeke");
    assert_eq!(accounts[1].id, 76561199258820835u64);
}

#[test]
fn read_steam_accounts_returns_empty_when_file_missing() {
    let tmp = TempDir::new().unwrap();
    assert!(read_steam_accounts(tmp.path()).is_empty());
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test -p rewind-core read_steam_accounts 2>&1 | tail -5
```
Expected: `error[E0425]: cannot find function 'read_steam_accounts'`

- [ ] **Step 3: Add SteamAccount struct and read_steam_accounts() to scanner.rs**

After the existing `InstalledGame` struct (after line 24), add:

```rust
/// A Steam account found in loginusers.vdf.
#[derive(Debug, Clone)]
pub struct SteamAccount {
    pub id: u64,               // SteamID64
    pub persona_name: String,  // Display name (e.g. "yanekeke")
    pub account_name: String,  // Login name (e.g. "yanekyuk")
}

/// Read all Steam accounts from `<steam_root>/config/loginusers.vdf`.
/// Returns an empty Vec if the file is missing or unparseable.
pub fn read_steam_accounts(steam_root: &Path) -> Vec<SteamAccount> {
    let path = steam_root.join("config").join("loginusers.vdf");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    parse_loginusers_vdf(&content)
}

fn parse_loginusers_vdf(content: &str) -> Vec<SteamAccount> {
    let mut accounts = Vec::new();
    let mut current_id: Option<u64> = None;
    let mut current_persona: Option<String> = None;
    let mut current_account: Option<String> = None;
    let mut depth = 0i32;
    let mut in_users = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            depth += 1;
            continue;
        }

        if trimmed == "}" {
            // Closing a user entry block (depth 2 → 1)
            if depth == 2 && in_users {
                if let (Some(id), Some(persona), Some(account)) = (
                    current_id.take(),
                    current_persona.take(),
                    current_account.take(),
                ) {
                    accounts.push(SteamAccount { id, persona_name: persona, account_name: account });
                } else {
                    current_id = None;
                    current_persona = None;
                    current_account = None;
                }
            }
            depth -= 1;
            continue;
        }

        if depth == 0 && trimmed == "\"users\"" {
            in_users = true;
            continue;
        }

        // SteamID64 key at depth 1 inside "users"
        if in_users && depth == 1 {
            if let Some(id_str) = extract_quoted_only(trimmed) {
                if let Ok(id) = id_str.parse::<u64>() {
                    current_id = Some(id);
                }
            }
            continue;
        }

        // Fields inside a user block at depth 2
        if in_users && depth == 2 {
            if trimmed.starts_with("\"AccountName\"") {
                let rest = trimmed["\"AccountName\"".len()..].trim();
                if let Some(val) = extract_quoted_only(rest) {
                    current_account = Some(val.to_string());
                }
            } else if trimmed.starts_with("\"PersonaName\"") {
                let rest = trimmed["\"PersonaName\"".len()..].trim();
                if let Some(val) = extract_quoted_only(rest) {
                    current_persona = Some(val.to_string());
                }
            }
        }
    }

    accounts
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```
cargo test -p rewind-core read_steam_accounts 2>&1 | tail -5
```
Expected: `test result: ok. 2 passed`

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/scanner.rs
git commit -m "feat: add SteamAccount type and read_steam_accounts() to scanner"
```

---

## Task 2: userdata_dir_for_account()

**Files:**
- Modify: `rewind-core/src/scanner.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block:

```rust
#[test]
fn userdata_dir_for_account_returns_path_when_exists() {
    let tmp = TempDir::new().unwrap();
    // SteamID64 76561197960265729 → account ID = 76561197960265729 - 76561197960265728 = 1
    let account_dir = tmp.path().join("userdata").join("1");
    fs::create_dir_all(&account_dir).unwrap();
    let result = userdata_dir_for_account(tmp.path(), 76561197960265729u64);
    assert_eq!(result, Some(account_dir));
}

#[test]
fn userdata_dir_for_account_returns_none_when_missing() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("userdata")).unwrap();
    let result = userdata_dir_for_account(tmp.path(), 76561197960265729u64);
    assert!(result.is_none());
}

#[test]
fn userdata_dir_for_account_returns_none_on_underflow() {
    let tmp = TempDir::new().unwrap();
    let result = userdata_dir_for_account(tmp.path(), 0u64);
    assert!(result.is_none());
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test -p rewind-core userdata_dir_for_account 2>&1 | tail -5
```
Expected: `error[E0425]: cannot find function 'userdata_dir_for_account'`

- [ ] **Step 3: Add userdata_dir_for_account() to scanner.rs**

Add after `read_steam_accounts` (before the existing `scan_library`):

```rust
const STEAM_ID64_BASE: u64 = 76561197960265728;

/// Convert a SteamID64 to its userdata directory path.
/// Returns `None` if the directory does not exist or the ID is below the base.
pub fn userdata_dir_for_account(steam_root: &Path, steam_id64: u64) -> Option<PathBuf> {
    let account_id = steam_id64.checked_sub(STEAM_ID64_BASE)?;
    let dir = steam_root.join("userdata").join(account_id.to_string());
    if dir.is_dir() { Some(dir) } else { None }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```
cargo test -p rewind-core userdata_dir_for_account 2>&1 | tail -5
```
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/scanner.rs
git commit -m "feat: add userdata_dir_for_account() to scanner"
```

---

## Task 3: Update read_launch_options() to accept preferred_account

**Files:**
- Modify: `rewind-core/src/scanner.rs`

- [ ] **Step 1: Update existing tests to pass None as third argument**

In the `#[cfg(test)]` block, update the three existing `read_launch_options` tests:

```rust
// read_launch_options_finds_value_in_userdata: change last assertion to:
assert_eq!(
    read_launch_options(tmp.path(), 42, None),
    Some("DXVK_ASYNC=1 %command%".to_string())
);

// read_launch_options_returns_none_when_no_userdata:
assert_eq!(read_launch_options(tmp.path(), 42, None), None);

// read_launch_options_returns_none_when_app_not_found:
assert_eq!(read_launch_options(tmp.path(), 42, None), None);
```

Also add a new test for the preferred-account path:

```rust
#[test]
fn read_launch_options_uses_preferred_account() {
    let tmp = TempDir::new().unwrap();

    // Account ID 1 (SteamID64 76561197960265729): has launch options for app 42
    let acct1_cfg = tmp.path().join("userdata").join("1").join("config");
    fs::create_dir_all(&acct1_cfg).unwrap();
    let vdf_with = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "42"
                    {
                        "LaunchOptions"		"-preferred"
                    }
                }
            }
        }
    }
}"#;
    fs::write(acct1_cfg.join("localconfig.vdf"), vdf_with).unwrap();

    // Account ID 2 (SteamID64 76561197960265730): no launch options for app 42
    let acct2_cfg = tmp.path().join("userdata").join("2").join("config");
    fs::create_dir_all(&acct2_cfg).unwrap();
    let vdf_without = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "42"
                    {
                    }
                }
            }
        }
    }
}"#;
    fs::write(acct2_cfg.join("localconfig.vdf"), vdf_without).unwrap();

    // Preferred = account 1 → should find "-preferred"
    assert_eq!(
        read_launch_options(tmp.path(), 42, Some(76561197960265729u64)),
        Some("-preferred".to_string())
    );

    // Preferred = account 2 → no options; fall back to heuristic (most recent modified)
    // Both VDFs exist; account 1 was written last so heuristic picks it
    // Result depends on mtime, but the key check is: preferred account 2 has no options,
    // so we fall back, and the heuristic finds account 1's options.
    let result = read_launch_options(tmp.path(), 42, Some(76561197960265730u64));
    // Falls back to heuristic which finds account 1's VDF
    assert_eq!(result, Some("-preferred".to_string()));

    // No preference → heuristic (same result)
    assert_eq!(
        read_launch_options(tmp.path(), 42, None),
        Some("-preferred".to_string())
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail due to signature mismatch**

```
cargo test -p rewind-core read_launch_options 2>&1 | tail -10
```
Expected: compile errors about wrong number of arguments.

- [ ] **Step 3: Update read_launch_options() signature and implementation**

Replace the existing `read_launch_options` function (lines 242–266) with:

```rust
/// Read launch options for a game. If `preferred_account` is `Some(id)`, targets that
/// account's `localconfig.vdf` first. Falls back to the most-recently-modified heuristic
/// if the preferred account's directory doesn't exist or the app isn't listed there.
pub fn read_launch_options(
    steam_root: &Path,
    app_id: u32,
    preferred_account: Option<u64>,
) -> Option<String> {
    // Try preferred account's directory first.
    if let Some(id) = preferred_account {
        if let Some(account_dir) = userdata_dir_for_account(steam_root, id) {
            let vdf_path = account_dir.join("config").join("localconfig.vdf");
            if let Ok(content) = std::fs::read_to_string(&vdf_path) {
                if let Some(opts) = extract_launch_options_from_vdf(&content, app_id) {
                    return Some(opts);
                }
            }
        }
    }
    // Fall back to most-recently-modified heuristic.
    read_launch_options_heuristic(steam_root, app_id)
}

fn read_launch_options_heuristic(steam_root: &Path, app_id: u32) -> Option<String> {
    let userdata = steam_root.join("userdata");
    let entries = std::fs::read_dir(&userdata).ok()?;

    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let vdf_path = entry.path().join("config").join("localconfig.vdf");
        if vdf_path.exists() {
            if let Ok(meta) = std::fs::metadata(&vdf_path) {
                if let Ok(mtime) = meta.modified() {
                    if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                        best = Some((mtime, vdf_path));
                    }
                }
            }
        }
    }

    let (_, vdf_path) = best?;
    let content = std::fs::read_to_string(&vdf_path).ok()?;
    extract_launch_options_from_vdf(&content, app_id)
}
```

Also update `find_launch_options` (lines 268–274) to accept and forward `preferred_account`:

```rust
/// Convenience wrapper: resolves Steam root via steamlocate, then calls `read_launch_options`.
pub fn find_launch_options(app_id: u32, preferred_account: Option<u64>) -> Option<String> {
    use steamlocate::SteamDir;
    let steam_dir = SteamDir::locate().ok()?;
    read_launch_options(steam_dir.path(), app_id, preferred_account)
}
```

- [ ] **Step 4: Run all scanner tests**

```
cargo test -p rewind-core 2>&1 | tail -10
```
Expected: `test result: ok. N passed`

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/scanner.rs
git commit -m "feat: update read_launch_options to accept preferred_account parameter"
```

---

## Task 4: Config — add preferred_steam_account field

**Files:**
- Modify: `rewind-core/src/config.rs`

- [ ] **Step 1: Update config_roundtrip test to cover the new field**

In the `config_roundtrip` test, update the Config construction and assertions:

```rust
#[test]
fn config_roundtrip() {
    let config = Config {
        steam_username: Some("testuser".into()),
        libraries: vec![Library { path: "/tmp/steamapps".into() }],
        preferred_steam_account: Some(76561198858787719u64),
    };
    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.steam_username.as_deref(), Some("testuser"));
    assert_eq!(parsed.libraries.len(), 1);
    assert_eq!(parsed.preferred_steam_account, Some(76561198858787719u64));
}
```

Also add a test that an old config without the field deserializes to None:

```rust
#[test]
fn config_without_preferred_account_defaults_to_none() {
    let toml_str = r#"steam_username = "user""#;
    let parsed: Config = toml::from_str(toml_str).unwrap();
    assert!(parsed.preferred_steam_account.is_none());
}
```

- [ ] **Step 2: Run to confirm test fails**

```
cargo test -p rewind-core config_roundtrip 2>&1 | tail -5
```
Expected: struct literal error (missing field).

- [ ] **Step 3: Add the field to Config**

Update `Config` in `rewind-core/src/config.rs`:

```rust
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub steam_username: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub preferred_steam_account: Option<u64>,
}
```

- [ ] **Step 4: Run tests**

```
cargo test -p rewind-core 2>&1 | tail -5
```
Expected: `test result: ok. N passed`

- [ ] **Step 5: Commit**

```bash
git add rewind-core/src/config.rs
git commit -m "feat: add preferred_steam_account field to Config"
```

---

## Task 5: App state — FirstRunState, updated SettingsState, is_first_run

**Files:**
- Modify: `rewind-cli/src/app.rs`

- [ ] **Step 1: Add imports and new types to app.rs**

At the top of `rewind-cli/src/app.rs`, add `SteamAccount` to the imports:

```rust
use rewind_core::{
    config::{Config, GameEntry, GamesConfig},
    depot::DepotProgress,
    reshade::ReshadeProgress,
    scanner::{InstalledGame, SteamAccount},
};
```

After the `ReshadeSetupState` struct, add:

```rust
#[derive(Debug, Default, PartialEq)]
pub enum FirstRunStep {
    #[default]
    Welcome,
    AccountPicker,
}

#[derive(Debug, Default)]
pub struct FirstRunState {
    pub step: FirstRunStep,
    pub accounts: Vec<SteamAccount>,  // populated when showing the picker
    pub selected_index: usize,
}
```

- [ ] **Step 2: Update SettingsState**

Replace the existing `SettingsState` struct:

```rust
#[derive(Debug, Default)]
pub struct SettingsState {
    pub username_input: String,
    pub library_input: String,
    pub focused_field: usize,
    /// Accounts loaded from loginusers.vdf when Settings opens. Empty = file missing.
    pub available_accounts: Vec<SteamAccount>,
    /// 0 = Auto (no preference), 1..n = available_accounts[account_index - 1]
    pub account_index: usize,
}
```

- [ ] **Step 3: Add is_first_run and first_run_state to App**

In the `App` struct, add after `should_quit`:

```rust
pub is_first_run: bool,
pub first_run_state: FirstRunState,
```

In `App::new()`, add to the struct literal:

```rust
is_first_run: first_run,
first_run_state: FirstRunState::default(),
```

- [ ] **Step 4: Build to check for compile errors**

```
cargo build -p rewind-cli 2>&1 | grep "^error" | head -20
```
Expected: errors about `SettingsState` struct literal missing `available_accounts`/`account_index` fields in `main.rs`. That's fine — they'll be fixed in Task 8.

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/app.rs
git commit -m "feat: add FirstRunState, update SettingsState and App for multi-account support"
```

---

## Task 6: Wire preferred_steam_account to launch options call

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs`

- [ ] **Step 1: Update the find_launch_options call**

In `rewind-cli/src/ui/main_screen.rs` around line 156, change:

```rust
let opts = rewind_core::scanner::find_launch_options(game_app_id);
```

to:

```rust
let opts = rewind_core::scanner::find_launch_options(
    game_app_id,
    app.config.preferred_steam_account,
);
```

- [ ] **Step 2: Build to confirm it compiles**

```
cargo build -p rewind-cli 2>&1 | grep "^error" | head -10
```
Expected: no errors from main_screen.rs (other tasks' errors may still show).

- [ ] **Step 3: Commit** (after other compile errors in main.rs are also fixed — see Task 8)

This step's commit is bundled into Task 8's commit.

---

## Task 7: Settings UI — Steam Account field

**Files:**
- Modify: `rewind-cli/src/ui/settings.rs`

- [ ] **Step 1: Replace settings.rs with version including the new field**

Replace the full contents of `rewind-cli/src/ui/settings.rs`:

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
            Constraint::Length(3),  // username
            Constraint::Length(1),  // spacer
            Constraint::Length(3),  // library path input
            Constraint::Length(1),  // spacer
            Constraint::Length(3),  // steam account
            Constraint::Min(0),     // libraries list
        ])
        .split(content);

    // Username input
    let username_focused = app.settings_state.focused_field == 0;
    let username_block = Block::default()
        .title(" Steam Username ")
        .borders(Borders::ALL)
        .border_style(if username_focused { theme::border_focused() } else { theme::border() });
    let cursor = if username_focused { "█" } else { "" };
    let username_para =
        Paragraph::new(format!("{}{}", app.settings_state.username_input, cursor))
            .style(if username_focused { theme::input_active() } else { theme::input_inactive() })
            .block(username_block);
    f.render_widget(username_para, sections[0]);

    // Library path input
    let library_focused = app.settings_state.focused_field == 1;
    let library_block = Block::default()
        .title(" Add Steam Library Path (Enter to add) ")
        .borders(Borders::ALL)
        .border_style(if library_focused { theme::border_focused() } else { theme::border() });
    let lib_cursor = if library_focused { "█" } else { "" };
    let library_para =
        Paragraph::new(format!("{}{}", app.settings_state.library_input, lib_cursor))
            .style(if library_focused { theme::input_active() } else { theme::input_inactive() })
            .block(library_block);
    f.render_widget(library_para, sections[2]);

    // Steam Account selector
    let account_focused = app.settings_state.focused_field == 2;
    let account_label = if app.settings_state.available_accounts.is_empty() {
        "Auto (most recent)".to_string()
    } else if app.settings_state.account_index == 0 {
        "◀  Auto (most recent)  ▶".to_string()
    } else {
        let acct = &app.settings_state.available_accounts[app.settings_state.account_index - 1];
        format!("◀  {} ({})  ▶", acct.persona_name, acct.account_name)
    };
    let account_block = Block::default()
        .title(" Steam Account ")
        .borders(Borders::ALL)
        .border_style(if account_focused { theme::border_focused() } else { theme::border() });
    let account_para = Paragraph::new(account_label)
        .style(if account_focused { theme::input_active() } else { theme::input_inactive() })
        .block(account_block);
    f.render_widget(account_para, sections[4]);

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
        .border_style(theme::border());
    if lib_items.is_empty() {
        let msg = Paragraph::new("  No libraries configured yet.")
            .style(theme::text_secondary())
            .block(lib_list_block);
        f.render_widget(msg, sections[5]);
    } else {
        let list = List::new(lib_items).block(lib_list_block);
        f.render_widget(list, sections[5]);
    }

    // Status bar
    let help_text = if app.settings_state.available_accounts.is_empty() {
        " [Tab] switch field   [Enter] save/add   [Esc] back "
    } else {
        " [Tab] switch field   [←/→] change account   [Enter] save/add   [Esc] back "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, outer[2]);
}
```

- [ ] **Step 2: Build to confirm it compiles**

```
cargo build -p rewind-cli 2>&1 | grep "^error" | grep "settings.rs" | head -5
```
Expected: no errors from settings.rs.

- [ ] **Step 3: Commit** (bundled with Task 8's commit after all compile errors resolved)

---

## Task 8: Update main.rs — Settings handler, open_settings helper, handle_first_run

**Files:**
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Add open_settings() helper and current_account_index() helper after the existing imports**

Add after the `use` statements (near the top of main.rs, before `fn main`):

```rust
fn current_account_index(accounts: &[rewind_core::scanner::SteamAccount], preferred: Option<u64>) -> usize {
    match preferred {
        None => 0,
        Some(id) => accounts.iter().position(|a| a.id == id).map(|i| i + 1).unwrap_or(0),
    }
}

fn open_settings(app: &mut App) {
    use steamlocate::SteamDir;
    let available_accounts = SteamDir::locate().ok()
        .map(|sd| rewind_core::scanner::read_steam_accounts(sd.path()))
        .unwrap_or_default();
    let account_index = current_account_index(&available_accounts, app.config.preferred_steam_account);
    app.settings_state = app::SettingsState {
        username_input: app.config.steam_username.clone().unwrap_or_default(),
        library_input: String::new(),
        focused_field: 0,
        available_accounts,
        account_index,
    };
    app.screen = Screen::Settings;
}
```

- [ ] **Step 2: Update handle_first_run() to handle both steps**

Replace the existing `handle_first_run` function:

```rust
fn handle_first_run(app: &mut App, key: KeyCode) {
    match app.first_run_state.step {
        app::FirstRunStep::Welcome => match key {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Enter => open_settings(app),
            _ => {}
        },
        app::FirstRunStep::AccountPicker => match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.first_run_state.selected_index > 0 {
                    app.first_run_state.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.first_run_state.selected_index + 1 < app.first_run_state.accounts.len() {
                    app.first_run_state.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                let selected = app.first_run_state.accounts[app.first_run_state.selected_index].clone();
                app.config.preferred_steam_account = Some(selected.id);
                let _ = config::save_config(&app.config);
                app.is_first_run = false;
                app.screen = Screen::Main;
            }
            _ => {}
        },
    }
}
```

- [ ] **Step 3: Update handle_settings() — account field interaction and first-run routing on Esc**

Replace the existing `handle_settings` function:

```rust
fn handle_settings(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            // Save username
            app.config.steam_username =
                Some(app.settings_state.username_input.clone()).filter(|s| !s.is_empty());
            // Save account preference
            app.config.preferred_steam_account = if app.settings_state.account_index == 0 {
                None
            } else {
                app.settings_state.available_accounts
                    .get(app.settings_state.account_index - 1)
                    .map(|a| a.id)
            };
            let _ = config::save_config(&app.config);

            if app.is_first_run && app.config.preferred_steam_account.is_none() {
                // No account selected yet — show picker if 2+ accounts available
                use steamlocate::SteamDir;
                let accounts = SteamDir::locate().ok()
                    .map(|sd| rewind_core::scanner::read_steam_accounts(sd.path()))
                    .unwrap_or_default();
                if accounts.len() >= 2 {
                    app.first_run_state = app::FirstRunState {
                        step: app::FirstRunStep::AccountPicker,
                        accounts,
                        selected_index: 0,
                    };
                    app.screen = Screen::FirstRun;
                    return;
                } else if let Some(only) = accounts.into_iter().next() {
                    // Auto-select the single account
                    app.config.preferred_steam_account = Some(only.id);
                    let _ = config::save_config(&app.config);
                }
            }
            app.is_first_run = false;
            app.screen = Screen::Main;
        }
        KeyCode::Left => {
            if app.settings_state.focused_field == 2
                && !app.settings_state.available_accounts.is_empty()
            {
                let n = app.settings_state.available_accounts.len() + 1;
                app.settings_state.account_index =
                    (app.settings_state.account_index + n - 1) % n;
            }
        }
        KeyCode::Right => {
            if app.settings_state.focused_field == 2
                && !app.settings_state.available_accounts.is_empty()
            {
                let n = app.settings_state.available_accounts.len() + 1;
                app.settings_state.account_index =
                    (app.settings_state.account_index + 1) % n;
            }
        }
        KeyCode::Backspace => match app.settings_state.focused_field {
            0 => { app.settings_state.username_input.pop(); }
            1 => { app.settings_state.library_input.pop(); }
            _ => {}
        },
        KeyCode::Char(c) => match app.settings_state.focused_field {
            0 => app.settings_state.username_input.push(c),
            1 => app.settings_state.library_input.push(c),
            _ => {}
        },
        KeyCode::Tab => {
            app.settings_state.focused_field = (app.settings_state.focused_field + 1) % 3;
        }
        KeyCode::Enter => match app.settings_state.focused_field {
            0 => {
                app.config.steam_username =
                    Some(app.settings_state.username_input.clone()).filter(|s| !s.is_empty());
                let _ = config::save_config(&app.config);
            }
            1 => {
                let path = std::path::PathBuf::from(app.settings_state.library_input.trim());
                if path.exists() && !app.config.libraries.iter().any(|l| l.path == path) {
                    app.config.libraries.push(rewind_core::config::Library { path });
                    let _ = config::save_config(&app.config);
                    app.settings_state.library_input.clear();
                    app.installed_games.clear();
                    for lib in &app.config.libraries.clone() {
                        let steamapps = lib.path.join("steamapps");
                        if steamapps.exists() {
                            if let Ok(games) = scanner::scan_library(&steamapps) {
                                app.installed_games.extend(games);
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}
```

- [ ] **Step 4: Update the handle_main 's' branch to call open_settings()**

In `handle_main`, replace the `KeyCode::Char('s')` branch:

```rust
KeyCode::Char('s') => {
    open_settings(app);
}
```

- [ ] **Step 5: Build and confirm clean compile**

```
cargo build -p rewind-cli 2>&1 | grep "^error" | head -20
```
Expected: no errors.

- [ ] **Step 6: Commit** (together with Tasks 6 and 7)

```bash
git add rewind-cli/src/main.rs rewind-cli/src/ui/settings.rs rewind-cli/src/ui/main_screen.rs
git commit -m "feat: add account field to Settings and first-run routing for account picker"
```

---

## Task 9: FirstRun UI — AccountPicker step

**Files:**
- Modify: `rewind-cli/src/ui/first_run.rs`

- [ ] **Step 1: Replace first_run.rs with version handling both steps**

```rust
use crate::app::{App, FirstRunStep};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.first_run_state.step {
        FirstRunStep::Welcome => draw_welcome(f),
        FirstRunStep::AccountPicker => draw_account_picker(f, app),
    }
}

fn draw_welcome(f: &mut Frame) {
    let area = f.area();
    let dialog_area = centered_rect(60, 14, area);
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Welcome to rewind ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

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

fn draw_account_picker(f: &mut Frame, app: &App) {
    let area = f.area();
    let accounts = &app.first_run_state.accounts;
    let height = (accounts.len() as u16 + 8).min(area.height);
    let dialog_area = centered_rect(60, height, area);
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Select Steam Account ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let inner_padded = inner.inner(Margin { horizontal: 1, vertical: 1 });

    // Split: account list on top, footer note on bottom
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(2),
        ])
        .split(inner_padded);

    // Account list
    let items: Vec<ListItem> = accounts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let label = format!("{} ({})", a.persona_name, a.account_name);
            let style = if i == app.first_run_state.selected_index {
                theme::list_selected()
            } else {
                theme::text()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, layout[0]);

    // Footer note
    let footer = Paragraph::new("[↑/↓] select   [Enter] confirm\nYou can change this later in Settings.")
        .alignment(Alignment::Center)
        .style(theme::text_secondary());
    f.render_widget(footer, layout[1]);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}
```

- [ ] **Step 2: Check that theme::list_selected() exists**

```
cargo grep "fn list_selected" rewind-cli/src/ui/theme.rs 2>/dev/null || grep -n "list_selected\|selected" rewind-cli/src/ui/theme.rs | head -10
```

If `list_selected` does not exist, check what the existing version picker uses for the highlighted row and use the same style. Add to `theme.rs` if needed:

```rust
pub fn list_selected() -> ratatui::style::Style {
    ratatui::style::Style::default()
        .fg(ratatui::style::Color::Black)
        .bg(ratatui::style::Color::White)
}
```

- [ ] **Step 3: Build to confirm it compiles**

```
cargo build -p rewind-cli 2>&1 | grep "^error" | grep "first_run" | head -5
```
Expected: no errors from first_run.rs.

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/ui/first_run.rs rewind-cli/src/ui/theme.rs
git commit -m "feat: add AccountPicker step to FirstRun screen"
```

---

## Task 10: Final build and smoke test

- [ ] **Step 1: Full build**

```
cargo build 2>&1 | grep "^error" | head -20
```
Expected: no errors.

- [ ] **Step 2: Run all tests**

```
cargo test 2>&1 | tail -15
```
Expected: all tests pass (the 2 known macOS immutability failures are Linux-only safe to ignore).

- [ ] **Step 3: Commit if any fixes were needed**

```bash
git add -p
git commit -m "fix: resolve any remaining compile issues from multi-account feature"
```

---

## Self-Review Against Spec

| Spec requirement | Covered by |
|---|---|
| `read_steam_accounts()` parses loginusers.vdf | Task 1 |
| `userdata_dir_for_account()` SteamID64 → 32-bit dir | Task 2 |
| `read_launch_options` uses preferred account, falls back to heuristic | Task 3 |
| `Config.preferred_steam_account: Option<u64>` | Task 4 |
| First-run: skip picker if 0 or 1 accounts | Task 8 (handle_settings Esc) |
| First-run: auto-select single account silently | Task 8 (handle_settings Esc) |
| First-run: show picker if 2+ accounts | Task 8 + Task 9 |
| First-run picker: PersonaName (AccountName) display | Task 9 |
| First-run picker: ↑/↓ + Enter | Task 8 (handle_first_run) |
| First-run footer: "You can change this later in Settings" | Task 9 |
| Settings: Steam Account field with ←/→ cycling | Tasks 7 + 8 |
| Settings: shows "Auto (most recent)" when None | Task 7 |
| Settings: non-interactive when no accounts | Task 7 (no ◀▶ arrows, handler guards) |
| Settings: saves SteamID64 or None on Esc | Task 8 |
| Launch options uses preferred_steam_account | Task 6 |
| Silent fallback when preferred account dir missing | Task 3 |
