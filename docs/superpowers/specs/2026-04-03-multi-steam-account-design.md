# Multi-Steam Account Support

**Issue:** #28  
**Date:** 2026-04-03  
**Status:** Approved

## Problem

Rewind assumes a single Steam user. On machines with multiple accounts (multiple directories under `~/.steam/steam/userdata/`), features that read per-user config (e.g. `localconfig.vdf` for launch options) pick arbitrarily. Users need a way to configure a preferred account.

## Approach

Store `preferred_steam_account: Option<u64>` (SteamID64) in `config.toml`. Resolve display names from `~/.steam/steam/config/loginusers.vdf`. Fall back to the existing most-recently-modified heuristic when no preference is set or the account's directory is missing.

---

## Data Layer (`rewind-core`)

### New functions in `scanner.rs`

**`read_steam_accounts(steam_root: &Path) -> Vec<SteamAccount>`**

- Parses `<steam_root>/config/loginusers.vdf`
- Returns a `Vec<SteamAccount>` sorted by `Timestamp` descending (most recently used first)
- Returns empty vec if the file is missing or unparseable

```rust
pub struct SteamAccount {
    pub id: u64,               // SteamID64
    pub persona_name: String,  // Display name (e.g. "yanekeke")
    pub account_name: String,  // Login name (e.g. "yanekyuk")
}
```

**`userdata_dir_for_account(steam_root: &Path, steam_id64: u64) -> Option<PathBuf>`**

- Converts SteamID64 to 32-bit account ID: `steam_id64 - 76561197960265728`
- Returns `Some(<steam_root>/userdata/<32-bit-id>/)` if the directory exists, else `None`

### Modified function in `scanner.rs`

**`read_launch_options(steam_root, app_id, preferred_account: Option<u64>) -> Option<String>`**

- If `preferred_account` is `Some(id)` and `userdata_dir_for_account` returns a path, use that account's `localconfig.vdf` directly.
- Otherwise fall back to the existing most-recently-modified heuristic across all `userdata/*/config/localconfig.vdf` files.

### Config change (`config.rs`)

```rust
pub struct Config {
    pub steam_username: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub preferred_steam_account: Option<u64>,  // new
}
```

---

## First-Run Flow (`rewind-cli`)

A new step is added to the `FirstRun` screen, **after** the existing username and library steps.

**Conditions:**
- `read_steam_accounts()` returns 0 accounts → skip step, no account saved.
- `read_steam_accounts()` returns exactly 1 account → skip step, auto-select that account silently (save its SteamID64).
- `read_steam_accounts()` returns 2+ accounts → show the picker.

**Picker UI:**
- List of accounts, each displayed as `PersonaName (AccountName)`.
- `↑`/`↓` to navigate, `Enter` to confirm.
- Footer: `"You can change this later in Settings."`
- On confirm, saves the SteamID64 to `config.toml` as `preferred_steam_account`.

---

## Settings Screen (`rewind-cli`)

A new focusable field **"Steam Account"** is added to the Settings screen.

**Display:**
- Shows `PersonaName (AccountName)` for the currently selected account.
- Shows `"Auto (most recent)"` if `preferred_steam_account` is `None`.

**Interaction:**
- `←`/`→` cycles through: `[Auto, account1, account2, ...]`
- If `loginusers.vdf` is missing/empty, field shows `"Auto (most recent)"` and is non-interactive.
- On `Esc` (save & exit), writes the selected SteamID64 to `config.toml`, or clears `preferred_steam_account` if "Auto" is selected.

---

## Fallback Behaviour

In all cases where a preferred account is configured but its `userdata/` directory doesn't exist (e.g. account removed, directory deleted), `read_launch_options` silently falls back to the most-recently-modified heuristic. No error or warning is shown.

---

## Affected Features

- Launch options display (reads `userdata/<id>/config/localconfig.vdf`)
- Any future per-user Steam data (playtime, cloud saves, etc.) can use `userdata_dir_for_account` as the single resolution point

---

## Out of Scope

- Resolving account names from the Steam Web API (online lookup)
- Per-game account overrides
- Multiple simultaneous account support
