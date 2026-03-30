---
title: "SteamKit Sidecar"
type: domain
tags: [steamkit, sidecar, steam, authentication, manifests, json]
created: 2026-03-30
updated: 2026-03-30
---

# SteamKit Sidecar

The SteamKit sidecar is a .NET console application that provides structured access to SteamKit2 functionality. The Rust backend spawns it as a subprocess and communicates via newline-delimited JSON (NDJSON) on stdout.

## Architecture

The sidecar is a command-line tool that:

1. Receives commands as CLI arguments
2. Authenticates with Steam using SteamKit2
3. Performs Steam operations (list manifests, fetch manifest metadata, download files)
4. Outputs progress and results as JSON lines to stdout
5. Outputs errors and logs to stdout/stderr

Each command is a separate invocation. The Rust backend handles session persistence by saving the authenticated session file and passing it to subsequent commands.

## Commands

### `login`

Authenticate with Steam and save the session.

**Arguments:**
- `--username <user>` — Steam username
- `--password <pass>` — Steam password
- `--guard-code <code>` (optional) — Steam Guard code if 2FA is required

**Output:**

```json
{"type":"auth_success","session_file":"/path/to/session.json"}
```

Or, if Steam Guard is required:

```json
{"type":"guard_prompt","method":"email","hint":"guard_code@email.com"}
```

The Rust layer must then:
1. Prompt the user for the code
2. Rerun login with the `--guard-code` argument
3. Retry authentication

**Exit code:** 0 on success, 1 on failure.

### `list-manifests`

Enumerate all available manifests for a depot.

**Arguments:**
- `--username <user>` — Steam username
- `--password <pass>` — Steam password
- `--app <id>` — App ID
- `--depot <id>` — Depot ID
- `--guard-code <code>` (optional) — Steam Guard code if needed

**Output:**

```json
{"type":"manifest_list","manifests":[
  {"id":"7446650175280810671","date":"2026-03-22 16:01:45"},
  {"id":"7446500175280810670","date":"2026-03-15 14:30:20"}
]}
```

Returns a list of available manifests for the depot, ordered newest first. Includes manifest ID and build date for each.

**Exit code:** 0 on success, 1 on failure.

### `get-manifest`

Fetch metadata for a specific manifest (file listing, sizes, hashes).

**Arguments:**
- `--username <user>` — Steam username
- `--password <pass>` — Steam password
- `--app <id>` — App ID
- `--depot <id>` — Depot ID
- `--manifest <id>` — Manifest ID
- `--guard-code <code>` (optional) — Steam Guard code if needed

**Output:**

```json
{"type":"manifest","depot_id":3321461,"manifest_id":"7446650175280810671","total_files":257,"total_chunks":130874,"total_bytes_on_disk":133352312992,"total_bytes_compressed":100116131120,"date":"2026-03-22 16:01:45","files":[
  {"name":"0000/0.pamt","sha":"8a11847b3e22b2fb909b57787ed94d1bb139bcb2","size":6740755,"chunks":7,"flags":0},
  {"name":"0000/0.paz","sha":"3e6800918fef5f8880cf601e5b60bff031465e60","size":912261088,"chunks":896,"flags":0}
]}
```

Returns complete manifest metadata including:
- Manifest ID and date
- Total file count, chunk count, and size information
- File entries (name, SHA hash, uncompressed size, chunk count, flags)

Used for computing diffs between versions.

**Exit code:** 0 on success, 1 on failure.

### `download`

Download files from a manifest.

**Arguments:**
- `--username <user>` — Steam username
- `--password <pass>` — Steam password
- `--app <id>` — App ID
- `--depot <id>` — Depot ID
- `--manifest <id>` — Manifest ID
- `--dir <path>` — Output directory for downloaded files
- `--filelist <path>` (optional) — File with newline-separated file names to download; if omitted, downloads entire manifest

**Output (streaming):**

```json
{"type":"progress","percent":45.5,"bytes_downloaded":4500000000,"bytes_total":9900000000}
```

Progress updates are sent periodically during download. When download completes:

```json
{"type":"done","success":true}
```

Or on error:

```json
{"type":"error","code":"DOWNLOAD_FAILED","message":"Failed to download chunk..."}
{"type":"done","success":false}
```

**Exit code:** 0 on success, 1 on failure.

## JSON Output Formats

### Message Types

All output is newline-delimited JSON where the first field is `type`:

| Type | Purpose | Fields |
|------|---------|--------|
| `log` | Info/warning logs | `type`, `level` (info/warn), `message` |
| `guard_prompt` | 2FA required | `type`, `method` (email/mobile), `hint` |
| `auth_success` | Login completed | `type`, `session_file` |
| `manifest_list` | Available manifests | `type`, `manifests` (array of {id, date}) |
| `manifest` | Manifest metadata | `type`, `depot_id`, `manifest_id`, `total_files`, `total_chunks`, `total_bytes_on_disk`, `total_bytes_compressed`, `date`, `files` |
| `progress` | Download progress | `type`, `percent`, `bytes_downloaded`, `bytes_total` |
| `error` | Error message | `type`, `code`, `message` |
| `done` | Command complete | `type`, `success` |

### Field Name Convention

All fields use `snake_case` naming (e.g., `total_files`, `bytes_downloaded`). This is consistent with Rust conventions and makes JSON more readable.

## Session Persistence

The sidecar does not maintain long-lived state between invocations. Each command is a fresh process.

For commands that require authentication, the Rust backend must:

1. Call `login` once to create a session
2. Pass the username/password to subsequent commands
3. If a `guard_prompt` is received, prompt the user and retry the command with the guard code

The sidecar uses SteamKit2's built-in session caching to minimize server calls within a single command invocation.

## Error Handling

Errors are reported via:

1. **JSON error messages** on stdout (if parsing error before main operation)
2. **JSON error messages** on stderr (if error during operation)
3. **Exit code 1** (always)

The `code` field contains a machine-readable error code (e.g., `INVALID_CREDENTIALS`, `ACCOUNT_DISABLED`, `DOWNLOAD_FAILED`). The `message` field is human-readable and includes details.

Example:

```json
{"type":"error","code":"INVALID_CREDENTIALS","message":"Username or password is incorrect"}
```

## 2FA/Steam Guard Handling

When Steam requires 2FA (via email code or mobile authenticator), the sidecar outputs:

```json
{"type":"guard_prompt","method":"email","hint":"user@example.com"}
```

Or for mobile:

```json
{"type":"guard_prompt","method":"mobile"}
```

The Rust backend must:

1. Detect the `guard_prompt` message
2. Prompt the user for the code
3. Rerun the command with `--guard-code <code>`

The sidecar will cache the authenticated session and subsequent commands will not require re-authentication if called within the session timeout window (typically ~24 hours).

## Performance Characteristics

- **Login**: 1-2 seconds (depends on network, Steam's servers)
- **Manifest list**: 2-5 seconds (one request per manifest version)
- **Get manifest**: 1-3 seconds (one request, downloads manifest metadata only)
- **Download**: Depends on file count and network speed; progress updates every 100-200 chunks or ~5 seconds

All operations are cancellable by terminating the process (SIGTERM on Unix, TerminateProcess on Windows).

## Licensing

SteamKit2 is licensed under **LGPL v2.1**. The sidecar project itself is GPL-2.0 to match Rewind's overall licensing strategy.

See [decisions/gpl2-licensing](../decisions/gpl2-licensing.md) for licensing implications.
