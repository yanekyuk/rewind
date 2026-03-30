# Rewind - Steam Game Downgrader

A cross-platform Tauri app that automates downgrading Steam games to previous versions.

## How It Works

The process has been validated manually for Crimson Desert and should generalize to any Steam game. Below are the exact steps the app needs to automate.

## Step-by-Step Process

### 1. Detect Steam Installation

Locate the Steam installation and `steamapps` folder:

- **Linux:** `~/.local/share/Steam/steamapps/`
- **macOS:** `~/Library/Application Support/Steam/steamapps/`
- **Windows:** `C:\Program Files (x86)\Steam\steamapps\`

### 2. Identify Installed Games

Parse `appmanifest_<appid>.acf` files in the `steamapps` folder. These are VDF (Valve Data Format) text files containing:

```
"AppState"
{
    "appid"        "3321460"
    "name"         "Crimson Desert"
    "buildid"      "22560074"
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
- `appid` — Steam app ID
- `name` — game name
- `buildid` — currently installed build
- `InstalledDepots` — map of depot ID to manifest ID and size
- `installdir` — game folder name under `steamapps/common/`

### 3. Fetch Available Versions

Use the SteamKit2 library (or equivalent) to list all available manifests for each depot. This requires Steam authentication.

Alternatively, scrape SteamDB at `https://steamdb.info/depot/<depotid>/manifests/` — but this requires a logged-in SteamDB session and is less reliable.

The best approach is to use DepotDownloader's `-manifest-only` flag as a subprocess:

```
DepotDownloader -app <appid> -depot <depotid> -manifest <manifestid> -manifest-only -username <user> -remember-password -dir <output>
```

This generates a text file listing all files with their sizes, chunk counts, and SHA hashes:

```
Content Manifest for Depot 3321461

Manifest ID / date     : 3559081655545104676 / 03/22/2026 16:01:45
Total number of files  : 257
Total number of chunks : 130874
Total bytes on disk    : 133352312992
Total bytes compressed : 100116131120

          Size Chunks File SHA                                 Flags Name
       6740755      7 8a11847b3e22b2fb909b57787ed94d1bb139bcb2     0 0000/0.pamt
     912261088    896 3e6800918fef5f8880cf601e5b60bff031465e60     0 0000/0.paz
```

### 4. Compare Manifests (Diff)

To avoid downloading the entire game, compare the manifest of the target version against the current version:

1. Download manifest metadata for both versions (target + current) using `-manifest-only`
2. Parse both manifest text files
3. Compare files by their SHA hash — files with different hashes need to be downloaded
4. Generate a filelist of only the changed files

For Crimson Desert 1.01.01 -> 1.00.03: 153 of 257 files changed (~80GB instead of ~133GB).

### 5. Download Changed Files

Use DepotDownloader with the `-filelist` flag to download only the changed files:

```
DepotDownloader -app <appid> -depot <depotid> -manifest <target_manifest> -username <user> -remember-password -filelist <changed_files.txt> -dir <download_dir>
```

This requires Steam credentials. DepotDownloader supports `-remember-password` to cache the session.

### 6. Apply the Downgrade

With Steam **fully closed**:

1. Copy all downloaded files over the game installation directory, overwriting existing files
2. The game install dir is at `steamapps/common/<installdir>/`

### 7. Patch the App Manifest

Edit `appmanifest_<appid>.acf` to prevent Steam from detecting a version mismatch:

| Field | Set To | Reason |
|-------|--------|--------|
| `buildid` | Latest build ID (not the target) | Tricks Steam into thinking game is up to date |
| `manifest` (under InstalledDepots) | Latest manifest ID (not the target) | Same reason |
| `size` (under InstalledDepots) | Latest size value | Consistency |
| `StateFlags` | `4` | Means "fully installed" |
| `TargetBuildID` | `0` | No pending update target |
| `FullValidateAfterNextUpdate` | `0` | Prevent validation on next launch |
| `BytesToDownload` | `0` | No pending download |

**Important:** The buildid and manifest must be set to the **latest** values, not the target version. This is what prevents Steam from showing "Update Required".

### 8. Lock the Manifest File

Prevent Steam from overwriting the patched manifest:

- **Linux:** `sudo chattr +i <path>` (undo: `sudo chattr -i`)
- **macOS:** `sudo chflags uchg <path>` (undo: `sudo chflags nouchg`)
- **Windows:** Set read-only attribute via `SetFileAttributes`

Note: On Linux, `chmod 444` is not sufficient — Steam can bypass it. The immutable flag (`chattr +i`) requires root privileges.

### 9. Set Steam Update Preference

This step is manual — the user should set the game's update preference to "Wait until I launch the game" in Steam's Properties > Updates.

The app could remind the user to do this after the downgrade completes.

## Known Limitations

- **Denuvo games** require periodic online authentication, so permanent offline mode is not viable
- **GeForce Now / cloud gaming** users cannot use this tool as they have no access to local game files
- **Verifying game files** in Steam will undo the downgrade
- **Steam may still detect the mismatch** through its internal database even with the manifest locked — this needs further investigation
- **Manifest availability** depends on Steam's servers retaining old manifests. Some games may have manifests removed by the developer

## Dependencies

- [DepotDownloader](https://github.com/SteamRE/DepotDownloader) — used as a subprocess for downloading manifests and game files. Requires .NET runtime.
- Steam account credentials for authentication

## Future Considerations

- Could integrate SteamKit2 directly (Rust bindings or reimplementation) to avoid the DepotDownloader dependency
- Build ID to version name mapping could be scraped from SteamDB patch notes or the game's Steam news feed
- Backup system: save current files before overwriting so the user can restore without re-downloading
- Profile system: save known-good manifest IDs per game for quick switching
