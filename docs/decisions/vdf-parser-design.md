---
title: "VDF Parser Design"
type: decision
tags: [vdf, parser, nom, domain, architecture]
created: 2026-03-30
updated: 2026-03-30
---

# VDF Parser Design

## Context

Rewind needs to parse and serialize Valve Data Format (VDF) files -- specifically ACF app manifests -- as part of the domain layer. The parser must handle VDF's quirks (no commas, no colons, quoted keys, arbitrary nesting) and support round-trip fidelity for ACF patching.

## Decisions

### Parser approach: nom combinator library

We chose `nom` (v7) for parsing rather than hand-rolling a recursive descent parser or using `pest` (PEG grammar). Rationale:

- **nom** provides composable parser combinators that map naturally to VDF's grammar (quoted strings, brace-delimited maps, key-value pairs).
- It produces good error messages on parse failure without additional effort.
- It is a well-established crate with no transitive dependencies that would conflict with Tauri.
- **pest** was considered but rejected because VDF's grammar is simple enough that a PEG file would be unnecessary indirection.
- **Regex** was explicitly excluded by the handoff constraints.

### Data model: ordered Vec of pairs, not HashMap

The VDF tree uses `Vec<(String, VdfValue)>` (aliased as `VdfMap`) rather than `HashMap` or `BTreeMap`. Rationale:

- **Insertion order preservation** -- VDF files have a conventional field order. Preserving it improves round-trip fidelity and human readability of serialized output.
- **Duplicate key support** -- VDF technically allows duplicate keys in the same block. A `HashMap` would silently drop duplicates.
- **Trade-off**: Key lookup is O(n) instead of O(1). This is acceptable because VDF maps are small (typically fewer than 20 keys per block).

### String fields for numeric values

ACF fields like `appid`, `buildid`, and `manifest` are stored as `String` in the typed `AppState` struct, not as integers. Rationale:

- These values can be very large (manifest IDs exceed u64 range in some cases).
- The parser's job is to faithfully represent the data, not interpret it. Downstream code can parse to numeric types as needed.
- String storage guarantees round-trip fidelity -- no lossy conversion.

## Consequences

- Adding new VDF features (e.g., unquoted keys, `#include` directives) requires extending the nom parser, not changing the data model.
- The O(n) key lookup is a known performance trade-off. If profiling shows this is a bottleneck (unlikely for ACF files), a secondary index can be added without changing the public API.
