---
title: "SteamKit Sidecar Setup"
type: spec
tags: [steamkit, sidecar, tauri, infrastructure, dotnet]
created: 2026-03-30
updated: 2026-03-30
---

# SteamKit Sidecar Setup

## Overview

The SteamKit sidecar is a .NET console application that wraps SteamKit2 functionality. It is bundled as a Tauri sidecar binary, built separately from the Rust backend, and resolved/spawned at runtime.

## Build Process

### Prerequisites

- .NET 9 SDK (for development and building from source)
- Target platforms: Linux (x86_64), macOS (x86_64), Windows (x86_64)

### Building the Sidecar

1. **Source directory**: `sidecar/SteamKitSidecar/`
2. **Build command**: `dotnet publish -c Release -o dist/{platform}` (for each platform)
3. **Self-contained**: The sidecar is published as a self-contained executable (no .NET runtime dependency for end users)
4. **Output binary names**:
   - Linux: `SteamKitSidecar-x86_64-unknown-linux-gnu`
   - macOS: `SteamKitSidecar-x86_64-apple-darwin`
   - Windows: `SteamKitSidecar-x86_64-pc-windows-msvc.exe`

### Build Script

A shell script `scripts/build-sidecar.sh` automates the process:

1. Detects the current platform or accepts platform argument
2. Runs `dotnet publish` with appropriate target framework and runtime identifier
3. Copies the binary to `src-tauri/binaries/` with the correct naming
4. Verifies the binary exists and is executable

Example usage:
```bash
./scripts/build-sidecar.sh linux
./scripts/build-sidecar.sh macos
./scripts/build-sidecar.sh windows
```

Or build all platforms:
```bash
./scripts/build-sidecar.sh all
```

## Tauri Integration

### Configuration

`tauri.conf.json` declares the sidecar in the `bundle.externalBin` array:

```json
{
  "bundle": {
    "externalBin": [
      "binaries/SteamKitSidecar-$TARGET_TRIPLE"
    ]
  }
}
```

Tauri substitutes `$TARGET_TRIPLE` with the runtime target triple at build time.

### Binary Placement

Platform-specific sidecar binaries go in `src-tauri/binaries/`:

```
src-tauri/
  binaries/
    SteamKitSidecar-x86_64-unknown-linux-gnu
    SteamKitSidecar-x86_64-apple-darwin
    SteamKitSidecar-x86_64-pc-windows-msvc.exe
```

These binaries are **not committed to git** (.gitignore entry exists).

### Sidecar Resolution (Rust)

The Rust infrastructure layer provides `spawn_sidecar()` in `src-tauri/src/infrastructure/sidecar.rs`:

```rust
pub fn spawn_sidecar(
    app: &AppHandle,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(Receiver<CommandEvent>, CommandChild), tauri_plugin_shell::Error>
```

This function:
1. Uses Tauri's shell plugin to resolve the platform-specific binary
2. Spawns the sidecar as a subprocess
3. Returns a receiver for stdout/stderr events and a process handle
4. Handles platform-specific differences (e.g., .exe on Windows)

The function includes a unit test verifying the sidecar name is correct.

## Development Workflow

### Initial Setup

1. Install .NET 9 SDK
2. Build the sidecar for your platform: `./scripts/build-sidecar.sh $(uname -s | tr '[:upper:]' '[:lower:]')`
3. Run Tauri dev: `bun run tauri dev`

### Making Changes to the Sidecar

1. Edit C# files in `sidecar/SteamKitSidecar/`
2. Rebuild: `./scripts/build-sidecar.sh`
3. Restart the Tauri dev server

The sidecar is stateless and does not cache state between invocations, so rebuilding and restarting is safe.

### Testing

**Rust tests** (subprocess interaction):
```bash
cd src-tauri
cargo test
```

**C# tests** (sidecar logic):
```bash
cd sidecar/SteamKitSidecar
dotnet test
```

## Deployment

### Pre-built Binaries

For production releases, pre-built sidecar binaries for all platforms are:
1. Built in CI/CD
2. Committed to the release (not to the git repo)
3. Downloaded by the Tauri build process or the user's build script

Users building from source must have .NET 9 SDK installed.

### Download Script

`scripts/download-sidecar.sh` (optional, for development):
- Downloads pre-built sidecar binaries from GitHub releases
- Places them in `src-tauri/binaries/` with correct names
- Marks them as executable

Usage:
```bash
./scripts/download-sidecar.sh
```

This is useful for developers who want to use pre-built binaries without installing the .NET SDK.

### Binary Size

Expected size per platform: 50-60 MB (self-contained .NET runtime + SteamKit2)

## Licensing

- **SteamKit2**: LGPL v2.1
- **Sidecar project**: GPL-2.0 (to match Rewind's overall licensing)

License file `sidecar/LICENSE` is included in the sidecar source and referenced in packaging.

## Constraints

- Must work cross-platform: Linux, macOS, Windows (x86_64 only for MVP)
- Sidecar must be self-contained (no .NET SDK requirement for end users)
- All communication via newline-delimited JSON on stdout
- Process must be cancellable via SIGTERM/TerminateProcess
- Must handle Steam rate limits and session expiration gracefully

## Acceptance Criteria

- [x] `sidecar/SteamKitSidecar/` compiles with `dotnet publish` for all platforms
- [x] `scripts/build-sidecar.sh` builds and places binaries correctly
- [x] `tauri.conf.json` has `bundle.externalBin` entry for SteamKit sidecar
- [x] `src-tauri/src/infrastructure/sidecar.rs` has `spawn_sidecar()` function
- [x] Unit test verifies sidecar name matches configured name
- [x] `src-tauri/binaries/` is in `.gitignore`
- [x] Sidecar outputs newline-delimited JSON (see [domain/steamkit-sidecar.md](../domain/steamkit-sidecar.md))
- [x] Rust can receive and parse JSON messages from sidecar (integration layer)
- [x] Process can be spawned and terminated cleanly from Rust tests
