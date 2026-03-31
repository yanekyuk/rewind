# rewind-cli Design Spec
Date: 2026-04-01

## Overview

`rewind` is a cross-platform Rust CLI tool that manages Steam game version downgrades. It provides a full-screen interactive TUI where users can see their installed Steam games, downgrade to any previous version using DepotDownloader, and switch between cached versions instantly via symlinks. It runs as a persistent manager — credentials, library paths, and game state are stored so users never need to repeat setup.

---

## Workspace Structure

Cargo workspace with two crates:

```
rewind-cli/
  Cargo.toml             ← workspace root
  rewind-core/           ← business logic library
    src/
      lib.rs
      config.rs
      scanner.rs
      cache.rs
      depot.rs
      patcher.rs
      immutability.rs
      steamdb.rs
  rewind-cli/            ← ratatui TUI binary
    src/
      main.rs
      app.rs
      ui/
        ...
  docs/
    superpowers/
      specs/
        2026-04-01-rewind-cli-design.md
```

---

## Data Model & Config

All state stored in a platform-specific data directory:
- Linux/macOS: `~/.local/share/rewind/`
- Windows: `%APPDATA%\rewind\`

```
~/.local/share/rewind/
  config.toml          ← Steam username, Steam library paths
  games.toml           ← per-game registry
  bin/
    DepotDownloader    ← auto-downloaded on first run
  cache/
    <app_id>/
      <depot_id>/
        <manifest_id>/
          <delta files for this version>
```

### config.toml

```toml
steam_username = "myusername"

[[libraries]]
path = "/home/user/.steam/steam/steamapps"

[[libraries]]
path = "/mnt/games/steamapps"
```

### games.toml

```toml
[[games]]
name = "Crimson Desert"
app_id = 3321460
depot_id = 3321461
install_path = "/home/user/.steam/steam/steamapps/common/Crimson Desert"
active_manifest_id = "abc123"
latest_manifest_id = "def456"
cached_manifest_ids = ["abc123", "def456"]
acf_locked = true
```

---

## rewind-core Modules

### `config.rs`
Read/write `config.toml` and `games.toml`. Provides typed structs for `Config` and `GameEntry`. Handles first-run defaults.

### `scanner.rs`
- Detects Steam library paths per OS by reading `libraryfolders.vdf`
- Parses all `appmanifest_*.acf` files in each library
- Returns list of installed games with app_id, name, install path, current manifest ID

### `cache.rs`
- Manages `cache/<app_id>/<depot_id>/<manifest_id>/` directories
- On first downgrade:
  1. DepotDownloader runs first, downloading target manifest files into `cache/.../target_manifest_id/`
  2. The downloaded files are exactly the delta — cache identifies changed files by inspecting what was downloaded
  3. For each changed file, copy the current version from game dir into `cache/.../current_manifest_id/`
  4. Replace game dir files with symlinks → `cache/.../target_manifest_id/`
- On version switch (cached): repoints existing symlinks to new manifest dir — no download
- On restore: removes symlinks, restores real files from cache, unlocks ACF

### `depot.rs`
- Checks for DepotDownloader in `~/.local/share/rewind/bin/`
- If missing: fetches latest release from `https://github.com/SteamRE/DepotDownloader/releases`, downloads zip, extracts, stores binary
- Verifies .NET runtime is available; warns user if not
- Invokes DepotDownloader with: `-app <id> -depot <id> -manifest <id> -username <user> -remember-password -dir <cache_dir>`
- Streams stdout/stderr back to TUI progress display

### `patcher.rs`
- Parses `appmanifest_<app_id>.acf` (Valve KeyValues format)
- Updates fields: `buildid`, `LastOwner` manifest ID, `StateFlags` (set to `4` = fully installed, no update needed)
- Writes patched ACF back to disk

### `immutability.rs`
- **Windows**: `SetFileAttributesW` with `FILE_ATTRIBUTE_READONLY`
- **Linux**: `ioctl` with `FS_IMMUTABLE_FL` flag (requires root / `chattr +i`)
- **macOS**: `chflags uchg`
- Unlock: reverse of the above

### `steamdb.rs`
- Constructs SteamDB URLs from known IDs:
  - Depot manifests: `https://www.steamdb.info/depot/<depot_id>/manifests/`
  - App page: `https://www.steamdb.info/app/<app_id>/`

---

## TUI Layout

Full-screen `ratatui` UI with two panels and a status bar:

