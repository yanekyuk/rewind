# rewind

> Roll back. Play the old way.

`rewind` is a cross-platform tool for managing Steam game version downgrades. It gives you a full-screen interactive TUI to browse your installed games, downgrade to any previous version, and switch between cached versions instantly — no re-downloading required.

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

# For Gamers

## Step 1 — Install .NET Runtime

`rewind` uses [DepotDownloader](https://github.com/SteamRE/DepotDownloader) under the hood, which requires the .NET runtime.

**Windows**

Download and run the installer from the official page:
https://dotnet.microsoft.com/download/dotnet

Or install via winget:
```
winget install Microsoft.DotNet.Runtime.9
```

**macOS**

```sh
brew install dotnet
```

Or download the installer from https://dotnet.microsoft.com/download/dotnet

**Linux**

Ubuntu / Debian:
```sh
sudo apt-get update && sudo apt-get install -y dotnet-runtime-9.0
```

Fedora:
```sh
sudo dnf install dotnet-runtime-9.0
```

Arch Linux:
```sh
sudo pacman -S dotnet-runtime
```

Verify the installation:
```sh
dotnet --version
```

## Step 2 — Download rewind

Go to the [Releases](https://github.com/yanekyuk/rewind/releases) page and download the binary for your platform:

| Platform | File |
|----------|------|
| Windows (64-bit) | `rewind-x86_64-windows.zip` |
| macOS (Apple Silicon) | `rewind-aarch64-macos.tar.gz` |
| macOS (Intel) | `rewind-x86_64-macos.tar.gz` |
| Linux (64-bit) | `rewind-x86_64-linux.tar.gz` |

Extract the archive and place the `rewind` binary somewhere on your PATH.

## Step 3 — Platform Notes

**Windows** — Run `rewind` as Administrator. Symlink creation requires elevated privileges.

**Linux** — ACF locking (`chattr +i`) works best with root or `CAP_LINUX_IMMUTABLE`. Without it, `rewind` falls back to read-only file permissions, which may not fully prevent Steam from overwriting files.

**macOS** — No special permissions required.

## Keybindings

| Key | Action                          |
|-----|---------------------------------|
| `D` | Downgrade / pick cached version |
| `U` | Upgrade to latest               |
| `L` | Toggle ACF lock                 |
| `O` | Open SteamDB page               |
| `A` | Add Steam library               |
| `S` | Settings                        |
| `?` | Help                            |
| `Q` | Quit                            |

## Data Directory

All state is stored locally — no cloud, no telemetry.

| Platform      | Path                     |
|---------------|--------------------------|
| Linux / macOS | `~/.local/share/rewind/` |
| Windows       | `%APPDATA%\rewind\`      |

---

# For Developers

## Building from Source

Requires [Rust](https://rustup.rs) (stable).

```sh
git clone https://github.com/yanekyuk/rewind
cd rewind
cargo build --release
```

The binary will be at `target/release/rewind`.

## Install via Cargo

```sh
cargo install rewind
```

## Workspace Structure

```
rewind/
  Cargo.toml          ← workspace root
  rewind-core/        ← business logic library
  rewind-cli/         ← ratatui TUI binary
  docs/
```

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | Cross-platform full-screen TUI |
| `tokio` | Async runtime |
| `serde` + `toml` | Config serialization |
| `reqwest` | HTTP for downloading DepotDownloader |
| `keyvalues-parser` | Steam `.acf` / `.vdf` file parsing |
| `thiserror` | Structured error types |

## Out of Scope (v1)

- Cross-manifest file deduplication
- Multi-depot game support
- Automatic manifest ID lookup (SteamDB forbids scraping)
- GUI frontend

---

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
