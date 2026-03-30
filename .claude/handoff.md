---
trigger: "VDF/ACF parser — implement a dedicated parser for Valve Data Format (VDF) files used by Steam, including ACF app manifest files. This is the domain layer foundation that everything else depends on."
type: feat
branch: feat/vdf-parser
base-branch: main
created: 2026-03-30
version-bump: minor
---

## Related Files
- src-tauri/src/lib.rs (backend entry point — Tauri commands will eventually call the parser)
- src-tauri/Cargo.toml (dependencies — may need nom or similar parsing crate)

## Relevant Docs
- docs/domain/steam-internals.md (VDF/ACF format specification, field descriptions, example data)
- docs/domain/downgrade-process.md (how parsed ACF data feeds into the downgrade workflow)
- docs/decisions/layered-architecture.md (parser belongs in domain layer — no I/O, no infrastructure imports)

## Related Issues
None — no related issues found.

## Scope
Implement a VDF/ACF parser in Rust as part of the domain layer. This is the foundational module that all subsequent features depend on.

### What to build
- A VDF text parser that handles Valve Data Format: nested key-value pairs using braces and quotes
- Support for parsing ACF app manifest files (appmanifest_<appid>.acf) into strongly-typed Rust structs
- Typed structs for AppState including: appid, name, buildid, installdir, StateFlags, InstalledDepots (map of depot ID to manifest + size), TargetBuildID, BytesToDownload
- A serializer/writer that can output modified VDF back to text (needed later for ACF patching in Step 8)
- Unit tests with real-world ACF examples

### Constraints
- Domain layer only — no filesystem I/O, no Tauri imports, no infrastructure dependencies
- Must handle VDF quirks: no commas, no colons, quoted and potentially unquoted keys, arbitrary nesting depth
- Do not use regex for structured parsing — use a proper parser (nom, pest, or hand-rolled recursive descent)
- Must be cross-platform (no OS-specific code)
- Round-trip fidelity: parse then serialize should preserve the original structure (whitespace normalization is acceptable)
