---
title: "GPL-2.0 Licensing"
type: decision
tags: [licensing, gpl-2, depotdownloader, legal]
created: 2026-03-30
updated: 2026-03-30
---

# GPL-2.0 Licensing

## Context

Rewind uses a custom SteamKit2-based sidecar for Steam interactions. The sidecar is written in C# and wrapped as a Tauri sidecar binary. SteamKit2 is licensed under LGPL-2.1, which is more permissive than GPL-2.0.

Originally, Rewind used DepotDownloader (GPL-2.0), which forced the entire project to adopt GPL-2.0. The migration to SteamKit2 (LGPL-2.1) provides more licensing flexibility.

## Decision

Rewind is licensed under GPL-2.0 to maintain consistency and encourage community contributions. SteamKit2's LGPL-2.1 is GPL-compatible, so the choice is compatible with the underlying dependency.

## Rationale

- GPL-2.0 remains the appropriate license for Rewind's goals -- it is a community tool, not a commercial product with proprietary interests.
- LGPL-2.1 (SteamKit2) is compatible with GPL-2.0; using an LGPL dependency in a GPL-2.0 project is a standard and legal practice.
- The sidecar (which depends on SteamKit2) is proprietary to Rewind and licensed under GPL-2.0 to match the main project.
- This decision maintains contributor expectations and the project's open-source ethos while allowing use of better-licensed dependencies.

## Implications

- All Rewind source code must be made available to recipients of the binary.
- The custom SteamKit2 sidecar is licensed under GPL-2.0 to match the main project.
- Contributors must be aware that their contributions fall under GPL-2.0.
- Third-party libraries used by Rewind must be GPL-2.0-compatible. LGPL-2.1 (SteamKit2) and MIT/Apache-2.0/BSD are all compatible with GPL-2.0.

## License Attribution

- **Rewind**: GPL-2.0
- **Sidecar**: GPL-2.0 (custom application wrapping SteamKit2)
- **SteamKit2**: LGPL-2.1 (dependency of sidecar)
