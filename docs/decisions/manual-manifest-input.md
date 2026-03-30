---
title: "Manual Manifest ID Input"
type: decision
tags: [manifest, version-discovery, steamdb, ux]
created: 2026-03-30
updated: 2026-03-30
---

# Manual Manifest ID Input

## Context

To downgrade a game, the user must specify which version to downgrade to. This requires a manifest ID -- a numeric identifier for a specific snapshot of a depot's contents.

Steam has no public API for listing historical manifests for a depot. The only reliable source is [SteamDB](https://steamdb.info/), which tracks manifest history through its own data collection.

## Decision

In the MVP, users manually input the target manifest ID. The app displays the current depot ID to help users find the correct SteamDB page.

## Rationale

- **No official API**: Steam/Valve does not expose an endpoint for historical manifest listings.
- **SteamDB scraping is fragile**: SteamDB requires authentication, uses anti-scraping measures, and its HTML structure can change without notice. Building a scraper would create a brittle dependency.
- **SteamKit2 manifest listing**: DepotDownloader/SteamKit2 can list manifests for depots the user owns, but the interface is not straightforward and requires additional authentication handling. This is a candidate for post-MVP improvement.
- **Acceptable UX trade-off**: The target audience (gamers who want to downgrade) is generally comfortable navigating SteamDB. The manifest ID is a copy-paste operation.

## User Workflow

1. Rewind shows the installed game's depot ID.
2. User navigates to `https://steamdb.info/depot/<depotid>/manifests/`.
3. User finds the desired version and copies the manifest ID.
4. User pastes the manifest ID into Rewind.

## Future Improvements

- Integrate SteamKit2's manifest listing capability to show available versions directly in the app.
- Cache known manifest-to-version mappings submitted by the community.
- Parse Steam news feeds or SteamDB patch notes to map build IDs to human-readable version names.
