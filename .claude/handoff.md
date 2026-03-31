---
trigger: "Depot list in GameDetail needs more info (names, config type, DLC app ID, max size for non-installed) and restyling. VersionSelect only shows 1 manifest per branch from SteamKit PICS — need to embed SteamDB webview (https://steamdb.info/depot/<depotId>/manifests/) so users can see full historical manifest list and select a version to downgrade to."
type: feat
branch: feat/steamdb-webview
base-branch: main
created: 2026-03-31
---

## Related Files
- src/components/GameDetail.tsx — depot list display, MergedDepot type, buildMergedDepots
- src/components/GameDetail.test.tsx — tests for depot display
- src/components/VersionSelect.tsx — manifest selection, currently only shows PICS manifests
- src/components/VersionSelect.test.tsx — tests for version selection
- src/types/game.ts — SteamDepotInfo type (has max_size, dlc_app_id already)
- src/hooks/useDepotList.ts — fetches depot list from PICS
- src/hooks/useManifestList.ts — fetches manifests from PICS (1 per branch)
- src/App.css — styling
- docs/specs/downgrade-ui.md — spec for SteamDB webview (section 1: Version Selection)

## Relevant Docs
- docs/specs/downgrade-ui.md — spec for SteamDB webview integration
- docs/specs/list-depots.md — list-depots sidecar command spec
- docs/specs/steam-ui-overhaul.md — Steam UI theming and navigation
- docs/specs/manifest-listing-overhaul.md — manifest listing with branch metadata

## Related Issues
None — no related issues found.

## Scope

### 1. Enrich GameDetail depot display
- Add `max_size` and `dlc_app_id` to MergedDepot type (already available in SteamDepotInfo)
- Show max_size for non-installed depots (formatted as GB/MB)
- Show DLC app ID badge when depot belongs to DLC
- Show depot names from PICS (they may be null for some depots — handle gracefully)
- Restyle depot cards: better button styling, clearer visual hierarchy, consistent with Steam theme

### 2. Embed SteamDB webview in VersionSelect
This is the core feature. When user clicks "Browse Versions" on a depot:
- VersionSelect loads with the selected depot ID
- Embed a Tauri webview pointing to `https://steamdb.info/depot/<depotId>/manifests/`
- User browses SteamDB naturally (may need to log in for older manifests)
- Inject JavaScript into the webview to extract the manifest history table from the DOM
- Parse extracted data: manifest ID, date, branch labels
- Present extracted manifests in a native list below/alongside the webview
- User can select a manifest from the extracted list to start downgrade
- Graceful fallback: if JS injection fails, user can still copy manifest ID and use manual entry

### 3. Layout: three manifest sources in VersionSelect
- **SteamDB Webview** — historical manifests extracted from embedded webview (primary)
- **SteamKit PICS** — current branch manifests from sidecar (existing, keep as-is)
- **Manual Entry** — free-form manifest ID input (existing, keep as-is)

### Tauri Webview Integration Notes
- Tauri 2 supports webviews via `tauri::WebviewBuilder` or the `<webview>` window API
- The webview needs to load an external URL (SteamDB)
- JS injection for DOM extraction can use `webview.eval()` or Tauri's `on_page_load` hook
- The extracted data should be communicated back to React via Tauri events or IPC
- Consider: webview as a separate Tauri window vs embedded in the React view
