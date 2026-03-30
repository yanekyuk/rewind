# SteamKit Sidecar

A .NET 9.0 console application that provides Steam integration for the Rewind game downgrader using SteamKit2.

## Features

- **Phase 1: Authentication** - Steam login with 2FA support (email + mobile authenticator)
- **Phase 2: Manifest Listing** - List available manifests for a Steam depot
- **Phase 3: Manifest Fetching** - Download and parse manifest metadata
- **Phase 4: File Downloading** - Download individual files from depot manifests

## Architecture

- **SteamSession**: Manages Steam client connection, authentication callbacks, and session persistence
- **Commands**: Modular command handlers (Login, ListManifests, GetManifest, Download)
- **JsonOutput**: NDJSON (newline-delimited JSON) serialization for stdout/stderr
- **JsonAuthenticator**: Handles Steam Guard 2FA via stdin/stdout prompts

## Building

```bash
dotnet build SteamKitSidecar.csproj -c Release
# Output: bin/Release/net9.0/SteamKitSidecar
```

## Commands

### login
```bash
./SteamKitSidecar login --username <user> --password <pass> [--guard-code <code>]
```
Authenticates with Steam and persists session token for reuse.

### list-manifests
```bash
./SteamKitSidecar list-manifests --username <user> --password <pass> --app <id> --depot <id>
```
Lists available manifest IDs for a depot.

### get-manifest
```bash
./SteamKitSidecar get-manifest --username <user> --password <pass> --app <id> --depot <id> --manifest <id>
```
Downloads and parses a manifest, returning file metadata.

### download
```bash
./SteamKitSidecar download --username <user> --password <pass> --app <id> --depot <id> --manifest <id> --dir <path> [--filelist <path>]
```
Downloads files from a manifest to disk, optionally filtered by a filelist.

## Output Format

All output is NDJSON (newline-delimited JSON) to enable streaming and progressive parsing.

**Stdout** contains:
- Info logs: `{"type":"log","level":"info","message":"..."}`
- Guard prompts: `{"type":"guard_prompt","method":"email|device|device_confirm","hint":"..."}`
- Success messages: `{"type":"auth_success","session_file":"..."}`, `{"type":"manifest_list",...}`, etc.
- Progress events: `{"type":"progress","percent":50.0,"bytes_downloaded":...,"bytes_total":...}`
- Completion: `{"type":"done","success":true|false}`

**Stderr** contains:
- Error messages: `{"type":"error","code":"ERROR_CODE","message":"..."}`

## Session Persistence

Sessions are saved to platform-specific cache directory:
- Windows: `%LOCALAPPDATA%\rewind\sessions\<username>.json`
- Linux: `~/.local/share/rewind/sessions/<username>.json`
- macOS: `~/Library/Application Support/rewind/sessions/<username>.json`

Sessions are automatically reused on subsequent logins with the same username, avoiding 2FA prompts.

## Dependencies

- **SteamKit2 3.4.0**: Steam network protocol and CDN access
- **.NET 9.0**: Runtime environment

## Integration with Rust Backend

The Rust infrastructure layer spawns this sidecar as a subprocess and communicates via:
- **stdin**: Guard code responses (when prompted)
- **stdout**: NDJSON output stream
- **stderr**: Error messages
- **exit code**: 0 for success, 1 for failure

## Development

### File Structure
```
SteamKitSidecar/
├── Program.cs              # Entry point and command routing
├── JsonOutput.cs           # NDJSON serialization utilities
├── SteamSession.cs         # Steam client management
└── Commands/
    ├── LoginCommand.cs     # Phase 1: Authentication
    ├── ListManifestsCommand.cs   # Phase 2: Manifest listing
    ├── GetManifestCommand.cs     # Phase 3: Manifest fetching
    └── DownloadCommand.cs        # Phase 4: File downloading
```

### Testing

See [TESTING.md](TESTING.md) for comprehensive testing instructions and examples.

## License

GPL-2.0 (required for SteamKit2 / Steam CDN access)
