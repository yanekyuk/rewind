# rewind

> Roll back. Play the old way.

`rewind` is a cross-platform tool for managing Steam game version downgrades. It gives you a full-screen interactive TUI to browse your installed games, downgrade to any previous version, and switch between cached versions instantly — no re-downloading required.

```
┌─────────────────────────────────────────────────────┐
│  rewind                                    [?] help │
├─────────────────┬───────────────────────────────────┤
│ GAMES           │  Crimson Desert                   │
│                 │  App ID: 3321460                  │
│ > Crimson Desert│  Status:    Updates disabled       │
│   Elden Ring    │  Installed: manifest abc123        │
│   Dark Souls III│  Spoofed as: manifest def456       │
│                 │  Cached:    1.00, 1.01             │
│                 │  Launch:    -high -novid           │
│                 │                                    │
│                 │  [D] Download   [U] Switch version │
│                 │  [O] Open SteamDB                  │
├─────────────────┴───────────────────────────────────┤
│ [A] Add library  [S] Settings  [Q] Quit             │
└─────────────────────────────────────────────────────┘
```

---

## Features

- **Instant version switching** — cached versions switch via symlink repoint, no download needed
- **Delta caching** — only stores files that differ between manifests, not entire game copies
- **ACF patching & locking** — patches `appmanifest_*.acf` and locks it to prevent Steam auto-updates
- **Launch options display** — shows configured Steam launch options in the game detail panel
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
| Windows (64-bit) | [`rewind-x86_64-windows.zip`](https://github.com/yanekyuk/rewind/releases/download/v0.2.0/rewind-x86_64-windows.zip) |
| macOS (Apple Silicon) | [`rewind-aarch64-macos.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v0.2.0/rewind-aarch64-macos.tar.gz) |
| macOS (Intel) | [`rewind-x86_64-macos.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v0.2.0/rewind-x86_64-macos.tar.gz) |
| Linux (64-bit) | [`rewind-x86_64-linux.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v0.2.0/rewind-x86_64-linux.tar.gz) |

`rewind` is a terminal application — it always needs to be launched from a terminal window. It cannot be opened by double-clicking.

**Windows**

1. Extract the `.zip` — you'll get `rewind.exe`
2. Move `rewind.exe` to a folder of your choice, e.g. `C:\Users\you\rewind\`
3. Open that folder in File Explorer, then right-click inside it and choose **Open in Terminal** (or **Open PowerShell window here**)
4. Type `.\rewind.exe` and press Enter

To run `rewind` from any terminal without navigating to its folder first:
1. Open **Start**, search for **"Edit the system environment variables"**
2. Click **Environment Variables** → under **User variables**, select **Path** → **Edit**
3. Click **New** and paste the full path to the folder containing `rewind.exe`
4. Click OK, restart your terminal

**macOS**

1. Extract the `.tar.gz` — you'll get a `rewind` binary
2. Open **Terminal** (find it in Spotlight with `Cmd+Space`, type "Terminal")
3. Drag the `rewind` file into the Terminal window — this pastes its full path
4. Press Enter to run it

On first launch, macOS may block it as an unrecognized app. If that happens:
- Open **System Settings → Privacy & Security**, scroll down, and click **Open Anyway**

To run `rewind` from any terminal by just typing `rewind`:
```sh
sudo mv rewind /usr/local/bin/rewind
chmod +x /usr/local/bin/rewind
```

**Linux**

1. Extract the `.tar.gz` — you'll get a `rewind` binary
2. Open a terminal in the folder where you extracted it
3. Make it executable and run it:
```sh
chmod +x ./rewind
./rewind
```

To run `rewind` from any terminal by just typing `rewind`:
```sh
sudo mv rewind /usr/local/bin/rewind
```

## Step 3 — Platform Notes

**Windows** — Run `rewind` as Administrator. Symlink creation requires elevated privileges.

**Linux** — Most features work without any special permissions. The one exception is **ACF locking**: `rewind` locks the game's manifest file to stop Steam from auto-updating over your downgraded version. This uses `chattr +i` under the hood, which requires a specific Linux privilege.

You do **not** need to run `sudo rewind`. Instead, grant only the required privilege to the binary once:

```sh
sudo setcap cap_linux_immutable+ep /usr/local/bin/rewind
```

After that, `rewind` can lock and unlock ACF files without any further sudo usage.

If you skip this step, `rewind` will still work but falls back to read-only file permissions for locking. This is weaker — Steam runs as your own user and can override read-only flags, meaning it may auto-update and overwrite your downgraded version.

**macOS** — No special permissions required.

## Keybindings

| Key | Action                          |
|-----|---------------------------------|
| `D` | Download new version            |
| `U` | Switch between cached versions  |
| `O` | Open SteamDB page               |
| `S` | Settings                        |
| `Q` | Quit                            |

### Download Wizard

| Key     | Action                          |
|---------|---------------------------------|
| `P`     | Open SteamDB patches page       |
| `M`     | Open SteamDB manifests page     |
| `Enter` | Download pasted manifest ID     |
| `Esc`   | Cancel                          |

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
