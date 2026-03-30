---
trigger: "Migrate from DepotDownloader sidecar to a custom SteamKit-based .NET sidecar — replacing auth, manifest enumeration, and file download operations with structured JSON output"
type: feat
branch: feat/steamkit-migration
base-branch: main
created: 2026-03-30
---

## Related Files

### Infrastructure (to replace)
- src-tauri/src/infrastructure/sidecar.rs — current sidecar spawning, Steam Guard stdin relay, build_authenticated_args
- src-tauri/src/infrastructure/depot_downloader.rs — list_manifests subprocess orchestration (text parsing)
- src-tauri/src/infrastructure/mod.rs — module declarations

### Domain (to adapt)
- src-tauri/src/domain/auth.rs — Credentials struct, to_depot_args() builds CLI args for DepotDownloader
- src-tauri/src/domain/manifest/mod.rs — ManifestListEntry, DepotManifest, ManifestEntry types
- src-tauri/src/domain/manifest/list_parser.rs — text-based manifest list parser
- src-tauri/src/domain/manifest/parser.rs — text-based manifest file parser

### Application (to adapt)
- src-tauri/src/application/auth.rs — AuthStore (in-memory credential store)

### IPC (to refactor)
- src-tauri/src/lib.rs — Tauri commands: list_manifests takes username/password inline, should use AuthStore

### Frontend hooks (to refactor)
- src/hooks/useManifestList.ts — passes credentials to list_manifests IPC
- src/hooks/useAuth.ts — auth state management

### Build system (to replace)
- scripts/download-sidecar.sh — downloads DepotDownloader binaries
- package.json — ensure-sidecar script
- src-tauri/tauri.conf.json — externalBin: ["binaries/DepotDownloader"]

### Licensing (to update)
- src-tauri/resources/DEPOTDOWNLOADER-LICENSE — GPL-2.0 license for DepotDownloader (remove)

### Documentation (to update)
- docs/decisions/depotdownloader-sidecar.md — decision doc for sidecar approach
- docs/domain/depotdownloader.md — DepotDownloader CLI reference
- docs/specs/sidecar-setup.md — sidecar bundling spec

## Relevant Docs
- docs/decisions/depotdownloader-sidecar.md — current sidecar decision rationale
- docs/decisions/layered-architecture.md — architecture rules (infrastructure layer owns I/O)
- docs/domain/depotdownloader.md — DepotDownloader CLI, manifest format, auth flow
- docs/domain/steam-internals.md — Steam concepts (depots, manifests, CDN)

## Related Issues
None — no related issues found.

## Scope

### Goal
Replace the DepotDownloader sidecar with a custom .NET console app built on SteamKit2 (LGPL v2.1). This gives Rewind direct control over Steam operations with structured JSON output instead of fragile text parsing.

### Licensing
- SteamKit2: LGPL v2.1 (compatible with GPL-2.0, more permissive)
- DepotDownloader: GPL-2.0 (being removed)
- After migration: Rewind can potentially relicense away from GPL-2.0 since the LGPL v2.1 constraint is weaker

### What the new sidecar must support
1. **Authentication** — username/password login, Steam Guard 2FA (email + mobile authenticator), session persistence
2. **Manifest enumeration** — list historical manifest IDs for a given app/depot (the key gap DepotDownloader couldn't fill)
3. **Manifest fetching** — download and parse a specific manifest (file listings with SHA hashes, sizes, chunks)
4. **File downloading** — download depot files for a specific manifest via Steam CDN (chunk downloading + decryption)

### Communication protocol
- The .NET sidecar should communicate via **structured JSON on stdout** (one JSON object per line / newline-delimited JSON)
- Commands sent as CLI arguments or stdin
- Progress reporting via JSON events (not text parsing)
- Error reporting via JSON with error codes

### Implementation phases (suggested)
1. **Phase 1**: Scaffold .NET project with SteamKit2 dependency, implement auth flow (login, Steam Guard, session save)
2. **Phase 2**: Implement manifest listing (historical manifest IDs per depot)
3. **Phase 3**: Implement manifest download + parsing (file entries)
4. **Phase 4**: Implement file/depot downloading via CDN
5. **Phase 5**: Wire Rust infrastructure to new sidecar, remove DepotDownloader references
6. **Phase 6**: Update build system (replace download-sidecar.sh, update tauri.conf.json externalBin)

### What to remove
- DepotDownloader binaries and download script
- Text-based manifest/list parsers (replaced by JSON deserialization)
- GPL-2.0 license file for DepotDownloader
- stdin-based Steam Guard relay (auth handled natively by new sidecar)
