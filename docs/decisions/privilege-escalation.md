---
title: "Privilege Escalation via pkexec/polkit"
type: decision
tags: [privilege-escalation, pkexec, polkit, security, manifest-locking, platform]
created: 2026-03-30
updated: 2026-03-30
---

# Privilege Escalation via pkexec/polkit

## Context

Manifest locking (Step 8 of the downgrade process) requires elevated privileges on Linux and macOS. The `chattr +i` command on Linux and `chflags uchg` on macOS both require root access.

Options considered:

1. **Run the entire app as root/admin** -- security risk, poor practice.
2. **Escalate only for the locking step** -- minimal privilege surface.
3. **Skip manifest locking** -- Steam would overwrite the ACF file, undoing the downgrade.

## Decision

Escalate privileges only for the manifest locking step, using platform-native mechanisms.

## Implementation

### Linux

Use `pkexec` to run only the `chattr` command with root privileges:

```bash
pkexec chattr +i /path/to/appmanifest_<appid>.acf
```

This triggers a polkit authentication dialog, which is standard on desktop Linux. Most desktop environments (GNOME, KDE, XFCE) include a polkit agent by default.

### macOS

Use `osascript` to trigger the standard macOS administrator authentication dialog for the `chflags` command.

### Windows

No escalation needed. The read-only attribute can be set without administrator privileges.

## Rationale

- **Principle of least privilege**: The app handles game files (user-owned) without elevation. Only the single `chattr`/`chflags` operation needs root.
- **User trust**: Running the entire app as root would raise security concerns. A single, visible privilege prompt for a specific action is transparent and expected.
- **Platform conventions**: `pkexec` and macOS authorization dialogs are the standard patterns for requesting temporary elevation on their respective platforms.

## Risks

- **Missing polkit agent**: On minimal Linux installations (e.g., window managers without a desktop environment), no polkit agent may be running. The `pkexec` call would fail silently or produce an unhelpful error. Rewind should detect this and guide the user.
- **User denial**: If the user denies the privilege request, the manifest will not be locked. Rewind should warn that Steam may overwrite the downgrade.
