---
title: "Version Selection"
type: decision
tags: [manifest, version-discovery, steamdb, ux, depotdownloader, auth]
created: 2026-03-30
updated: 2026-03-30
---

# Version Selection

## Context

To downgrade a game, the user must specify which version to downgrade to. This requires a manifest ID -- a numeric identifier for a specific snapshot of a depot's contents.

Steam has no public API for listing historical manifests for a depot. However, DepotDownloader (via SteamKit2) can list available manifests for depots the authenticated user owns.

## Decision

**Updated from MVP:** The app now fetches and displays available manifests using DepotDownloader, replacing the manual-only manifest input. Manual input is retained as a fallback.

The "Select Version" step:
1. Prompts for Steam credentials (username/password) inline
2. Calls DepotDownloader to list available manifests for the game's depot
3. Displays a selectable list of manifests with dates
4. Also allows manual manifest ID entry as a fallback

## Rationale

- **DepotDownloader supports manifest listing**: When authenticated, DepotDownloader can list all manifests for depots the user owns. This eliminates the need for users to visit SteamDB.
- **Better UX**: Users see available versions directly in the app rather than navigating to an external website.
- **Auth required**: Manifest listing requires Steam credentials, which the app collects inline and passes to DepotDownloader. Credentials are never persisted by Rewind; DepotDownloader's `-remember-password` flag handles session caching.
- **Manual fallback preserved**: Users who already know their manifest ID (e.g., from SteamDB or community guides) can still enter it directly.

## Previous Decision (MVP)

In the MVP, users manually input the target manifest ID. This was due to:
- SteamKit2 manifest listing not being straightforward
- Additional authentication handling being needed
- Manual input being an acceptable UX trade-off for the target audience

## Future Improvements

- Cache known manifest-to-version mappings submitted by the community.
- Parse Steam news feeds or SteamDB patch notes to map build IDs to human-readable version names.
- Support multi-depot games (show depot selector or list all depots).
