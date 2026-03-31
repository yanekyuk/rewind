# rewind

> Roll back. Play the old way.

`rewind` is a cross-platform Rust CLI tool for managing Steam game version downgrades. It gives you a full-screen interactive TUI to browse your installed games, downgrade to any previous version via [DepotDownloader](https://github.com/SteamRE/DepotDownloader), and switch between cached versions instantly — no re-downloading required.

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

---

## Features

- **Instant version switching** — cached versions switch via symlink repoint, no download needed
- **Delta caching** — only stores files that differ between manifests, not entire game copies
- **ACF patching & locking** — patches `appmanifest_*.acf` and locks it to prevent Steam auto-updates
- **Auto DepotDownloader setup** — downloads and manages the DepotDownloader binary for you
- **Persistent config** — Steam credentials and library paths stored once, never asked again
- **Cross-platform** — Linux, macOS, and Windows

---

## How It Works

On the first downgrade, `rewind`:
1. Invokes DepotDownloader to fetch the target manifest's changed files into its cache
2. Backs up the current versions of those files
3. Replaces game directory files with symlinks pointing into the cache

Subsequent version switches just repoint the symlinks — instant, no network required. Restoring to latest removes the symlinks and lets Steam take over again.

---

## Requirements

- [.NET Runtime](https://dotnet.microsoft.com/download) (required by DepotDownloader)
- **Windows**: must run as Administrator (symlink creation requires elevation)
- **Linux**: `CAP_LINUX_IMMUTABLE` or root recommended for ACF immutability (`chattr +i`); falls back to read-only permissions otherwise
- **macOS**: no special permissions required

---

## Installation

```sh
cargo install rewind
```

Or build from source:

```sh
git clone https://github.com/yanekyuk/rewind
cd rewind
cargo build --release
```

---

## Data Directory

All state is stored locally — no cloud, no telemetry.

| Platform      | Path                          |
|---------------|-------------------------------|
| Linux / macOS | `~/.local/share/rewind/`      |
| Windows       | `%APPDATA%\rewind\`           |

```
~/.local/share/rewind/
  config.toml          ← Steam username, library paths
  games.toml           ← per-game version registry
  bin/
    DepotDownloader    ← auto-downloaded on first run
  cache/
    <app_id>/
      <depot_id>/
        <manifest_id>/
          <delta files>
```

---

## Keybindings

| Key | Action             |
|-----|--------------------|
| `D` | Downgrade / pick cached version |
| `U` | Upgrade to latest  |
| `L` | Toggle ACF lock    |
| `O` | Open SteamDB page  |
| `A` | Add Steam library  |
| `S` | Settings           |
| `?` | Help               |
| `Q` | Quit               |

---

## Tech Stack

Built with Rust. Core logic lives in `rewind-core`, the TUI in `rewind-cli`.

`ratatui` · `crossterm` · `tokio` · `serde` · `reqwest` · `keyvalues-parser`

---

## Out of Scope (v1)

- Cross-manifest file deduplication
- Multi-depot game support
- Automatic manifest ID lookup (SteamDB forbids scraping)
- GUI frontend

---

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
