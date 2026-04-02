# Launch Options Display — Design Spec

**Date:** 2026-04-02
**Status:** Approved
**Issue:** [yanekyuk/rewind#28](https://github.com/yanekyuk/rewind/issues/28) (multi-account follow-up)

---

## Overview

Display the Steam launch options for a selected game in Rewind's detail panel (read-only). Launch options are stored per-user in `localconfig.vdf`, not in ACF files.

---

## Background: `localconfig.vdf` Structure

Steam stores per-user game configuration in:

```
<steam_root>/userdata/<steamid>/config/localconfig.vdf
```

Platform-specific Steam roots (resolved via the `steamlocate` crate):
- **Linux**: `~/.steam/steam/` or `~/.local/share/Steam/`
- **Windows**: `C:\Program Files (x86)\Steam\`
- **macOS**: `~/Library/Application Support/Steam/`

The `userdata/` directory is always a direct child of the Steam root on all platforms.

### Launch options location within the file

```
"UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "<appid>"
                    {
                        "LaunchOptions"    "-novid %command%"
                    }
                }
            }
        }
    }
}
```

The `LaunchOptions` key is absent when the user has not set any launch options for that game.

---

## Multi-User Heuristic

When multiple Steam accounts exist (multiple subdirectories under `userdata/`), select the `localconfig.vdf` with the most recent file modification time. Full multi-account management is tracked in [yanekyuk/rewind#28](https://github.com/yanekyuk/rewind/issues/28).

---

## Architecture

### 1. Core parsing — `rewind-core/src/scanner.rs`

New public function:

```rust
pub fn read_launch_options(steam_root: &Path, app_id: u32) -> Option<String>
```

- Globs `steam_root/userdata/*/config/localconfig.vdf`
- Selects the file with the most recent modification time
- Parses the text VDF using the existing hand-written line-by-line approach (no new dependencies)
- Navigates depth: `UserLocalConfigStore > Software > Valve > Steam > apps > <appid>`
- Returns the `LaunchOptions` value, or `None` if absent or empty

### 2. App state — `rewind-cli/src/app.rs`

New field on `App`:

```rust
pub launch_options_cache: HashMap<u32, Option<String>>,
```

- Missing key = not yet attempted for that appid
- `Some(None)` = loaded, no options set
- `Some(Some(s))` = loaded, options string `s`

Populated lazily in `draw_detail_panel()` on cache miss: calls `steamlocate::SteamDir::locate()` and `read_launch_options()`. At most one file read per appid per session.

### 3. Display — `rewind-cli/src/ui/main_screen.rs`

In `draw_detail_panel()`:

- **Cache miss (first render):** show `"  Launch:    …"` as a placeholder
- **`Some(None)` or `Some("")`:** omit the launch options line entirely
- **`Some(Some(s))`:** show `"  Launch:    <s>"`, wrapped by the existing `Wrap { trim: false }` paragraph

Example display:
```
  NBA 2K26
  App ID:    3472040
  Depot:     3472041

  Status:    ✓ Updates enabled
  Installed: 8417590105049508430
  Cached:    none

  Launch:    DXVK_FRAME_RATE=60 VKD3D_CONFIG=no_upload_hvv,pipeline_library_no_serialize_spirv
             DXVK_ASYNC=1 SteamDeck=1 gamescope -W 3840 -H 2160 -f --force-grab-cursor
             --backend wayland -- mangohud gamemoderun %command%

  [D] Download new version
  [U] Switch version
  [O] Open app on SteamDB
```

---

## Out of Scope

- Editing launch options (future feature)
- Selecting a specific Steam account (tracked in yanekyuk/rewind#28)
- Caching launch options to `games.toml`
