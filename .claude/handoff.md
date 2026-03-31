---
trigger: "Sidecar spawns a new process for every command, each requiring fresh Steam connection and auth. Even with saved session tokens, this causes repeated Steam Guard prompts. The sidecar should be a long-running daemon — one connection, one auth, many commands."
type: refactor
branch: refactor/persistent-sidecar
base-branch: main
created: 2026-03-31
---

## Related Files

### Sidecar (.NET)
- sidecar/SteamKitSidecar/Program.cs — entry point, CLI arg routing (becomes stdin command loop)
- sidecar/SteamKitSidecar/SteamSession.cs — Steam connection + auth (becomes long-lived, shared across commands)
- sidecar/SteamKitSidecar/JsonOutput.cs — NDJSON output helpers (add request_id correlation)
- sidecar/SteamKitSidecar/Commands/LoginCommand.cs — login handler
- sidecar/SteamKitSidecar/Commands/ListManifestsCommand.cs — manifest listing
- sidecar/SteamKitSidecar/Commands/ListDepotsCommand.cs — depot listing
- sidecar/SteamKitSidecar/Commands/GetManifestCommand.cs — manifest fetch
- sidecar/SteamKitSidecar/Commands/DownloadCommand.cs — file download

### Rust infrastructure
- src-tauri/src/infrastructure/sidecar.rs — spawn_sidecar (becomes manage_sidecar / send_command)
- src-tauri/src/infrastructure/depot_downloader.rs — per-command spawn + parse (becomes send + await response)

### Rust app layer
- src-tauri/src/lib.rs — IPC commands that call infrastructure (minimal changes — same API)

## Relevant Docs
- docs/specs/sidecar-setup.md — sidecar build and Tauri integration
- docs/specs/steamkit-sidecar.md — NDJSON protocol, command spec
- docs/specs/downgrade-pipeline.md — pipeline uses sidecar for manifests + download
- docs/domain/steamkit-sidecar.md — domain knowledge about Steam sidecar

## Related Issues
None — no related issues found.

## Scope

### Problem
Every sidecar command (login, list-manifests, list-depots, get-manifest, download) spawns a new .NET process. Each process:
1. Creates a new SteamClient
2. Connects to Steam servers (~1-2s)
3. Authenticates (saved session token or fresh credentials + Steam Guard)
4. Runs the single command
5. Disconnects and exits

This means:
- 5+ Steam connections per downgrade operation
- If saved session expires mid-flow, user gets Steam Guard prompt again
- Each connection has ~1-2s overhead
- Transient connection failures cause auth errors

### Solution: Long-running sidecar daemon

Convert the sidecar from a CLI-per-command model to a stdin/stdout daemon:

#### Sidecar changes (Program.cs)
- Instead of parsing CLI args and routing to a command handler, enter a **read loop** on stdin
- Read NDJSON commands from stdin, one per line
- Each command includes a `request_id` for response correlation
- Route to the appropriate handler (reuse existing command logic)
- Write NDJSON responses to stdout with the matching `request_id`
- The SteamSession is created once and shared across all commands

#### Command protocol (stdin → sidecar)
```json
{"request_id":"r1","command":"login","username":"user","password":"pass"}
{"request_id":"r2","command":"list-depots","app_id":3321460}
{"request_id":"r3","command":"list-manifests","app_id":3321460,"depot_id":3321461}
{"request_id":"r4","command":"get-manifest","app_id":3321460,"depot_id":3321461,"manifest_id":1234567890}
```

#### Response protocol (sidecar → stdout)
```json
{"request_id":"r1","type":"auth_success","session_file":"/path"}
{"request_id":"r2","type":"depot_list","depots":[...]}
{"request_id":"r3","type":"manifest_list","manifests":[...]}
{"request_id":"r1","type":"done","success":true}
```

Progress events (download) are streamed with the same request_id:
```json
{"request_id":"r4","type":"progress","percent":45.2,"bytes_downloaded":1024000,"bytes_total":2048000}
{"request_id":"r4","type":"done","success":true}
```

#### Rust infrastructure changes

**sidecar.rs** — New sidecar lifecycle management:
- `start_sidecar(app) -> SidecarHandle` — spawn once, store handle in Tauri managed state
- `SidecarHandle` holds stdin writer + stdout reader + child process
- `send_command(handle, command) -> Response` — write command to stdin, read correlated response(s)
- Automatic restart if the sidecar crashes

**depot_downloader.rs** — Refactor each function:
- Instead of `spawn_sidecar(app, args)`, call `send_command(handle, command)`
- Parse responses the same way (NDJSON), but now correlated by request_id
- The `login` function sends a login command instead of spawning a login process
- `list_manifests`, `list_depots`, etc. send their respective commands

#### Auth flow simplification
- On app startup, spawn the sidecar daemon
- When `set_credentials` or `resume_session` is called, send `login` command to the running sidecar
- All subsequent commands (list-manifests, list-depots, etc.) use the already-authenticated session
- No re-auth needed unless the sidecar crashes and restarts

### What stays the same
- Tauri IPC commands (frontend API unchanged)
- NDJSON message types (same payload shapes)
- Domain types and application layer
- Frontend code (no changes)
- Build/bundle process (same sidecar binary, just different invocation)
