# Version Management Redesign

## Summary

Redesign the downgrade/upgrade flow to clearly separate "download new version" from "switch cached version", improve SteamDB guidance, show explicit manifest state, and automatically manage locking based on whether the user is on the latest version.

## State Model

Games have two states:

- **Updates disabled** — ACF is locked, Steam cannot update. Active manifest differs from latest (or user explicitly downgraded).
- **Updates enabled** — ACF is unlocked, Steam manages updates freely. Active manifest equals latest.

### Detail Panel Display

- `Installed: <active_manifest_id>` — what's actually on disk
- `Spoofed as: <latest_manifest_id>` — only shown when updates are disabled
- `Cached versions: <count>` — number of versions in cache

### Game List Indicators

- `▼` — Updates disabled (downgraded/pinned)
- `✓` — Updates enabled

## Key Bindings

### Main Screen

| Key | Action |
|-----|--------|
| `[D]` | Download new version (opens wizard) |
| `[U]` | Switch version (opens picker, requires 2+ cached versions) |
| `[O]` | Open app on SteamDB |
| `[↑↓/jk]` | Navigate games |
| `[S]` | Settings |
| `[Q]` | Quit |

The `[L]` lock toggle is removed. Locking is automatic based on version state.

### Download Wizard

| Key | Action |
|-----|--------|
| `[P]` | Open patches page on SteamDB |
| `[M]` | Open manifests page on SteamDB |
| `[Enter]` | Download pasted manifest ID |
| `[Esc]` | Cancel |

### Switch Version Overlay

| Key | Action |
|-----|--------|
| `[↑↓]` | Navigate cached versions |
| `[Enter]` | Switch to selected version |
| `[Esc]` | Cancel / dismiss |

## Download Wizard (D)

Single-screen layout with guidance text and manifest ID input:

```
┌─ Download New Version ──────────────────────────────┐
│                                                      │
│  Find your target version:                           │
│  [P] Patches  — browse patch notes to find the       │
│                 version you want, note its date       │
│  [M] Manifests — match the date to find the          │
│                  corresponding manifest ID            │
│                                                      │
│  Manifest ID: [____________________________]         │
│                                                      │
│  [Enter] Download  [Esc] Cancel                      │
├──────────────────────────────────────────────────────┤
│  Output:                                             │
│  ...                                                 │
└──────────────────────────────────────────────────────┘
```

### SteamDB URLs

- Patches: `https://steamdb.info/app/{app_id}/patchnotes/`
- Manifests: `https://steamdb.info/depot/{depot_id}/manifests/`

### Guidance Text

The wizard explains how to cross-reference the two pages:

> Press `[P]` to open **Patches** — find the version you want by reading patch notes and note its **date**.
> Press `[M]` to open **Manifests** — match the date to find the corresponding **manifest ID**.

After pressing `[Enter]`, the screen transitions to the download progress view with the existing 7-step display (CheckDotnet, DownloadDepot, DownloadManifest, BackupFiles, LinkFiles, PatchManifest, LockManifest).

## Switch Version Overlay (U)

### Phase 1 — Version Selection

```
┌─ Switch Version ─────────────────────────────────────┐
│                                                       │
│  Select a version:                                    │
│    ● 123456789  (installed)                           │
│      987654321                                        │
│      111222333  (latest)                              │
│                                                       │
│  [↑↓] Navigate  [Enter] Switch  [Esc] Cancel         │
└───────────────────────────────────────────────────────┘
```

The latest manifest is labeled `(latest)` so users know which one restores updates.

### Phase 2 — Progress

```
┌─ Switch Version ─────────────────────────────────────┐
│                                                       │
│  Switching to 987654321...                            │
│                                                       │
│  [✓] Repoint symlinks                                │
│  [✓] Patch ACF                                       │
│  [✓] Lock ACF                                        │
│                                                       │
│  Done! [Esc] Close                                    │
└───────────────────────────────────────────────────────┘
```

When switching to the latest version, step 3 shows:
`[—] Lock ACF (skipped — updates enabled)`

The overlay stays open after completion so the user can see all steps before pressing `[Esc]` to dismiss.

## Logic Changes

### Switch to Non-Latest Version

1. Repoint symlinks to target cache directory
2. Patch ACF with `latest_manifest_id` + `latest_buildid` (spoof)
3. Lock ACF
4. Set `active_manifest_id` = selected manifest
5. Status becomes "Updates disabled"

### Switch to Latest Version

1. Repoint symlinks to latest cache directory
2. Patch ACF back to real values (no spoofing needed)
3. Do **not** lock ACF
4. Set `active_manifest_id` = `latest_manifest_id`
5. Status becomes "Updates enabled"

### Download Completes (Wizard)

- Same as current flow — always locks, always sets "Updates disabled"
- Newly downloaded manifest added to `cached_manifest_ids`

### Startup Repair

- Only repair games that are "Updates disabled" (locked)
- Skip games with "Updates enabled" — Steam is managing those

### Removed

- `[L]` lock toggle removed from main screen
- Manual lock/unlock is no longer exposed to the user
