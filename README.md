<p align="center">
  <img src="public/tauri.svg" width="80" alt="Rewind" />
</p>

<h1 align="center">Rewind</h1>

<p align="center">
  <strong>Roll back any Steam game to a previous version.</strong>
</p>

<p align="center">
  <a href="#how-it-works">How It Works</a> &middot;
  <a href="#installation">Installation</a> &middot;
  <a href="#usage">Usage</a> &middot;
  <a href="#building-from-source">Building from Source</a> &middot;
  <a href="#license">License</a>
</p>

---

Rewind is a cross-platform desktop app that automates downgrading Steam games to previous versions. It handles manifest diffing, selective file downloading, ACF patching, and manifest locking — so you don't have to do it manually.

> [!WARNING]
> Rewind is in early development and not yet usable. Watch this repo for updates.

## How It Works

Steam doesn't offer a built-in way to roll back game updates. Rewind automates the manual process:

1. **Detects your Steam installation** and lists installed games
2. **You pick a game** and provide the target manifest ID from [SteamDB](https://steamdb.info/)
3. **Diffs the manifests** — compares file hashes between your current version and the target to figure out what actually changed
4. **Downloads only the changed files** via [DepotDownloader](https://github.com/SteamRE/DepotDownloader), saving bandwidth and time
5. **Applies the downgrade** — overwrites the changed files in your game directory (with Steam closed)
6. **Patches the ACF manifest** — tricks Steam into thinking the game is already up to date
7. **Locks the manifest file** — prevents Steam from reverting the patch on next launch

## Installation

Pre-built binaries will be available on the [Releases](https://github.com/yanekyuk/rewind/releases) page once the first version is ready.

Rewind bundles DepotDownloader — no separate .NET installation required.

### Supported Platforms

| Platform | Status |
|----------|--------|
| Linux    | Planned |
| Windows  | Planned |
| macOS    | Planned |

## Usage

1. Open Rewind
2. Select the game you want to downgrade
3. Find the target version's manifest ID on [SteamDB](https://steamdb.info/) (navigate to the game's depot page)
4. Paste the manifest ID into Rewind
5. Enter your Steam credentials (used only for DepotDownloader authentication)
6. Wait for the download to complete
7. Close Steam when prompted, and Rewind will apply the downgrade
8. Set the game's update preference to **"Only update this game when I launch it"** in Steam

### Restoring to the Latest Version

To undo a downgrade:

1. Use Rewind to unlock the manifest file
2. In Steam, right-click the game → Properties → Installed Files → **Verify integrity of game files**
3. Steam will re-download the latest version

## Building from Source

### Prerequisites

- [Bun](https://bun.sh/) (frontend runtime)
- [Rust](https://rustup.rs/) (backend)
- Platform-specific Tauri dependencies — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

### Steps

```bash
# Clone the repo
git clone https://github.com/yanekyuk/rewind.git
cd rewind

# Install frontend dependencies
bun install

# Run in development mode
bun run tauri dev

# Build for production
bun run tauri build
```

## Known Limitations

- **No automatic version discovery** — you need to find the manifest ID on SteamDB yourself
- **Denuvo games** require periodic online authentication, so permanent offline play isn't viable
- **Cloud gaming** (GeForce Now, etc.) is not supported — local file access is required
- **"Verify integrity"** in Steam will undo the downgrade
- **Windows manifest locking** uses the read-only attribute, which is less robust than Linux/macOS immutable flags

## License

[GPL-2.0](LICENSE) — required for bundling [DepotDownloader](https://github.com/SteamRE/DepotDownloader).
