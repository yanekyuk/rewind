---
trigger: "DepotDownloader sidecar setup — configure Tauri's external binary bundling for DepotDownloader, download self-contained binaries for each platform (Linux, macOS, Windows), set up tauri.conf.json externalBin config, and verify sidecar resolution at runtime."
type: chore
branch: chore/sidecar-setup
base-branch: main
created: 2026-03-30
version-bump: none
---

## Related Files
- src-tauri/tauri.conf.json (needs externalBin / bundle.externalBin config for sidecar)
- src-tauri/src/infrastructure/mod.rs (infrastructure layer — subprocess module will use the sidecar path)
- src-tauri/Cargo.toml (may need tauri-plugin-shell or similar for sidecar resolution)

## Relevant Docs
- docs/decisions/depotdownloader-sidecar.md (decision: bundle as self-contained ~33MB per-platform binary)
- docs/domain/depotdownloader.md (CLI interface, flags, authentication, manifest output format)
- docs/decisions/gpl2-licensing.md (GPL-2.0 — must include DepotDownloader license in bundle)
- docs/domain/platform-differences.md (platform-specific binary names)
- docs/decisions/layered-architecture.md (sidecar management belongs in infrastructure layer)

## Related Issues
None — no related issues found.

## Scope
Configure Tauri to bundle DepotDownloader as a sidecar binary so the app can spawn it as a subprocess at runtime.

### What to do

1. **Download DepotDownloader self-contained binaries**
   - Fetch the latest release from https://github.com/SteamRE/DepotDownloader/releases
   - Download the self-contained builds for each platform:
     - `DepotDownloader-linux-x64` (or the appropriate archive)
     - `DepotDownloader-osx-x64`
     - `DepotDownloader-windows-x64.exe`
   - Place them in `src-tauri/binaries/` following Tauri's sidecar naming convention:
     - `DepotDownloader-x86_64-unknown-linux-gnu`
     - `DepotDownloader-x86_64-apple-darwin`
     - `DepotDownloader-x86_64-pc-windows-msvc.exe`

2. **Configure tauri.conf.json**
   - Add `bundle.externalBin` (or the Tauri 2 equivalent) pointing to the sidecar binaries
   - Ensure the binary is included in the app bundle for all target platforms

3. **Add a Rust helper for sidecar resolution**
   - In the infrastructure layer, add a utility function that resolves the sidecar path using Tauri's API
   - This will be used later by the subprocess management module
   - Include a basic smoke test: resolve the path and verify the binary exists

4. **Add .gitignore entry for binaries**
   - The actual binaries (~33MB each) should NOT be committed to git
   - Add `src-tauri/binaries/` to .gitignore
   - Add a script or instructions for downloading the binaries (e.g., a `scripts/download-sidecar.sh`)

5. **Include DepotDownloader license**
   - Add the GPL-2.0 license text for DepotDownloader in the bundle (required by the license)

### Constraints
- Binaries must NOT be committed to git (too large, ~33MB each)
- Must work cross-platform — Tauri resolves the correct binary per target triple
- Use Tauri 2's sidecar/external binary API (check context7 for current docs)
- The sidecar is not invoked in this PR — just configured and resolvable
