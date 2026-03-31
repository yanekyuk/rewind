---
trigger: "Show non-installed depots in the game detail view by adding a sidecar command that lists all depots for an app via Steam PICS API"
type: feat
branch: feat/non-installed-depots
base-branch: main
created: 2026-03-31
---

## Related Files
- sidecar/SteamKitSidecar/Commands/ListManifestsCommand.cs (already queries PICS, accesses depots KeyValues)
- sidecar/SteamKitSidecar/Program.cs (command registration)
- src-tauri/src/domain/game.rs (GameInfo, DepotInfo)
- src-tauri/src/domain/vdf/acf.rs (AppState, InstalledDepot)
- src-tauri/src/infrastructure/depot_downloader.rs (sidecar communication)
- src-tauri/src/lib.rs (IPC commands)
- src/components/GameDetail.tsx (depot display UI)
- src/types/game.ts (frontend types)

## Relevant Docs
- docs/domain/steamkit-sidecar.md
- docs/domain/steam-internals.md
- docs/sidecar-architecture.md

## Related Issues
None — no related issues found.

## Scope
Add a new sidecar command `list-depots` that queries `PICSGetProductInfo()` and enumerates ALL depots for an app (iterating `depots.Children` instead of accessing a single depot). Return each depot's ID and metadata (name, size if available, whether it's a DLC depot, etc.).

Backend changes:
- New function in `infrastructure/depot_downloader.rs` to call `list-depots`
- New domain type for depot info from Steam (distinct from `DepotInfo` which is for installed depots)
- New IPC command `list_depots(app_id)` that returns all depots

Frontend changes:
- Update GameDetail to show both installed and non-installed depots
- Installed depots show manifest + size (existing)
- Non-installed depots shown with different styling (greyed out or similar)
- Each depot should be selectable for version browsing
