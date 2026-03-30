# SteamKit Sidecar Architecture

## Overview

The SteamKit sidecar is a .NET 9.0 console application that implements Steam manifest and file downloading for the Rewind game downgrader. It replaces the previous DepotDownloader integration with a native SteamKit2-based implementation.

## Design Principles

1. **Simplicity**: Single responsibility per command, minimal dependencies
2. **Streaming**: NDJSON output for progressive parsing by Rust backend
3. **State Management**: Session persistence to avoid re-authentication
4. **Error Clarity**: Typed error codes for systematic error handling
5. **Async First**: All I/O operations are fully asynchronous

## Phase 1: Authentication Flow

### Components

**SteamSession class**
- Manages SteamClient connection lifecycle
- Handles callback subscriptions and processing
- Implements both one-shot login and persistent connected sessions

**JsonAuthenticator class**
- Implements SteamKit2's IAuthenticator interface
- Supports three 2FA methods:
  - `device`: Mobile authenticator codes
  - `email`: Email confirmation codes
  - `device_confirm`: Device confirmation for new logins
- Can accept preset codes via `--guard-code` to avoid prompts

**LoginCommand**
- Authenticates and persists session

### Session Persistence Strategy

Sessions are cached in platform-specific directories:

```
Windows: %LOCALAPPDATA%\rewind\sessions\<username>.json
Linux:   ~/.local/share/rewind/sessions/<username>.json
macOS:   ~/Library/Application Support/rewind/sessions/<username>.json
```

Session file structure:
```json
{
  "username": "steamuser",
  "refresh_token": "..."
}
```

On next login, the sidecar tries to reuse the saved refresh token:
1. Load saved session from disk
2. Attempt to log in with refresh token
3. If expired, fall back to credential authentication

This approach eliminates 2FA prompts on subsequent logins.

### Callback Management

```
Client.Connect()
  -> ConnectedCallback -> _connectTcs.SetResult(true)
  -> Authentication.BeginAuthSessionViaCredentialsAsync()
  -> PollingWaitForResultAsync() (polls Steam servers)
  -> LogOn() with refresh token
  -> LoggedOnCallback -> _loginTcs.SetResult(true)
```

All callbacks are processed in a background task that polls with 100ms intervals:

```csharp
var callbackTask = Task.Run(() => {
    while (!cts.Token.IsCancellationRequested) {
        _manager.RunWaitCallbacks(TimeSpan.FromMilliseconds(100));
    }
}, cts.Token);
```

## Phase 2: Manifest Listing

### Steam CDN Architecture

Steam stores manifests in PICS (Product Info Code Service):

```
App (e.g., 570 = Dota 2)
  └── Depots
      └── Depot (e.g., 373307)
          └── Manifests
              └── Public: <manifest_id>
```

### Implementation

ListManifestsCommand:
1. Calls `Apps.GetDepotDecryptionKey()` to get depot access
2. Calls `Apps.PICSGetProductInfo()` to retrieve product metadata
3. Navigates KeyValues tree: `app[depotId]["manifests"]["public"]`
4. Returns manifest ID and (empty) date

### Limitations

Steam's PICS API provides the current public manifest but not full history. Full manifest enumeration would require:
- Polling Steam's CDN for historical versions
- Using PICSGetChangesSince() with previous change numbers
- Maintaining local manifest history cache

Current implementation is sufficient for the most recent manifest.

## Phase 3: Manifest Fetching

### Manifest Download Pipeline

```
1. GetDepotDecryptionKey() -> depot_key
2. GetServersForSteamPipe() -> cdn_servers
3. GetManifestRequestCode() -> manifest_code
4. CDN.Client.DownloadManifestAsync() -> Manifest object
5. Parse Manifest.Files -> file metadata
```

### Data Extraction

For each file in manifest:

```csharp
var file = manifest.Files[i];
var entry = new ManifestFileEntry {
    Name = file.FileName,
    Sha = BitConverter.ToString(file.FileHash).Replace("-", ""),
    Size = file.TotalSize,
    Chunks = (uint)file.Chunks.Count,
    Flags = (uint)file.Flags  // 0=file, 16=directory, etc.
};
```

Aggregate statistics:
- `TotalFiles`: Count of all files
- `TotalChunks`: Sum of chunks across all files
- `TotalBytesOnDisk`: Uncompressed total
- `TotalBytesCompressed`: Sum of all chunk.CompressedLength

### CDN Server Selection

```csharp
var cdnServers = await session.Content.GetServersForSteamPipe();
var server = cdnServers.First(); // Load balancing would pick best server
```

In production, selection could consider:
- Geographic latency
- Server load
- Network throughput

## Phase 4: File Downloading

### Download Strategy

```
For each file in manifest:
  1. Create directory structure
  2. If file has no chunks (directory flag):
     - Create empty directory, continue
  3. Otherwise, for each chunk in file:
     a. Allocate buffer[chunk.UncompressedLength]
     b. Call CDN.Client.DownloadDepotChunkAsync()
     c. Chunk data returned already decompressed
     d. Write to file
     e. Report progress
```

### Progress Reporting

```csharp
var percent = totalBytes > 0 ? (double)downloadedBytes / totalBytes * 100.0 : 0;
JsonOutput.Progress(Math.Round(percent, 1), downloadedBytes, totalBytes);
```

