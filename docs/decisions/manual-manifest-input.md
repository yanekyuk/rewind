---
title: "Version Selection"
type: decision
tags: [manifest, version-discovery, steamdb, ux, steamkit, sidecar, auth]
created: 2026-03-30
updated: 2026-03-30
---

# Version Selection

## Context

To downgrade a game, the user must specify which version to downgrade to. This requires a manifest ID -- a numeric identifier for a specific snapshot of a depot's contents.

Steam has no public API for listing historical manifests for a depot. However, SteamKit2 (via the custom sidecar) can list available manifests for depots the authenticated user owns.

## Decision

The app fetches and displays available manifests using the SteamKit sidecar, replacing the manual-only manifest input. Manual input is retained as a fallback.

The "Select Version" step:
1. Prompts for Steam credentials (username/password) inline
2. Calls the SteamKit sidecar's `list-manifests` command with the game's depot
3. Displays a selectable list of manifests with dates
4. Also allows manual manifest ID entry as a fallback

## Rationale

- **SteamKit2 supports manifest listing**: The sidecar's `list-manifests` command returns available manifests for depots the authenticated user owns. This eliminates the need for users to visit SteamDB.
- **Better UX**: Users see available versions directly in the app rather than navigating to an external website.
- **Auth required**: Manifest listing requires Steam credentials, which the app collects inline and passes to the sidecar. Credentials are not persisted by Rewind; the sidecar handles session management.
- **Structured output**: The sidecar returns JSON, eliminating fragile text parsing.
- **Manual fallback preserved**: Users who already know their manifest ID (e.g., from SteamDB or community guides) can still enter it directly.

## Previous Approaches

**MVP approach**: Users manually input the target manifest ID due to:
- Time constraint on implementing manifest listing
- Authentication handling complexity
- Manual input being acceptable for MVP users

**Original approach**: DepotDownloader's command-line tool was used, but had limitations:
- Fragile text parsing of manifest lists
- No native 2FA support
- Required stdin relay for Steam Guard codes

## Future Improvements

- Cache known manifest-to-version mappings submitted by the community.
- Parse Steam news feeds or SteamDB patch notes to map build IDs to human-readable version names.
- Support multi-depot games (show depot selector or list all depots).
