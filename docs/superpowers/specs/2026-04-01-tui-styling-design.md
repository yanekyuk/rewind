# TUI Styling Design: Steam Theme & Inline Game Images

**Date:** 2026-04-01
**Status:** Draft

## Overview

Redesign the rewind TUI to match Steam's visual style and display inline game artwork (background hero + logo) in the detail panel, with graceful fallback for terminals that don't support image protocols.

## Steam Color Palette

Centralized in a new `theme.rs` module, replacing all hardcoded inline colors.

| Role | Color | Hex | Usage |
|------|-------|-----|-------|
| Base background | Dark navy | `#1b2838` | App background |
| Panel background | Slate blue | `#2a475e` | Detail panel, modals |
| Accent | Steam blue | `#66c0f4` | Selection, focused borders, titles |
| Text primary | Light gray | `#c7d5e0` | Body text |
| Text secondary | Mid gray | `#8f98a0` | Help text, borders, secondary info |
| Success | Green | `#5ba32b` | Active version, completed steps |
| Warning | Gold | `#e5a00d` | In-progress, focused inputs |
| Error | Red | `#c33c3c` | Failed steps, error messages |
| Selected bg | Dark accent | `#3d6c8e` | Highlighted list items |

## Image System

### Steam CDN URLs

Two images per game, keyed by App ID:

- **Background:** `https://cdn.akamai.steamstatic.com/steam/apps/{appid}/library_hero.jpg` (hero art)
- **Logo:** `https://cdn.akamai.steamstatic.com/steam/apps/{appid}/logo.png` (transparent game logo)

### Rendering

Uses the `ratatui-image` crate with auto-detected protocol support:

- **Kitty graphics protocol** (Ghostty, Kitty, WezTerm)
- **Sixel** (many terminal emulators)
- **iTerm2 inline images** (iTerm2)
- **No support detected** → images are not shown; detail panel displays text-only layout (current behavior)

### Detail Panel Layout (with images)

```
┌─ Detail Panel ──────────────────────────────┐
│ ┌─────────────────────────────────────────┐  │
│ │         library_hero background         │  │
│ │            with logo overlay            │  │
│ │              (top ~40%)                 │  │
│ └─────────────────────────────────────────┘  │
│                                              │
│  Game Name              App ID: 12345        │
│  Status: Downgraded (locked)                 │
│  Depot ID: 67890                             │
│  Active Manifest: abc123                     │
│  ...                                         │
│                                              │
│  [d] Downgrade  [r] Restore  [s] SteamDB     │
└──────────────────────────────────────────────┘
```

If images aren't supported or haven't loaded yet, the top image area is absent and text info fills the panel as it does today.

### Disk Cache

Images are cached on disk in the platform-appropriate data directory:

- **Linux:** `~/.local/share/rewind/images/`
- **macOS:** `~/Library/Application Support/rewind/images/`
- **Windows:** `%APPDATA%/rewind/images/`

Files are named `{appid}_hero.jpg` and `{appid}_logo.png`.

**Loading flow on game selection:**

1. Check disk cache for the selected game's images
2. If cached, load from disk
3. If not cached, fetch via HTTP in a `tokio::spawn` task, save to disk, then display
4. The panel renders immediately with text; images appear asynchronously once loaded (no blocking)

### Error Handling

Image fetch failures are silent. A missing or failed image results in the text-only layout being shown. No user-facing errors for image issues.

## Dependencies Added

| Crate | Purpose | Added to |
|-------|---------|----------|
| `ratatui-image` | Image widget + protocol detection | `rewind-cli` |
| `image` | Image decoding (required by ratatui-image) | `rewind-cli` |
| `reqwest` | HTTP client for Steam CDN fetches | `rewind-core` |
| `dirs` | Platform-appropriate cache directory | `rewind-core` |

## Files Changed

### New files

- **`rewind-cli/src/ui/theme.rs`** — Centralized color/style constants
- **`rewind-core/src/image_cache.rs`** — Disk cache logic (fetch, store, load)

### Modified files

- **`rewind-cli/src/ui/main_screen.rs`** — Detail panel redesigned with image area, new palette
- **`rewind-cli/src/ui/first_run.rs`** — Apply Steam palette
- **`rewind-cli/src/ui/downgrade_wizard.rs`** — Apply Steam palette
- **`rewind-cli/src/ui/version_picker.rs`** — Apply Steam palette
- **`rewind-cli/src/ui/settings.rs`** — Apply Steam palette
- **`rewind-cli/src/ui/mod.rs`** — Apply Steam palette to any shared rendering
- **`rewind-cli/src/app.rs`** — Add image state (loaded images, protocol picker) to App struct
- **`rewind-cli/Cargo.toml`** — Add `ratatui-image`, `image`
- **`rewind-core/Cargo.toml`** — Add `reqwest`, `dirs`

## Out of Scope

- No changes to core business logic (ACF patching, depot downloader, immutability)
- No changes to keybindings or navigation
- No changes to game list panel layout (left side) beyond color updates