Events sent after each chunk for fine-grained progress updates.

### File Filtering

Optional `--filelist` parameter allows selective download:

```
filelist.txt:
game.exe
game.dll
assets/texture.dds
```

Implementation:
```csharp
var allowedFiles = new HashSet<string>(File.ReadAllLines(filelistPath),
    StringComparer.OrdinalIgnoreCase);
filesToDownload = filesToDownload
    .Where(f => allowedFiles.Contains(f.FileName))
    .ToList();
```

### Concurrency Considerations

Current implementation downloads sequentially. Could be enhanced with:
- Parallel chunk downloads (multiple chunks from different files)
- Thread pool to manage CDN connections
- Rate limiting to avoid overloading CDN

Trade-offs:
- Current: Simple, reliable, bounded memory
- Parallel: Faster but more resource intensive

## Communication Protocol

### NDJSON Format

Each line is a complete JSON object:

```json
{"type":"log","level":"info","message":"Connecting to Steam..."}
{"type":"auth_success","session_file":"..."}
{"type":"done","success":true}
```

No streaming of partial objects, no compression, ASCII-safe.

### Message Types

**Logging**
```json
{"type":"log","level":"info|warn","message":"text"}
```

**Guard Prompts**
```json
{"type":"guard_prompt","method":"email|device|device_confirm","hint":"email@example.com"}
```
Rust backend reads prompt, shows UI, writes response to sidecar stdin.

**Success Messages**
- `auth_success`: Login completed, session saved
- `manifest_list`: List of manifests returned
- `manifest`: Manifest metadata and file list
- `progress`: Download progress update
- `done`: Command completed (success: true/false)

**Errors**
```json
{"type":"error","code":"ERROR_CODE","message":"human readable text"}
```

### Synchronization

Commands are synchronous request-response:
1. Rust backend starts sidecar
2. Sidecar processes command
3. Sidecar outputs NDJSON
4. Sidecar exits with code 0 (success) or 1 (failure)
5. Rust backend parses output and reports

For interactive operations (Steam Guard), the sidecar can prompt:
1. Sidecar outputs `guard_prompt`
2. Sidecar blocks on `Console.ReadLine()`
3. Rust backend reads prompt, shows UI, writes code to stdin
4. Sidecar reads code and continues authentication

## Error Handling

All errors are caught and converted to JSON messages:

```csharp
catch (Exception ex) {
    JsonOutput.Error("ERROR_CODE", ex.Message);
    JsonOutput.Done(false);
    return 1;
}
```

Error codes indicate severity and category:
- `USAGE`: User error (missing args, invalid command)
- `CONNECTION_FAILED`: Network/Steam connectivity
- `AUTH_*`: Authentication issues
- `MANIFEST_*`: Manifest operations
- `CDN_ERROR`: Content delivery network issues
- `DOWNLOAD_ERROR`: File download failures
- `UNHANDLED_ERROR`: Unexpected exceptions

## Performance Characteristics

### Authentication
- First login: ~2-5 seconds (credential validation, 2FA if needed)
- Subsequent logins: ~1 second (refresh token reuse)

### Manifest Listing
- ~1 second (single PICS query)

### Manifest Fetching
- Depends on manifest size
- Typical: 100-500ms (network + parsing)

### File Downloading
- Bottleneck: CDN bandwidth
- Chunk size: typically 64KB-1MB
- Sequential throughput: limited by CDN, typically 10-50 Mbps

### Memory Usage
- SteamClient: ~50MB
- Manifest parsing: ~100MB (for large manifests)
- Download: Bounded by chunk buffer size (~10MB)

## Integration Points

### Tauri Sidecar
The Rust backend (src-tauri/) spawns this executable:
```rust
let child = Command::new("SteamKitSidecar")
    .arg("login")
    .arg("--username").arg(username)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped())
    .spawn()?;
```

### Manifest Storage
Downloaded manifests are stored in:
```
~/.rewind/manifests/<app>_<depot>_<manifestId>.json
```

### File Organization
Downloaded files maintain depot structure:
```
./output/
  game.exe
  game.dll
  assets/
    texture.dds
    model.mesh
```

## Testing Strategy

See [TESTING.md](../sidecar/SteamKitSidecar/TESTING.md) for comprehensive test scenarios.

Unit test opportunities:
- `JsonAuthenticator`: Mock Steam response
- `SteamSession`: Mock SteamClient callbacks
- `JsonOutput`: Verify NDJSON format
- Command parsers: Verify argument parsing

Integration test scenarios:
- Real Steam login with credentials
- Manifest listing for known apps
- Manifest fetching with validation
- File downloading with hash verification

## Future Enhancements

1. **Historical Manifests**: Implement full manifest history enumeration
2. **Parallel Downloads**: Concurrent chunk downloads with bounded concurrency
3. **Delta Downloads**: Only download changed files between manifests
4. **Compression**: ZSTD compression for manifest JSON
5. **Caching**: Local manifest cache to avoid re-fetching
6. **Rate Limiting**: Respect CDN rate limits and backoff
7. **Resilience**: Automatic retry with exponential backoff
8. **Metrics**: Report timing and throughput statistics
