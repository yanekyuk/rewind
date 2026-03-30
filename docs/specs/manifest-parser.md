---
title: "Manifest Parser"
type: spec
tags: [manifest, parser, domain, depotdownloader, diffing]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Parse DepotDownloader's `-manifest-only` text output into structured Rust types. The input is a plain text file with a header section (depot metadata) followed by a fixed-width file table.

### Header parsing

Extract metadata from the header section:

- **Depot ID** from the first line: `Content Manifest for Depot <id>`
- **Manifest ID** and **date** from: `Manifest ID / date : <id> / <date>`
- **Total files**, **total chunks**, **total bytes on disk**, **total bytes compressed** from key-value lines with ` : ` separator

### File table parsing

After the blank line following the header and the column header line (`Size Chunks File SHA Flags Name`), parse each subsequent non-empty line into a file entry:

- **Size** (u64): byte size of the file
- **Chunks** (u32): number of download chunks
- **File SHA** (String): 40-character hex-encoded SHA-1 hash
- **Flags** (u32): file flags
- **Name** (String): relative file path

Columns are whitespace-separated. The Name field may contain spaces (consume the rest of the line after Flags).

### Public API

```rust
pub fn parse_manifest(input: &str) -> Result<DepotManifest, ManifestError>
```

## Constraints

- Domain layer only -- no filesystem I/O, no infrastructure imports
- Operates on `&str` input; caller handles file reading
- Use nom or hand-rolled parsing for the main structure (regex acceptable for individual field extraction within a line)
- Numeric fields properly typed: u64 for sizes/bytes, u32 for chunks/flags
- SHA hashes stored as String (hex-encoded, 40 chars)
- Must handle edge cases: empty manifests (header only, no file entries), single file, large manifests

## Acceptance Criteria

1. `parse_manifest` successfully parses the example from `docs/domain/depotdownloader.md`
2. Depot ID, manifest ID, date, total files, total chunks, total bytes on disk, total bytes compressed are all correctly extracted from the header
3. Each file entry has correctly typed size (u64), chunks (u32), sha (String, 40 hex chars), flags (u32), and name (String)
4. Empty manifest (header with zero files and no file table rows) parses successfully with an empty entries vec
5. Single-file manifest parses correctly
6. Invalid input produces descriptive `ManifestError` values
7. No filesystem I/O or infrastructure layer imports in the module
