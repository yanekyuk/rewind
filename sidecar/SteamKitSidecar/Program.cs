using System.Text.Json;
using SteamKitSidecar.Commands;

namespace SteamKitSidecar;

/// <summary>
/// Entry point for the SteamKitSidecar .NET console application.
///
/// Runs as a long-lived daemon process: reads NDJSON commands from stdin,
/// one per line, and writes NDJSON responses to stdout. A single SteamSession
/// is shared across all commands, so the sidecar connects and authenticates
/// once and reuses the connection for subsequent operations.
///
/// Command protocol (stdin):
///   {"request_id":"r1","command":"login","username":"user","password":"pass"}
///   {"request_id":"r2","command":"list-depots","app_id":3321460}
///   {"request_id":"r3","command":"list-manifests","app_id":3321460,"depot_id":3321461}
///   {"request_id":"r4","command":"get-manifest","app_id":3321460,"depot_id":3321461,"manifest_id":1234567890}
///   {"request_id":"r5","command":"download","app_id":3321460,"depot_id":3321461,"manifest_id":1234567890,"dir":"/tmp/out"}
///
/// Response protocol (stdout):
///   Each response line includes the matching request_id for correlation.
/// </summary>
public static class Program
{
    /// <summary>
    /// Shared SteamSession instance, created lazily on first login command
    /// and reused for all subsequent commands. Disposed on exit.
    /// </summary>
    private static SteamSession? _session;

    /// <summary>
    /// Cancellation token source for the callback loop associated with the
    /// current session. Cancelled and recreated on session restart.
    /// </summary>
    private static CancellationTokenSource? _callbackCts;

    public static async Task<int> Main(string[] args)
    {
        // Daemon mode: read NDJSON commands from stdin
        JsonOutput.Info("SteamKitSidecar daemon started");

        try
        {
            while (true)
            {
                var line = await ReadLineAsync();
                if (line == null)
                {
                    // stdin closed — parent process terminated
                    break;
                }

                var trimmed = line.Trim();
                if (string.IsNullOrEmpty(trimmed))
                    continue;

                await HandleCommandLine(trimmed);
            }
        }
        finally
        {
            Cleanup();
        }

        return 0;
    }

    /// <summary>
    /// Read a line from stdin asynchronously. Returns null on EOF.
    /// </summary>
    private static async Task<string?> ReadLineAsync()
    {
        return await Task.Run(() => Console.ReadLine());
    }

