# SteamKit Sidecar Testing Guide

This document describes how to test all four phases of the SteamKit sidecar implementation.

## Build Instructions

```bash
dotnet build SteamKitSidecar.csproj -c Release
# Output: bin/Release/net9.0/SteamKitSidecar
```

## Phase 1: Authentication Flow

### Command: login

Authenticates with Steam and saves the session token for reuse.

```bash
./SteamKitSidecar login --username <steam_user> --password <steam_pass> [--guard-code <2fa_code>]
```

**Output (NDJSON):**
```json
{"type":"log","level":"info","message":"Connecting to Steam..."}
{"type":"log","level":"info","message":"Connected to Steam"}
{"type":"auth_success","session_file":"/path/to/session.json"}
{"type":"done","success":true}
```

**With Steam Guard (email):**
```bash
./SteamKitSidecar login --username user --password pass
# Waits for: {"type":"guard_prompt","method":"email","hint":"example@email.com"}
# User enters code on stdin: 123456
# Response: auth_success JSON
```

**With Steam Guard (mobile authenticator):**
```bash
./SteamKitSidecar login --username user --password pass
# Waits for: {"type":"guard_prompt","method":"device","hint":null}
# User enters code on stdin: 123456
# Response: auth_success JSON
```

**With preset guard code:**
```bash
./SteamKitSidecar login --username user --password pass --guard-code 123456
# Automatically uses guard code without prompting
```

### Session Persistence

Sessions are saved to `%LOCALAPPDATA%\rewind\sessions\<username>.json` on Windows,
and `~/.local/share/rewind/sessions/<username>.json` on Linux/macOS.

Next login with same username automatically reuses saved session, avoiding 2FA prompt.

---

## Phase 2: Manifest Listing

### Command: list-manifests

Lists available manifest IDs for a depot.

```bash
./SteamKitSidecar list-manifests --username <user> --password <pass> --app 570 --depot 373307
```

**Output (NDJSON):**
```json
{"type":"log","level":"info","message":"Requesting manifest history for app 570, depot 373307..."}
{"type":"manifest_list","manifests":[
  {"id":"1234567890","date":""},
  {"id":"9876543210","date":""}
]}
{"type":"done","success":true}
```

### Parameters

- `--username`: Steam account username
- `--password`: Steam account password
- `--app`: Steam app ID (e.g., 570 for Dota 2)
- `--depot`: Depot ID within the app
- `--guard-code`: Optional 2FA code (avoids prompt)

### Notes

- Current implementation lists the active public manifest via PICS data
- Full historical manifest enumeration requires additional Steam API endpoints
- Dates may be empty for manifests without timestamp metadata

---

## Phase 3: Manifest Fetching

### Command: get-manifest

Downloads and parses a specific manifest by ID.

```bash
./SteamKitSidecar get-manifest --username <user> --password <pass> --app 570 --depot 373307 --manifest 1234567890
```

**Output (NDJSON):**
```json
{"type":"log","level":"info","message":"Fetching manifest 1234567890 for depot 373307..."}
{"type":"manifest","depot_id":373307,"manifest_id":"1234567890","total_files":150,"total_chunks":1200,"total_bytes_on_disk":1073741824,"total_bytes_compressed":536870912,"date":"2024-01-15T10:30:00Z","files":[
  {"name":"game.exe","sha":"abc123def456...","size":50331648,"chunks":30,"flags":0},
  {"name":"game.dll","sha":"789abc012def...","size":10485760,"chunks":10,"flags":0},
  {"name":"assets/texture.dds","sha":"def456789abc...","size":268435456,"chunks":200,"flags":0}
]}
{"type":"done","success":true}
```

### Parameters

- `--username`: Steam account username
- `--password`: Steam account password
- `--app`: Steam app ID
- `--depot`: Depot ID
- `--manifest`: Manifest ID to fetch
- `--guard-code`: Optional 2FA code

### Output Fields

