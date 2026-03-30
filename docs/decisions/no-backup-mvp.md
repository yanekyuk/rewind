---
title: "No Backup in MVP"
type: decision
tags: [backup, restore, mvp, scope, steam]
created: 2026-03-30
updated: 2026-03-30
---

# No Backup in MVP

## Context

When applying a downgrade, Rewind overwrites game files with older versions. If the user wants to return to the latest version, they need a way to restore.

Options considered:

1. **Backup current files before overwriting** -- safe but requires significant disk space (potentially 50-100+ GB).
2. **Rely on Steam's "Verify integrity of game files"** -- Steam re-downloads any files that don't match the latest manifest.
3. **Implement a snapshot/delta system** -- complex, deferred to post-MVP.

## Decision

No backup functionality in v0.1. Steam's built-in "Verify integrity of game files" serves as the restore path.

## Rationale

- **Disk space**: Games can be 50-150 GB. Backing up even the changed files (often 60-80% of total size) would require significant additional disk space. Many users are already disk-constrained.
- **Steam provides a restore path**: "Verify integrity of game files" forces Steam to re-download all files that don't match the latest manifest. This effectively restores the game to the latest version. The user must also unlock the manifest file (remove the immutable flag) before verifying.
- **Scope management**: Backup adds complexity (storage management, partial backups, cleanup) that is not essential for the core downgrading use case.

## Restore Procedure (for users)

1. Unlock the manifest: remove the immutable flag (Rewind can automate this).
2. Open Steam, right-click the game, go to Properties > Installed Files > Verify integrity of game files.
3. Steam will re-download all files that differ from the latest version.

## Future Improvements

- Save changed files to a cache directory before overwriting, enabling instant rollback.
- Implement a profile system that stores manifest IDs for known-good versions, making it easy to switch between versions without re-downloading.
