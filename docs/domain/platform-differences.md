---
title: "Platform Differences"
type: domain
tags: [platform, linux, macos, windows, manifest-locking, privilege-escalation]
created: 2026-03-30
updated: 2026-03-30
---

# Platform Differences

Rewind targets Linux, macOS, and Windows. Most of the downgrade logic is platform-agnostic, but three areas require platform-specific handling: Steam path detection, manifest locking, and privilege escalation.

## Steam Path Detection

| Platform | Default steamapps path |
|----------|----------------------|
| Linux | `~/.local/share/Steam/steamapps/` |
| macOS | `~/Library/Application Support/Steam/steamapps/` |
| Windows | `C:\Program Files (x86)\Steam\steamapps\` |

All platforms support additional library folders configured in `steamapps/libraryfolders.vdf`. Rewind must read this file to discover games installed on secondary drives.

## Manifest Locking

After patching the ACF file (Step 8 of the downgrade process), the file must be made immutable to prevent Steam from overwriting it on next launch.

### Linux: `chattr +i`

```bash
sudo chattr +i /path/to/appmanifest_<appid>.acf
# Undo:
sudo chattr -i /path/to/appmanifest_<appid>.acf
```

The immutable flag (`+i`) prevents any modification, even by root, until the flag is removed. This is necessary because `chmod 444` (read-only permissions) is insufficient -- Steam runs as the same user and can bypass standard Unix permissions via its own file operations.

Requires root privileges.

### macOS: `chflags uchg`

```bash
sudo chflags uchg /path/to/appmanifest_<appid>.acf
# Undo:
sudo chflags nouchg /path/to/appmanifest_<appid>.acf
```

The `uchg` (user immutable) flag prevents modification. Requires root privileges to set on files not owned by the current user, though in practice Steam files are owned by the current user -- `sudo` is used for reliability.

### Windows: Read-Only Attribute

```
SetFileAttributes(path, FILE_ATTRIBUTE_READONLY)
```

The read-only attribute is less robust than Linux/macOS immutable flags. Steam may be able to clear the attribute and overwrite the file. This is a known limitation that needs further testing.

No privilege escalation is required on Windows for setting the read-only attribute.

## Privilege Escalation

Manifest locking on Linux and macOS requires elevated privileges. Rewind uses platform-specific mechanisms to request escalation only for the locking step, not for the entire application.

See [decisions/privilege-escalation](../decisions/privilege-escalation.md) for the design rationale.

### Linux: pkexec / polkit

Rewind uses `pkexec` to run the `chattr` command with root privileges. This displays a graphical authentication dialog (via polkit) asking the user to enter their password.

```bash
pkexec chattr +i /path/to/appmanifest.acf
```

Requires a polkit agent to be running (standard on most desktop Linux distributions).

### macOS: osascript

Rewind uses `osascript` to invoke an AppleScript that runs the `chflags` command with administrator privileges, triggering the standard macOS authentication dialog.

### Windows: No Escalation Needed

Setting the read-only attribute does not require administrator privileges.

## SteamKit Sidecar Binary

The SteamKit sidecar is bundled as a Tauri sidecar -- a self-contained, platform-specific binary:

| Platform | Binary | Approximate Size |
|----------|--------|-----------------|
| Linux | `SteamKitSidecar-x86_64-unknown-linux-gnu` | ~50-60 MB |
| macOS | `SteamKitSidecar-x86_64-apple-darwin` | ~50-60 MB |
| Windows | `SteamKitSidecar-x86_64-pc-windows-msvc.exe` | ~50-60 MB |

These are self-contained .NET binaries that include the runtime, eliminating the need for users to install .NET separately. See [decisions/depotdownloader-sidecar](../decisions/depotdownloader-sidecar.md) (now SteamKit2 migration decision) and [specs/sidecar-setup.md](../specs/sidecar-setup.md).
