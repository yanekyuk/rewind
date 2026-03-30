---
trigger: "Save brainstorming results as foundation documentation — domain knowledge, specs, and design decisions for the Rewind Steam game downgrader"
type: docs
branch: docs/foundation-docs
base-branch: main
created: 2026-03-30
---

## Related Files
- INIT.md (original project spec / manual process documentation)
- docs/swe-config.json (project configuration)
- src-tauri/Cargo.toml (backend manifest)
- package.json (frontend manifest)
- src-tauri/src/lib.rs (backend entry)
- src/App.tsx (frontend entry)

## Relevant Docs
None — knowledge base does not cover this area yet. This is the initial documentation effort.

## Related Issues
None — no remote configured yet.

## Scope
Write the foundational project documentation based on the brainstorming session. This includes:

### Domain Knowledge (docs/domain/)
- **Steam internals**: depots, manifests, ACF/VDF format, buildids, how Steam detects updates
- **Downgrade process**: the 9-step manual workflow and why each step is necessary
- **DepotDownloader**: capabilities, limitations, CLI interface, GPL-2.0 license implications
- **Platform differences**: Linux (chattr +i), macOS (chflags uchg), Windows (read-only attribute)

### Specifications (docs/specs/)
- **MVP scope**: game-agnostic downgrader, v0.1 feature set
- **Core flow**: detect Steam → list games → user picks game → user provides manifest ID → diff → download → apply → patch ACF → lock manifest → remind user
- **Auth flow**: in-app credential input passed to DepotDownloader
- **Version discovery**: auto-detect current installed version + manual manifest ID input for target

### Design Decisions (docs/decisions/)
- **GPL-2.0 licensing**: chosen to allow bundling DepotDownloader directly
- **DepotDownloader as Tauri sidecar**: bundled self-contained binary (~33MB per platform), no .NET dependency
- **Manual manifest ID input**: Steam has no API for historical manifests; SteamDB is the user's source
- **Privilege escalation via pkexec/polkit**: only for manifest locking step, not entire app
- **No backup in MVP**: Steam's "Verify integrity" serves as restore path
- **Embedded progress UI + background notifications**: for long-running downloads (tens of GBs)
- **Layered architecture**: domain/application/infrastructure separation with Tauri IPC boundary