- `total_files`: Number of files in the manifest
- `total_chunks`: Total number of chunks across all files
- `total_bytes_on_disk`: Uncompressed size of all files
- `total_bytes_compressed`: Compressed size of all chunks
- `date`: ISO 8601 manifest creation timestamp
- `files[]`: Array of file entries with:
  - `name`: File path within depot
  - `sha`: SHA-1 hash of uncompressed file
  - `size`: File size in bytes
  - `chunks`: Number of chunks for this file
  - `flags`: EDepotFileFlag bitmask (0=regular file, 16=directory, etc.)

---

## Phase 4: File Downloading

### Command: download

Downloads all files from a manifest or a filtered subset.

```bash
./SteamKitSidecar download --username <user> --password <pass> --app 570 --depot 373307 --manifest 1234567890 --dir ./output
```

**Output (NDJSON):**
```json
{"type":"log","level":"info","message":"Downloading depot 373307 manifest 1234567890..."}
{"type":"log","level":"info","message":"Downloading 150 files (1073741824 bytes)..."}
{"type":"progress","percent":0.1,"bytes_downloaded":10485760,"bytes_total":1073741824}
{"type":"progress","percent":0.2,"bytes_downloaded":20971520,"bytes_total":1073741824}
{"type":"progress","percent":50.0,"bytes_downloaded":536870912,"bytes_total":1073741824}
{"type":"log","level":"info","message":"Download complete: 150/150 files"}
{"type":"done","success":true}
```

### Parameters

- `--username`: Steam account username
- `--password`: Steam account password
- `--app`: Steam app ID
- `--depot`: Depot ID
- `--manifest`: Manifest ID to download from
- `--dir`: Output directory for downloaded files
- `--filelist`: Optional path to file with newline-separated file names to download

### Filelist Format

Create a text file with one file path per line:

```
game.exe
game.dll
assets/texture.dds
```

Then download only those files:

```bash
./SteamKitSidecar download \
  --username user \
  --password pass \
  --app 570 \
  --depot 373307 \
  --manifest 1234567890 \
  --dir ./output \
  --filelist ./filelist.txt
```

### Features

- Chunk-by-chunk downloading with automatic decompression
- Progress reported after each chunk
- Directory creation as needed
- Automatic file and directory flag handling
- Atomic file writing (files not overwritten mid-operation)

---

## Error Handling

All commands output errors as JSON to stderr:

```json
{"type":"error","code":"DEPOT_KEY_ERROR","message":"Failed to get depot key: InvalidPassword"}
{"type":"done","success":false}
```

### Common Error Codes

- `USAGE`: No command or invalid usage
- `UNKNOWN_COMMAND`: Unknown command name
- `UNHANDLED_ERROR`: Unexpected exception
- `CONNECTION_FAILED`: Failed to connect to Steam
- `AUTH_FAILED`: Authentication failed
- `AUTH_ERROR`: Authentication exception
- `MANIFEST_LIST_ERROR`: Error listing manifests
- `DEPOT_KEY_ERROR`: Failed to get depot decryption key
- `CDN_ERROR`: No CDN servers available
- `MANIFEST_ERROR`: Error fetching manifest
- `DOWNLOAD_ERROR`: Error downloading files

---

## Integration with Rust Backend

The Rust infrastructure layer reads NDJSON output from the sidecar and:

1. Deserializes each line as JSON
2. Routes by `"type"` field
3. Extracts relevant data for manifest/file operations
4. Handles `"guard_prompt"` by prompting user and writing response to sidecar's stdin
5. Tracks progress via `"progress"` events
6. Verifies `"done"` message and exit code

Example Rust code pattern:

```rust
let output = Command::new("./SteamKitSidecar")
    .arg("login")
    .arg("--username").arg(username)
    .arg("--password").arg(password)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped())
    .spawn()?;

for line in BufReader::new(output.stdout).lines() {
    let json: serde_json::Value = serde_json::from_str(&line?)?;
    match json["type"].as_str() {
        Some("auth_success") => { /* handle */ }
        Some("guard_prompt") => { /* prompt user, write to stdin */ }
        Some("error") => { /* handle error */ }
        Some("done") => break,
        _ => {}
    }
}
```
