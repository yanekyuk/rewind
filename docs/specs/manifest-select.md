---
title: "Manifest Select"
type: spec
tags: [manifest, version-select, depotdownloader, auth, ui]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Replace the manual "Enter Manifest ID" step with a "Select Version" step that fetches and displays available manifests for the selected game's depots using DepotDownloader.

### Backend: `list_manifests` IPC command

A new Tauri IPC command `list_manifests` that:

1. Accepts an app ID and depot ID
2. Reads credentials from the AuthStore (returns `AuthRequired` error if not set)
3. Spawns DepotDownloader with stored credentials to list available manifests
4. Parses the output into a list of manifest entries (manifest ID, date/time)
5. Returns the list to the frontend as a JSON array

DepotDownloader is invoked with credentials from the AuthStore:
```
DepotDownloader -username <user> -password <pass> -remember-password -app <appid> -depot <depotid>
```

The output contains lines listing available manifests with their IDs and dates. The parser extracts these into structured types.

### Domain types

A new `ManifestListEntry` type in the domain layer:
- `manifest_id: String` -- the manifest identifier
- `date: String` -- the date/time string from DepotDownloader output

### Frontend: ManifestSelect component

Replaces the placeholder StepView for step 1 (index 1). Follows the same pattern as GameSelect:

1. Receives the selected game from App state
2. Auto-fetches manifests on mount (credentials are already stored in AuthStore from the auth step)
3. Calls `list_manifests` IPC with the game's first depot (no credentials in the IPC call)
4. Shows loading/error/empty states
5. Renders manifest entries in a selectable list showing date and manifest ID
6. Provides a manual input fallback for users who already know their manifest ID
7. Emits the selected manifest ID to App state

### Auth handling

Credentials are stored in the Rust AuthStore during the authentication step (step 2) and read server-side by the `list_manifests` IPC command. ManifestSelect does not collect or pass credentials -- it relies on the auth gate to ensure credentials exist before this step is reached. See `docs/specs/auth-ui.md` for credential storage details.

### Step definition update

Update step 1 in steps.ts:
- Change ID from `enter-manifest` to `select-version`
- Change label to "Select Version"
- Update description

## Constraints

- Authentication is required -- DepotDownloader needs Steam credentials to list manifests
- Manual input must remain as a fallback option
- Credentials must never be logged or persisted by Rewind
- Cross-platform: all paths and subprocess calls must work on Linux, macOS, and Windows
- The `list_manifests` command must be cancellable (user navigating away should not leave orphaned processes)
- Domain types must not import infrastructure or application layers
- Frontend communicates with backend only via Tauri IPC

## Acceptance Criteria

1. `list_manifests` IPC command accepts app_id and depot_id; reads credentials from AuthStore; spawns DepotDownloader and returns parsed manifest entries
2. DepotDownloader output is parsed into `ManifestListEntry` structs with manifest_id and date fields
3. ManifestSelect component displays a loading state while fetching manifests
4. ManifestSelect component displays an error state with retry option on failure
5. ManifestSelect component renders a selectable list of manifest entries
6. ManifestSelect component provides a manual manifest ID input as fallback
7. Selected manifest ID is passed up to App state
8. Next button is disabled until a manifest is selected (either from list or manual input)
9. Step definition updated with new ID, label, and description
10. Manifests are auto-fetched on mount using credentials from AuthStore (no inline credential inputs)
11. The decision doc `manual-manifest-input.md` is updated to reflect this change