```
┌─────────────────────────────────────────────────────┐
│  rewind                                    [?] help  │
├─────────────────┬───────────────────────────────────┤
│ GAMES           │  Crimson Desert                   │
│                 │  App ID: 3321460                  │
│ > Crimson Desert│  Status: ▼ Downgraded             │
│   Elden Ring    │  Active:  1.00 (manifest abc123)  │
│   Dark Souls III│  Latest:  1.01 (manifest def456)  │
│                 │  Cached:  1.00, 1.01              │
│                 │                                   │
│                 │  [D] Downgrade  [U] Upgrade        │
│                 │  [L] Lock ACF   [O] Open SteamDB  │
├─────────────────┴───────────────────────────────────┤
│ [A] Add library  [S] Settings  [Q] Quit             │
└─────────────────────────────────────────────────────┘
```

### Screens / Overlays

- **Main** — game list (left) + detail panel (right), keyboard navigation
- **Downgrade wizard** — shows constructed SteamDB depot URL for user to open in browser, text input for manifest ID, progress bar during DepotDownloader run
- **Version picker** — shown when cached versions exist; list of cached manifest IDs to switch to instantly
- **Add library** — text input for Steam library path, triggers rescan
- **Settings** — Steam username, library paths list with add/remove
- **First run** — auto-detects Steam libraries, confirms with user before proceeding

---

## Workflows

### First-time downgrade

1. User selects game, presses `[D]`
2. TUI shows SteamDB depot manifests URL — user opens in browser
3. User enters target manifest ID
4. `rewind` invokes DepotDownloader:
   - Downloads target manifest files into `cache/.../target_manifest_id/`
5. `rewind` identifies changed files (those downloaded by DepotDownloader)
6. Current versions of those files copied from game dir → `cache/.../current_manifest_id/`
7. Game dir files replaced with symlinks → `cache/.../target_manifest_id/`
8. ACF patched and locked
9. Status shows "Downgraded"

### Switch to cached version (instant)

1. User presses `[D]` or `[U]`, sees cached version list
2. Selects target manifest
3. Symlinks repointed — no download
4. ACF patched and re-locked

### Restore to latest

1. User selects "Restore latest"
2. ACF unlocked
3. Symlinks removed, real files restored from cache
4. `games.toml` updated — Steam can now update normally

### Adding a new Steam library

1. User presses `[A]`
2. Enters path manually or browses
3. `scanner` reads all ACF files from new library
4. New games appear in list

---

## Caching Strategy

- Cache stores only **delta files** (files that differ between two specific manifests)
- Cache is a flat pool: `cache/<app_id>/<depot_id>/<manifest_id>/`
- No cross-manifest deduplication (kept simple for now)
- Switching between two cached versions is instant (symlink repoint only)
- Switching to an uncached version requires a DepotDownloader run

---

## Tech Stack

### rewind-core
| Crate | Purpose |
|-------|---------|
| `serde` + `toml` | Config serialization |
| `keyvalues-parser` | Parse Steam `.acf` / `.vdf` files |
| `reqwest` + `tokio` | Async HTTP for downloading DepotDownloader |
| `zip` | Extract DepotDownloader release zip |
| `walkdir` | File system traversal |
| `sha2` | Verify DepotDownloader binary integrity |
| `thiserror` | Structured error types |

### rewind-cli
| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | Cross-platform full-screen TUI |
| `tokio` | Async runtime for background downloads |
| `open` | Open SteamDB URLs in system browser |

---

## Platform Notes

- **Windows**: Requires running as Administrator for symlink creation. `rewind` checks for admin at startup and exits with a clear error if not elevated.
- **Linux**: `chattr +i` (ACF immutability) requires root or `CAP_LINUX_IMMUTABLE`. `rewind` warns if unavailable and falls back to read-only file permissions.
- **macOS**: `chflags uchg` works without root for user-owned files.

---

## DepotDownloader

- Downloaded from GitHub releases (`SteamRE/DepotDownloader`) on first run
- Stored in rewind data dir (`bin/DepotDownloader`)
- Requires .NET runtime — `rewind` checks via `dotnet --version` and shows install instructions if missing
- Uses `-remember-password` so Steam credentials are only entered once
- Steam username stored in `config.toml`; password handled entirely by DepotDownloader's session cache

---

## Out of Scope (v1)

- Cross-manifest file deduplication in cache
- Multi-depot games (games split across many depots — handled as single primary depot for now)
- Automatic manifest ID lookup (SteamDB forbids scraping; Steam Web API only returns latest)
- GUI frontend
