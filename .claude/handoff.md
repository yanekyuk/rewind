---
trigger: "Improve manifest listing to properly display branch manifests with correct labels instead of treating branch names as dates"
type: feat
branch: feat/manifest-history
base-branch: main
created: 2026-03-31
---

## Related Files
- sidecar/SteamKitSidecar/Commands/ListManifestsCommand.cs (manifest listing from PICS)
- src-tauri/src/domain/manifest/mod.rs (ManifestListEntry type)
- src-tauri/src/domain/manifest/list_parser.rs (NDJSON parsing)
- src-tauri/src/infrastructure/depot_downloader.rs (list_manifests function)
- src/components/VersionSelect.tsx (manifest display UI)
- src/hooks/useManifestList.ts (data fetching)
- src/types/manifest.ts (frontend types)

## Relevant Docs
- docs/domain/steamkit-sidecar.md
- docs/domain/steam-internals.md
- docs/sidecar-architecture.md
- docs/decisions/manual-manifest-input.md

## Related Issues
None — no related issues found.

## Scope
Steam's PICS API only returns current manifests per branch (public, beta, bleeding-edge, etc.), NOT historical versions. The current sidecar returns branch names in the `date` field which is misleading.

Changes needed:

Sidecar:
- Update ListManifestsCommand to return richer data: branch name, manifest GID, depot size, download size (all available in PICS KeyValues)
- Consider extracting additional metadata like `pwdrequired`, `timeupdated` if available

Backend:
- Update `ManifestListEntry` to include branch name as a proper field (not abusing `date`)
- Add optional fields for size info

Frontend:
- Update VersionSelect to display branch names properly (e.g., "public", "beta") instead of as dates
- Show manifest size info if available
- Add a manual manifest ID input field (per docs/decisions/manual-manifest-input.md) for users who know their target manifest from SteamDB
- Highlight the current manifest in the list
