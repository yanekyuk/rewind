---
title: "VDF/ACF Parser"
type: spec
tags: [vdf, acf, parser, domain, steam]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

A pure Rust module in the domain layer that parses Valve Data Format (VDF) text into an in-memory tree, serializes the tree back to VDF text, and provides strongly-typed access to ACF app manifest data.

### Generic VDF Parser

- Parses VDF text into a tree of key-value pairs where values are either strings or nested key-value maps.
- Handles quoted keys and values (the primary format).
- Handles arbitrary nesting depth using brace-delimited blocks.
- Ignores VDF comments (lines starting with `//`).
- Uses a proper parser (nom, pest, or hand-rolled recursive descent) -- no regex for structural parsing.

### Generic VDF Serializer

- Converts the in-memory VDF tree back to text format.
- Produces valid VDF output with proper quoting, indentation, and brace nesting.
- Round-trip fidelity: `parse(serialize(parse(input)))` must equal `parse(input)` (structural equivalence; whitespace normalization is acceptable).

### Typed ACF Access

- Provides an `AppState` struct with strongly-typed fields extracted from parsed VDF data:
  - `appid: String`
  - `name: String`
  - `buildid: String`
  - `installdir: String`
  - `state_flags: String`
  - `installed_depots: HashMap<String, InstalledDepot>` (depot ID to manifest + size)
  - `target_build_id: Option<String>`
  - `bytes_to_download: Option<String>`
- `InstalledDepot` struct with `manifest: String` and `size: String`.
- Conversion from generic VDF tree to `AppState` with error handling for missing/malformed fields.
- Conversion from `AppState` back to VDF tree for serialization (needed for ACF patching).

## Constraints

- Domain layer only: no filesystem I/O, no Tauri imports, no infrastructure dependencies.
- No OS-specific code.
- No regex for structural parsing.
- All parsing operates on `&str` input -- the caller (infrastructure layer) is responsible for file I/O.
- Strings are used for numeric fields (appid, buildid, etc.) to preserve exact values and avoid lossy conversion.

## Acceptance Criteria

1. Parses the example ACF from `docs/domain/steam-internals.md` into an `AppState` struct with all fields correctly populated.
2. Serializes a parsed VDF tree back to valid VDF text that re-parses to the same tree.
3. Handles nested VDF structures of at least 5 levels deep.
4. Returns descriptive errors for malformed input (unclosed braces, unclosed quotes).
5. Handles ACF files with optional fields (`TargetBuildID`, `BytesToDownload`) missing.
6. Handles ACF files with multiple depots in `InstalledDepots`.
7. All tests pass with `cargo test` from the `src-tauri/` directory.
