---
title: "DepotDownloader as Tauri Sidecar"
type: decision
tags: [depotdownloader, sidecar, tauri, architecture, dependencies]
created: 2026-03-30
updated: 2026-03-30
---

# DepotDownloader as Tauri Sidecar

## Context

Rewind needs DepotDownloader to download game files from Steam's CDN. DepotDownloader is a .NET application, while Rewind's backend is Rust (Tauri).

Options considered:

1. **Require users to install DepotDownloader and .NET separately** -- poor UX, high support burden.
2. **Bundle DepotDownloader as a Tauri sidecar** -- self-contained binary, no external dependencies.
3. **Reimplement Steam download protocol in Rust** -- large effort, high risk, would eliminate the dependency entirely.

## Decision

Bundle DepotDownloader as a Tauri sidecar using its self-contained (ahead-of-time compiled) build.

## Rationale

- The self-contained build of DepotDownloader includes the .NET runtime, so users do not need to install .NET separately.
- Tauri's sidecar mechanism handles bundling, extraction, and path resolution for platform-specific binaries.
- Each platform binary is approximately 33 MB -- acceptable for a desktop application that downloads multi-GB game files.
- Subprocess interaction (spawn, stdin/stdout piping, cancellation) is well-supported in Rust via `tokio::process`.

## Consequences

- **Binary size**: Each platform build includes ~33 MB for the DepotDownloader sidecar on top of the Tauri app itself.
- **Progress parsing**: Rewind must parse DepotDownloader's stdout to extract download progress and relay it to the frontend.
- **Error handling**: Exit codes and stderr must be parsed and translated into user-friendly messages.
- **Updates**: When DepotDownloader releases a new version, Rewind must update the bundled binary and test compatibility.
- **Cancellation**: Long-running downloads must be cancellable. The Rust backend must handle subprocess termination cleanly.

## Alternatives Rejected

- **User-installed DepotDownloader**: Unacceptable UX friction. Users should not need to install .NET or manage separate tools.
- **Rust reimplementation of SteamKit2**: The Steam download protocol is complex (authentication, CDN selection, chunk downloading, decryption). This is a future consideration but far too risky for the MVP.
