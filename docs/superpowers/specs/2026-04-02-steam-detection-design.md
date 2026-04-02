# Steam Detection Design

**Date:** 2026-04-02  
**Status:** Approved

## Overview

Detect whether Steam is running before the user performs version downgrade or switch operations. Show a non-blocking warning when entering relevant screens, and hard-block the operation if Steam is still running when the user tries to proceed.

## Architecture

### Core detection (`rewind-core`)

New module: `rewind-core/src/steam_guard.rs`

```rust
pub fn is_steam_running() -> bool
```

Uses the `sysinfo` crate to enumerate system processes and checks for any process with a name matching `steam` (case-insensitive). This covers:
- `steam` on Linux
- `Steam` on macOS
- `steam.exe` on Windows (sysinfo strips `.exe`)

Add `sysinfo` to `rewind-core/Cargo.toml` with the `processes` feature only.

Export `steam_guard` from `rewind-core/src/lib.rs`.

### Warning on screen entry (`rewind-cli`)

In `handle_main` (`main.rs`), when transitioning to:

- `Screen::DowngradeWizard` (key `d`): call `is_steam_running()`, set `wizard_state.steam_warning = true` if detected
- `Screen::VersionPicker` (key `u`): call `is_steam_running()`, set `version_picker_state.steam_warning = true` if detected

**State changes:**
- `DowngradeWizardState`: add `steam_warning: bool` field
- `VersionPickerState`: add `steam_warning: bool` field

Each screen renders a yellow/warning-styled inline line when `steam_warning` is true. This does not block navigation.

### Hard block on operation (`rewind-cli`)

Re-check `is_steam_running()` at the moment the user triggers an operation:

1. **`start_download`** — at the top of the function, if Steam is running set `app.wizard_state.error = Some("Steam is running. Quit Steam before downloading.")` and return early.
2. **`handle_version_picker`** on Enter — if Steam is running, set `version_picker_state.error = Some("Steam is running. Quit Steam before switching versions.")` and return early.

**State changes:**
- `VersionPickerState`: add `error: Option<String>` field (mirrors the existing pattern in `DowngradeWizardState`)

The check is intentionally re-run at operation time because the user may open a screen before Steam launches.

## Error Handling

`sysinfo::System::new_with_specifics` can fail silently (returns empty process list). In that case `is_steam_running()` returns `false` — fail open, don't block the user if detection is unavailable.

## Testing

- Unit test in `rewind-core`: mock process list is not practical with sysinfo; test by verifying the function compiles and returns `bool` on all platforms (CI covers Linux/macOS/Windows).
- Manual test: run with Steam open, verify warning appears on screen entry and hard block fires on operation.
