---
title: "MVP Scope (v0.1)"
type: spec
tags: [mvp, scope, v0.1, downgrade, workflow, auth]
created: 2026-03-30
updated: 2026-03-30
---

# MVP Scope (v0.1)

Rewind v0.1 is a game-agnostic Steam downgrader. It automates the manual 9-step downgrade process described in [domain/downgrade-process](../domain/downgrade-process.md).

## Goal

Allow any Steam user to downgrade any installed Steam game to a previous version, with minimal technical knowledge required beyond finding the target manifest ID on SteamDB.

## Core Flow

```
Detect Steam
    |
    v
List Installed Games
    |
    v
User Picks Game
    |
    v
User Provides Target Manifest ID  (manual input -- see Version Discovery below)
    |
    v
Diff Manifests (current vs target)
    |
    v
Download Changed Files  (via DepotDownloader sidecar)
    |
    v
Apply Files to Game Directory  (Steam must be closed)
    |
    v
Patch ACF Manifest  (set buildid/manifest to latest values)
    |
    v
Lock Manifest File  (platform-specific immutable flag)
    |
    v
Remind User  (set Steam update preference to "Only update when I launch")
```

## Feature Set

### Included in v0.1

- **Steam detection**: Auto-detect Steam installation path on Linux, macOS, and Windows. Check default paths and `libraryfolders.vdf` for additional libraries.
- **Game listing**: Parse ACF files to show installed games with name, app ID, and current build ID.
- **Manual manifest input**: User enters the target manifest ID (sourced from SteamDB).
- **Manifest diffing**: Fetch both manifests via DepotDownloader `-manifest-only`, parse output, diff by SHA hash, generate filelist of changed files.
- **Selective download**: Use DepotDownloader with `-filelist` to download only changed files.
- **Progress tracking**: Embedded progress UI showing download status. Background notifications for long-running downloads.
- **File application**: Copy downloaded files over the game directory (with Steam closed).
- **ACF patching**: Modify the ACF file to spoof the latest version.
- **Manifest locking**: Make the ACF file immutable using platform-specific methods (chattr +i, chflags uchg, read-only attribute).
- **User reminder**: Post-downgrade prompt to set Steam update preference.
- **Authentication**: In-app credential input (username, password, Steam Guard code) passed to DepotDownloader.

### Explicitly Excluded from v0.1

- **Automatic version discovery**: No API exists to list historical manifests. SteamDB scraping is fragile. Users must find manifest IDs manually.
- **Backup/restore**: Steam's "Verify integrity of game files" serves as the restore path. See [decisions/no-backup-mvp](../decisions/no-backup-mvp.md).
- **Profile system**: No saved configurations for known-good manifest IDs per game.
- **Build ID to version mapping**: No automatic mapping between Steam build IDs and human-readable version numbers.
- **Multi-depot games**: v0.1 targets single-depot games. Multi-depot support is deferred.
- **Offline mode management**: Users handle Steam's offline mode themselves.

## Version Discovery

The MVP uses a hybrid approach:

1. **Current version**: Auto-detected by parsing the installed game's ACF file (reading `buildid` and `InstalledDepots` manifest values).
2. **Target version**: Manual input. The user navigates to `https://steamdb.info/depot/<depotid>/manifests/` and copies the manifest ID for their desired version.

The app displays the current depot ID to help users find the correct SteamDB page.

## Authentication Flow

1. User enters Steam username and password in the app.
2. If Steam Guard is enabled, the app prompts for the 2FA code (email or mobile authenticator).
3. Credentials are passed to DepotDownloader as command-line arguments.
4. DepotDownloader's `-remember-password` flag caches the session for subsequent operations.
5. Credentials are never persisted by Rewind -- only DepotDownloader's session cache is retained.

## Technical Constraints

- **Tauri 2 + React 19**: Frontend is a React SPA communicating with a Rust backend via Tauri IPC commands.
- **DepotDownloader sidecar**: Bundled as a self-contained binary (~33 MB per platform). No .NET runtime dependency for users.
- **Layered architecture**: Domain/application/infrastructure separation. See [decisions/layered-architecture](../decisions/layered-architecture.md).
- **Cross-platform**: Must work on Linux, macOS, and Windows. Platform-specific code is isolated in the infrastructure layer.
