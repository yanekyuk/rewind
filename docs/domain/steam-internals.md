---
title: "Steam Internals"
type: domain
tags: [steam, depots, manifests, acf, vdf, buildid]
created: 2026-03-30
updated: 2026-03-30
---

# Steam Internals

This document covers the Steam concepts that Rewind interacts with. Contributors unfamiliar with Steam's internal data model should read this first.

## Core Concepts

### App, Depot, and Manifest

Steam organizes game content in a three-level hierarchy:

- **App** -- a game or application, identified by an `appid` (e.g., `3321460` for Crimson Desert).
- **Depot** -- a subset of an app's content, identified by a `depotid`. Most games have one primary depot; some split content across multiple depots (e.g., platform-specific assets, DLC).
- **Manifest** -- a snapshot of a depot's contents at a specific point in time, identified by a `manifestid` (a large numeric ID like `7446650175280810671`). Each time a developer pushes an update, a new manifest is created.

A manifest lists every file in the depot with its SHA hash, size, and chunk count. Comparing two manifests reveals exactly which files changed between versions.

### Build ID

A `buildid` is a monotonically increasing integer that Steam assigns to each build pushed by a developer. It is shared across all depots of an app -- unlike manifest IDs, which are per-depot.

Steam uses the build ID to determine whether a game needs updating. If the locally recorded build ID is lower than the latest known build ID, Steam marks the game as requiring an update.

## Local Data Structures

### ACF Files (App Cache Format)

Each installed game has a corresponding `appmanifest_<appid>.acf` file in the `steamapps/` directory. These are text files in Valve Data Format (VDF) that record the local installation state:

```
"AppState"
{
    "appid"        "3321460"
    "name"         "Crimson Desert"
    "buildid"      "22560074"
    "installdir"   "Crimson Desert"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}
```

Key fields:

| Field | Purpose |
|-------|---------|
| `appid` | Steam application identifier |
| `name` | Human-readable game name |
| `buildid` | Currently installed build number |
| `installdir` | Game folder name under `steamapps/common/` |
| `StateFlags` | Installation state (`4` = fully installed) |
| `InstalledDepots` | Map of depot ID to manifest ID and size |
| `TargetBuildID` | Pending update target (`0` = none) |
| `BytesToDownload` | Bytes remaining for pending update (`0` = none) |

### VDF (Valve Data Format)

VDF is a nested key-value text format used throughout Steam's local configuration. It uses braces for nesting and quotes for keys and values. It is not JSON -- it has no commas, no colons, and supports bare keys in some contexts.

ACF files are a specific use of VDF. Rewind needs a dedicated VDF parser rather than regex-based extraction, because the format supports arbitrary nesting and the field layout can vary between games.

## Steam Path Locations

Steam installs to platform-specific default locations:

| Platform | Default steamapps path |
|----------|----------------------|
| Linux | `~/.local/share/Steam/steamapps/` |
| macOS | `~/Library/Application Support/Steam/steamapps/` |
| Windows | `C:\Program Files (x86)\Steam\steamapps\` |

Steam also supports additional library folders configured in `libraryfolders.vdf`. A complete implementation should check this file for alternate install locations.

## How Steam Detects Updates

When Steam starts (or periodically while running), it compares the locally recorded `buildid` in each ACF file against the latest build ID from Steam's servers. If the local value is lower, Steam queues an update.

Steam may also validate file integrity through its internal database, independent of the ACF file. This is an area that needs further investigation -- the manifest locking strategy (see [decisions/no-backup-mvp](../decisions/no-backup-mvp.md)) works around this by making the ACF file immutable.