    /// <summary>
    /// Parse and dispatch a single NDJSON command line.
    /// </summary>
    private static async Task HandleCommandLine(string json)
    {
        JsonElement root;
        try
        {
            root = JsonSerializer.Deserialize<JsonElement>(json);
        }
        catch (JsonException ex)
        {
            JsonOutput.Error("PARSE_ERROR", $"Invalid JSON: {ex.Message}");
            return;
        }

        var requestId = root.TryGetProperty("request_id", out var rid) ? rid.GetString() : null;
        var command = root.TryGetProperty("command", out var cmd) ? cmd.GetString() : null;

        if (string.IsNullOrEmpty(command))
        {
            JsonOutput.Error("PARSE_ERROR", "Missing 'command' field", requestId);
            JsonOutput.Done(false, requestId);
            return;
        }

        try
        {
            switch (command)
            {
                case "login":
                    await HandleLogin(root, requestId);
                    break;
                case "list-depots":
                    await HandleListDepots(root, requestId);
                    break;
                case "list-manifests":
                    await HandleListManifests(root, requestId);
                    break;
                case "get-manifest":
                    await HandleGetManifest(root, requestId);
                    break;
                case "download":
                    await HandleDownload(root, requestId);
                    break;
                default:
                    JsonOutput.Error("UNKNOWN_COMMAND", $"Unknown command: {command}. Available: login, list-depots, list-manifests, get-manifest, download", requestId);
                    JsonOutput.Done(false, requestId);
                    break;
            }
        }
        catch (Exception ex)
        {
            JsonOutput.Error("UNHANDLED_ERROR", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
    }

    private static async Task HandleLogin(JsonElement root, string? requestId)
    {
        var username = GetRequiredString(root, "username");
        var password = GetRequiredString(root, "password");
        var guardCode = GetOptionalString(root, "guard_code");

        // Dispose previous session if any
        Cleanup();

        _session = new SteamSession();

        try
        {
            var success = await _session.LoginAsync(username, password, guardCode);
            if (success)
            {
                // Start callback processing for the shared session
                _callbackCts = _session.StartCallbackLoop();
                JsonOutput.AuthSuccess(username, requestId);
                JsonOutput.Done(true, requestId);
            }
            else
            {
                JsonOutput.Error("AUTH_FAILED", "Login returned false", requestId);
                JsonOutput.Done(false, requestId);
                Cleanup();
            }
        }
        catch (Exception ex)
        {
            JsonOutput.Error("AUTH_ERROR", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
            Cleanup();
        }
    }

    private static async Task HandleListDepots(JsonElement root, string? requestId)
    {
        EnsureLoggedIn(requestId);
        var appId = GetRequiredUint(root, "app_id");
        await ListDepotsCommand.RunAsync(_session!, appId, requestId);
    }

    private static async Task HandleListManifests(JsonElement root, string? requestId)
    {
        EnsureLoggedIn(requestId);
        var appId = GetRequiredUint(root, "app_id");
        var depotId = GetRequiredUint(root, "depot_id");
        await ListManifestsCommand.RunAsync(_session!, appId, depotId, requestId);
    }

    private static async Task HandleGetManifest(JsonElement root, string? requestId)
    {
        EnsureLoggedIn(requestId);
        var appId = GetRequiredUint(root, "app_id");
        var depotId = GetRequiredUint(root, "depot_id");
        var manifestId = GetRequiredUlong(root, "manifest_id");
        await GetManifestCommand.RunAsync(_session!, appId, depotId, manifestId, requestId);
    }

    private static async Task HandleDownload(JsonElement root, string? requestId)
    {
        EnsureLoggedIn(requestId);
        var appId = GetRequiredUint(root, "app_id");
        var depotId = GetRequiredUint(root, "depot_id");
        var manifestId = GetRequiredUlong(root, "manifest_id");
        var outputDir = GetRequiredString(root, "dir");
        var filelist = GetOptionalString(root, "filelist");
        await DownloadCommand.RunAsync(_session!, appId, depotId, manifestId, outputDir, filelist, requestId);
    }

    /// <summary>
    /// Verify that a session is active. Throws if not logged in.
    /// </summary>
    private static void EnsureLoggedIn(string? requestId)
    {
        if (_session == null || !_session.IsLoggedIn)
        {
            throw new AuthRequiredException("Not logged in. Send a 'login' command first.");
        }
    }

    /// <summary>
    /// Dispose the current session and cancel callback processing.
    /// </summary>
    private static void Cleanup()
    {
        _callbackCts?.Cancel();
        _callbackCts = null;
        _session?.Dispose();
        _session = null;
    }

    private static string GetRequiredString(JsonElement root, string key)
    {
        if (root.TryGetProperty(key, out var prop) && prop.ValueKind == JsonValueKind.String)
        {
            var value = prop.GetString();
            if (!string.IsNullOrEmpty(value))
                return value;
        }
        throw new ArgumentException($"Missing required field: {key}");
    }

    private static string? GetOptionalString(JsonElement root, string key)
    {
        if (root.TryGetProperty(key, out var prop) && prop.ValueKind == JsonValueKind.String)
            return prop.GetString();
        return null;
    }

    private static uint GetRequiredUint(JsonElement root, string key)
    {
        if (root.TryGetProperty(key, out var prop))
        {
            if (prop.ValueKind == JsonValueKind.Number)
                return prop.GetUInt32();
            if (prop.ValueKind == JsonValueKind.String && uint.TryParse(prop.GetString(), out var val))
                return val;
        }
        throw new ArgumentException($"Missing required field: {key}");
    }

    private static ulong GetRequiredUlong(JsonElement root, string key)
    {
        if (root.TryGetProperty(key, out var prop))
        {
            if (prop.ValueKind == JsonValueKind.Number)
                return prop.GetUInt64();
            if (prop.ValueKind == JsonValueKind.String && ulong.TryParse(prop.GetString(), out var val))
                return val;
        }
        throw new ArgumentException($"Missing required field: {key}");
    }
}
