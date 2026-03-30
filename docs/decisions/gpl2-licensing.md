---
title: "GPL-2.0 Licensing"
type: decision
tags: [licensing, gpl-2, depotdownloader, legal]
created: 2026-03-30
updated: 2026-03-30
---

# GPL-2.0 Licensing

## Context

Rewind bundles DepotDownloader as a sidecar binary. DepotDownloader is licensed under GPL-2.0.

## Decision

Rewind is licensed under GPL-2.0 to allow direct bundling of DepotDownloader.

## Rationale

- Bundling a GPL-2.0 binary as part of the distributed application means the combined work falls under GPL-2.0's copyleft requirements.
- Alternative approaches (e.g., treating DepotDownloader as a separate program the user installs independently) would degrade the user experience and add friction to the setup process.
- GPL-2.0 is acceptable for Rewind's goals -- it is a community tool, not a commercial product with proprietary interests.

## Implications

- All Rewind source code must be made available to recipients of the binary.
- Any modifications to DepotDownloader must also be shared under GPL-2.0.
- Contributors must be aware that their contributions fall under GPL-2.0.
- Third-party libraries used by Rewind must be GPL-2.0-compatible (MIT, Apache-2.0, BSD are all compatible).
