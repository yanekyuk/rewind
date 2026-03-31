---
title: "Manifest Listing Overhaul"
type: spec
tags: [manifest, sidecar, steamkit, version-select, ui, pics, branch]
created: 2026-03-31
updated: 2026-03-31
---

## Behavior

Improve the manifest listing pipeline (sidecar, backend, frontend) to return richer data from Steam's PICS API and display branch manifests with proper labels instead of abusing the `date` field for branch names.

### Sidecar: Richer manifest list output

Update `ListManifestsCommand` to emit additional fields available in the PICS KeyValues tree for each branch manifest:

- `branch` (string) -- the branch name (e.g., "public", "beta", "bleeding-edge")
- `size_on_disk` (ulong, optional) -- depot size from the `maxsize` key
- `size_compressed` (ulong, optional) -- compressed download size from the `systemdefined` key, if available
- `time_updated` (ulong, optional) -- Unix timestamp from the `timeupdated` key on the branch
- `pwd_required` (bool, optional) -- whether the branch requires a password from the `pwdrequired` key

Remove debug logging (depot key dumps, children descriptions) that was left from initial development.

Updated `ManifestListItem` shape:

```json
{
  "id": "7446650175280810671",
  "branch": "public",
  "time_updated": 1711123305,
  "pwd_required": false
}
```

The `date` field is removed from the sidecar output since it was being misused to hold branch names. The `branch` field replaces it with correct semantics.

### Backend: Updated domain types and parser

Update `ManifestListEntry` in `domain/manifest/mod.rs`:

- Add `branch: Option<String>` -- the Steam branch name
- Add `time_updated: Option<u64>` -- Unix timestamp of the branch update
- Add `pwd_required: Option<bool>` -- whether the branch requires a password
- Keep `date: String` as a computed display value (formatted from `time_updated` or empty)

Update `list_parser.rs` to handle the new fields from the sidecar envelope.

### Frontend: Improved version display

Update `ManifestListEntry` type in `src/types/manifest.ts` to include the new fields.

Update `VersionSelect` component:

1. Display branch name as the primary label for each version row (e.g., "public", "beta")
2. Show formatted time if `time_updated` is available
3. Highlight the currently installed manifest in the list with a "current" badge
4. Show a lock icon or "(password required)" indicator for branches with `pwd_required: true`
5. Add a manual manifest ID input field below the version list for users who know their target manifest from SteamDB

### Manual manifest input

Per `docs/decisions/manual-manifest-input.md`, provide a text input field where users can directly enter a manifest ID. This input should:

- Accept any numeric string as a manifest ID
- Call `onSelectManifest` with the entered value
- Be visually separated from the branch manifest list
- Include helper text explaining this is for advanced users who know their manifest ID

## Constraints

- PICS API only returns the current manifest per branch, not historical versions
- Domain types must not import from infrastructure or application layers
- Frontend communicates with backend only through Tauri IPC commands
- All field names use `snake_case` in JSON (consistent with existing protocol)
- The `id` alias on `manifest_id` must be preserved for backward compatibility with the sidecar
- Cross-platform: all paths and subprocess calls must work on Linux, macOS, and Windows

## Acceptance Criteria

1. Sidecar `list-manifests` output includes `branch`, `time_updated`, and `pwd_required` fields per manifest
2. Sidecar no longer emits debug logging (depot key dumps, children descriptions)
3. Rust `ManifestListEntry` has `branch`, `time_updated`, and `pwd_required` optional fields
4. Rust parser correctly deserializes the new fields from sidecar NDJSON
5. Frontend `ManifestListEntry` type includes the new fields
6. VersionSelect displays branch name as primary label per row
7. VersionSelect shows formatted update time when `time_updated` is available
8. VersionSelect highlights the currently installed manifest with a visual indicator
9. VersionSelect provides a manual manifest ID input field as fallback
10. Password-required branches show a visual indicator
11. Existing tests are updated to cover the new fields and UI elements
