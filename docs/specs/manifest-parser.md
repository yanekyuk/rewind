---
title: "Manifest Parser"
type: spec
tags: [manifest, parser, domain, steamkit, sidecar, json, diffing]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Deserialize and validate JSON manifest messages received from the SteamKit sidecar into structured Rust types. The sidecar outputs manifest metadata as newline-delimited JSON with a `type: "manifest"` message.

### Input Format

The sidecar outputs a JSON message:

```json
{
  "type": "manifest",
  "depot_id": 3321461,
  "manifest_id": "7446650175280810671",
  "total_files": 257,
  "total_chunks": 130874,
  "total_bytes_on_disk": 133352312992,
  "total_bytes_compressed": 100116131120,
  "date": "2026-03-22 16:01:45",
  "files": [
    {
      "name": "0000/0.pamt",
      "sha": "8a11847b3e22b2fb909b57787ed94d1bb139bcb2",
      "size": 6740755,
      "chunks": 7,
      "flags": 0
    }
  ]
}
```

### Deserialization

Parse the JSON message and deserialize into a domain type (e.g., `DepotManifest` struct) with:

- **depot_id** (u32): Depot identifier
- **manifest_id** (String or u64): Manifest identifier
- **total_files** (u64): Number of files in the manifest
- **total_chunks** (u64): Total number of download chunks
- **total_bytes_on_disk** (u64): Uncompressed total size
- **total_bytes_compressed** (u64): Compressed total size
- **date** (String): Build/manifest creation date
- **files** (Vec): List of file entries, each with name, sha (40-char hex), size (u64), chunks (u32), flags (u32)

### Public API

```rust
pub fn deserialize_manifest(json: &str) -> Result<DepotManifest, ManifestError>
```

## Constraints

- Domain layer only -- no filesystem I/O, no infrastructure imports
- Operates on `&str` input (one JSON line); caller handles reading from sidecar stdout
- Use `serde_json` for JSON deserialization
- Numeric fields properly typed: u64 for sizes/bytes, u32 for chunks/flags
- SHA hashes stored as String (hex-encoded, 40 chars)
- Must handle edge cases: empty manifests (zero files), single file, large manifests

## Acceptance Criteria

1. Successfully deserialize a manifest JSON message from the sidecar
2. All metadata fields (depot_id, manifest_id, date, total_files, total_chunks, total_bytes_on_disk, total_bytes_compressed) are correctly extracted
3. Each file entry has correctly typed size (u64), chunks (u32), sha (String, 40 hex chars), flags (u32), and name (String)
4. Empty manifest (zero files array) deserializes successfully
5. Single-file manifest deserializes correctly
6. Invalid JSON produces descriptive `ManifestError` values
7. Missing required fields produce descriptive deserialization errors
8. No filesystem I/O or infrastructure layer imports in the module
