---
trigger: "Wire the existing download-sidecar.sh script into the build process so bun run tauri dev and bun run tauri build automatically download DepotDownloader binaries if missing."
type: chore
branch: chore/ensure-sidecar
base-branch: main
created: 2026-03-30
version-bump: none
---

## Related Files
- package.json (needs ensure-sidecar script wired into dev/build commands)
- scripts/download-sidecar.sh (existing download script — no changes needed)
- src-tauri/tauri.conf.json (beforeDevCommand and beforeBuildCommand reference bun run dev / bun run build)

## Relevant Docs
- docs/decisions/depotdownloader-sidecar.md (decision to bundle as sidecar)
- docs/specs/sidecar-setup.md (sidecar configuration spec)

## Related Issues
None — no related issues found.

## Scope
Add an `ensure-sidecar` script to package.json that checks if the DepotDownloader binary exists for the current platform and runs `scripts/download-sidecar.sh` if missing. Wire it into the `dev` and `build` scripts so that `bun run tauri dev` and `bun run tauri build` work out of the box without manual binary setup.

### What to do
1. Add an `ensure-sidecar` script to package.json that checks for the platform-specific binary using `rustc -vV` target triple detection
2. Prepend `bun run ensure-sidecar &&` to the existing `dev` and `build` scripts
3. Verify the script is a no-op when the binary already exists
4. Verify the script downloads when the binary is missing

### Constraints
- Must be cross-platform (the rustc target triple detection works on all platforms)
- Must be idempotent — no-op if binary already exists
- No version bump needed (build tooling only)
