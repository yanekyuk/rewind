---
title: "DepotDownloader Sidecar Setup"
type: spec
tags: [depotdownloader, sidecar, tauri, infrastructure]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Configure Tauri to bundle DepotDownloader as a sidecar binary so the app can resolve and spawn it as a subprocess at runtime.

1. **Tauri configuration** -- `tauri.conf.json` declares DepotDownloader as an external binary via the `bundle.externalBin` array. Tauri resolves the correct platform-specific binary at runtime based on the current target triple.

2. **Binary placement** -- Platform-specific DepotDownloader binaries are placed in `src-tauri/binaries/` following Tauri's naming convention: `{name}-{target_triple}[.exe]`. The binaries are not committed to git.

3. **Download script** -- A shell script (`scripts/download-sidecar.sh`) fetches the latest DepotDownloader release from GitHub and places the binaries in the correct location with the correct names.

4. **Sidecar resolution helper** -- A Rust function in the infrastructure layer resolves the sidecar binary path using Tauri's shell plugin API. This function will be used by the future subprocess management module.

5. **GPL-2.0 license inclusion** -- The DepotDownloader GPL-2.0 license text is included in the Tauri resource bundle to satisfy licensing requirements.

## Constraints

- Binaries must not be committed to git (each is approximately 33 MB).
- Must work cross-platform: Linux (x86_64-unknown-linux-gnu), macOS (x86_64-apple-darwin), Windows (x86_64-pc-windows-msvc).
- The sidecar is not invoked in this change -- only configured and resolvable.
- Sidecar resolution code belongs in the infrastructure layer per the layered architecture decision.
- Must use Tauri 2's sidecar/shell plugin API.

## Acceptance Criteria

- [ ] `tauri.conf.json` contains `bundle.externalBin` entry for DepotDownloader.
- [ ] `src-tauri/binaries/` is in `.gitignore`.
- [ ] `scripts/download-sidecar.sh` downloads and places binaries correctly.
- [ ] A Rust function in infrastructure layer resolves the sidecar path using Tauri's API.
- [ ] A unit test verifies the sidecar command can be constructed.
- [ ] DepotDownloader's GPL-2.0 license is included as a Tauri resource.
- [ ] `Cargo.toml` includes `tauri-plugin-shell` dependency.
