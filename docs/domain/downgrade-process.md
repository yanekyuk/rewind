---
title: "Downgrade Process"
type: domain
tags: [steam, downgrade, workflow, manifests, depotdownloader]
created: 2026-03-30
updated: 2026-03-30
---

# Downgrade Process

This document describes the 9-step manual workflow for downgrading a Steam game to a previous version. Rewind automates steps 1-8 and provides guidance for step 9.

## Why Downgrading Is Needed

Steam auto-updates games by default. When a game update introduces bugs, performance regressions, or removes content, players need a way to revert to a known-good version. Steam provides no built-in rollback mechanism.

## The 9-Step Workflow

```
  1. Detect Steam    +--> 2. List Games +--> 3. User Picks Game
                                                     |
  +--------------------------------------------------+
  |
  v
  4. User Provides     5. Diff Manifests    6. Download
     Manifest ID  +-->  (current vs target) +--> Changed Files
                                                     |
  +--------------------------------------------------+
  |
  v
  7. Apply Files  +--> 8. Patch ACF &  +--> 9. Remind User:
     (Steam closed)     Lock Manifest       Set Update Pref
```

### Step 1: Detect Steam Installation

Locate the `steamapps/` directory on the user's system. See [steam-internals](./steam-internals.md) for platform-specific paths. The app must also check `libraryfolders.vdf` for additional library locations.

### Step 2: Identify Installed Games

Parse all `appmanifest_<appid>.acf` files in `steamapps/`. Each file corresponds to one installed game and contains its name, app ID, current build ID, and installed depot/manifest information.

### Step 3: User Selects a Game

Present the list of installed games. The user picks which game to downgrade.

### Step 4: User Provides Target Manifest ID

The user must supply the manifest ID for the version they want. Steam has no public API for listing historical manifests. The primary source is [SteamDB](https://steamdb.info/) -- users navigate to the game's depot page and find the manifest ID corresponding to their desired version.

See [decisions/manual-manifest-input](../decisions/manual-manifest-input.md) for why this is manual in the MVP.

### Step 5: Diff Manifests (Current vs Target)

To avoid downloading the entire game:

1. Fetch manifest metadata for both the current and target versions using DepotDownloader's `-manifest-only` flag
2. Parse both manifest files (lists of filenames, SHA hashes, sizes)
3. Compare files by SHA hash -- only files with different hashes need downloading
4. Generate a filelist of changed files

This optimization is significant. For example, downgrading Crimson Desert from 1.01.01 to 1.00.03 required downloading 153 of 257 files (~80 GB instead of ~133 GB).

### Step 6: Download Changed Files

Use DepotDownloader with the `-filelist` flag to download only the changed files from Steam's CDN. This step requires Steam credentials and can take a long time for large games (tens of GB).

See [depotdownloader](./depotdownloader.md) for the tool's capabilities and CLI interface.

### Step 7: Apply the Downgrade

With Steam **fully closed**, copy all downloaded files over the game's installation directory (`steamapps/common/<installdir>/`), overwriting existing files.

Steam must be closed because it monitors and potentially locks files in active game directories.

### Step 8: Patch the ACF Manifest

Edit `appmanifest_<appid>.acf` to prevent Steam from detecting a version mismatch. The key insight: **set the build ID and manifest ID to the latest values, not the target version's values**. This tricks Steam into thinking the game is already up to date.

| Field | Value | Reason |
|-------|-------|--------|
| `buildid` | Latest (not target) | Steam checks this against server |
| `manifest` (in InstalledDepots) | Latest (not target) | Must match buildid's expected manifest |
| `size` (in InstalledDepots) | Latest size value | Consistency |
| `StateFlags` | `4` | Fully installed |
| `TargetBuildID` | `0` | No pending update |
| `FullValidateAfterNextUpdate` | `0` | Prevent validation |
| `BytesToDownload` | `0` | No pending download |

After patching, lock the ACF file to prevent Steam from overwriting it:

- **Linux:** `sudo chattr +i <path>` (immutable flag; `chmod 444` is insufficient -- Steam can bypass it)
- **macOS:** `sudo chflags uchg <path>` (user immutable flag)
- **Windows:** Set read-only attribute via `SetFileAttributes`

See [platform-differences](./platform-differences.md) for details on privilege escalation.

### Step 9: Remind User to Set Update Preference

This step cannot be automated. The user must open Steam, go to the game's Properties > Updates, and set the update preference to "Only update this game when I launch it."

The app displays a reminder after the downgrade completes.

## Known Limitations

- **Denuvo games** require periodic online authentication; permanent offline mode is not viable for these titles.
- **GeForce Now / cloud gaming** users cannot use this tool (no local file access).
- **"Verify integrity of game files"** in Steam will undo the downgrade by re-downloading the latest version.
- **Manifest availability** depends on Steam's servers retaining old manifests. Developers can request removal of old manifests.
- **Steam internal database** may detect mismatches independently of the ACF file -- this needs further investigation.
