---
title: "SteamKit2 Migration from DepotDownloader"
type: decision
tags: [steamkit, sidecar, tauri, architecture, dependencies, licensing]
created: 2026-03-30
updated: 2026-03-30
---

# SteamKit2 Migration from DepotDownloader

## Context

Originally, Rewind used [DepotDownloader](https://github.com/SteamRE/DepotDownloader) to download game files from Steam's CDN. However, the approach had limitations:

- DepotDownloader is a command-line tool with fragile text output parsing
- No native 2FA support (required stdin relay workaround)
- Cannot enumerate available manifests for a depot (user must provide manifest IDs manually)
- GPL-2.0 licensing adds compliance burden
- Limited visibility into Steam operations

We evaluated two options:

1. **Continue with DepotDownloader**: Accept text parsing complexity and manifest enumeration gap.
2. **Migrate to SteamKit2 sidecar**: Use the .NET library directly via a custom sidecar for direct control over Steam operations.

## Decision

Migrate to [SteamKit2](https://github.com/DoctorMcKay/SteamKit) as a native Tauri sidecar written in C#/.NET. The sidecar handles all Steam operations and communicates with Rewind via newline-delimited JSON (NDJSON).

## Rationale

**Direct Control**: SteamKit2 is the .NET library DepotDownloader is built on. Using it directly eliminates:
- Fragile stdout text parsing (now structured JSON)
- stdin relay workarounds for 2FA (native support in sidecar)

**Manifest Enumeration**: The sidecar implements `list-manifests` command to enumerate all available manifests for a depot. This was a critical gap in DepotDownloader and is now addressed.

**Session Persistence**: The sidecar can cache authenticated sessions across commands, reducing login overhead.

**Structured Communication**: NDJSON protocol with typed messages (`type` field) makes integration more reliable and maintainable.

**Licensing Improvement**: SteamKit2 is LGPL v2.1 (more permissive than DepotDownloader's GPL-2.0). While Rewind will remain GPL-2.0 for other reasons, using LGPL dependencies is cleaner legally.

## Implementation

The sidecar is a .NET console application that:
- Receives commands: `login`, `list-manifests`, `get-manifest`, `download`
- Outputs newline-delimited JSON on stdout
- Handles 2FA prompts natively (returns `guard_prompt` messages, awaits responses)
- Manages session state (authentication tokens, cached manifests)

See [docs/domain/steamkit-sidecar.md](../domain/steamkit-sidecar.md) and [docs/specs/sidecar-setup.md](../specs/sidecar-setup.md).

## Consequences

- **Sidecar maintenance**: Must maintain C#/.NET sidecar codebase alongside Rust backend.
- **Session management**: Rust layer must handle guard prompt exchanges and session file persistence.
- **Binary size**: SteamKit2 is larger than DepotDownloader; sidecar binary ~50-60 MB per platform.
- **Deployment**: Users must have .NET 9 SDK to build from source (pre-built binaries do not require .NET).
- **Testing**: Sidecar logic requires separate test suite in C#.

## Alternatives Rejected

- **Continue with DepotDownloader**: Text parsing is fragile; manifest enumeration limitation cannot be worked around.
- **Pure Rust reimplementation of Steam protocol**: Too large a scope for MVP; high risk of protocol changes breaking the implementation.
- **Direct SteamKit2 Rust bindings**: Binding .NET library to Rust adds complexity; sidecar approach is cleaner and more maintainable.
