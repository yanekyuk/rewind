using System.Text.Json;
using SteamKit2;
using SteamKit2.Authentication;

namespace SteamKitSidecar;

/// <summary>
/// Manages a long-lived Steam client connection and authentication session.
/// In daemon mode, a single SteamSession is created once and shared across
/// all commands. The session handles login, Steam Guard 2FA, session
/// persistence, and callback processing.
/// </summary>
public sealed class SteamSession : IDisposable
{
    private readonly SteamClient _client;
    private readonly CallbackManager _manager;
    private readonly SteamUser _user;
    private readonly SteamApps _apps;
    private readonly SteamContent _content;
    private bool _isConnected;
    private bool _isLoggedIn;
    private TaskCompletionSource<bool> _connectTcs = new();
    private TaskCompletionSource<bool>? _loginTcs;

    public SteamClient Client => _client;
    public SteamUser User => _user;
    public SteamApps Apps => _apps;
    public SteamContent Content => _content;
    public bool IsLoggedIn => _isLoggedIn;

    public SteamSession()
    {
        _client = new SteamClient();
        _manager = new CallbackManager(_client);
        _user = _client.GetHandler<SteamUser>()!;
        _apps = _client.GetHandler<SteamApps>()!;
        _content = _client.GetHandler<SteamContent>()!;

        _manager.Subscribe<SteamClient.ConnectedCallback>(OnConnected);
        _manager.Subscribe<SteamClient.DisconnectedCallback>(OnDisconnected);
        _manager.Subscribe<SteamUser.LoggedOnCallback>(OnLoggedOn);
        _manager.Subscribe<SteamUser.LoggedOffCallback>(OnLoggedOff);
    }

    /// <summary>
    /// Connect to Steam and authenticate with the given credentials.
    /// Handles Steam Guard 2FA via JSON prompt/response on stdin/stdout.
    ///
    /// In daemon mode, the caller should call <see cref="StartCallbackLoop"/>
    /// after a successful login to keep callbacks running for subsequent commands.
    /// The callback loop started internally during login is stopped before returning.
    /// </summary>
    public async Task<bool> LoginAsync(string username, string password, string? guardCode = null)
    {
        JsonOutput.Info("Connecting to Steam...");

        _client.Connect();

        // Run callbacks in background during login
        var cts = new CancellationTokenSource();
        var callbackTask = Task.Run(() =>
        {
            while (!cts.Token.IsCancellationRequested)
            {
                _manager.RunWaitCallbacks(TimeSpan.FromMilliseconds(100));
            }
        }, cts.Token);

        try
        {
            // Wait for connection
            var connected = await _connectTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
            if (!connected)
            {
                JsonOutput.Error("CONNECTION_FAILED", "Failed to connect to Steam");
                return false;
            }

            JsonOutput.Info("Connected to Steam");

            // Attempt authentication using SteamKit2's authentication system
            var sessionDir = GetSessionDir();
            var sessionFile = Path.Combine(sessionDir, $"{username}.json");

            // Try to load existing session
            var savedSession = LoadSession(sessionFile);
            if (savedSession != null)
            {
                JsonOutput.Info("Using saved session...");
                _loginTcs = new TaskCompletionSource<bool>();

                _user.LogOn(new SteamUser.LogOnDetails
                {
                    Username = username,
                    AccessToken = savedSession.RefreshToken,
                });

                var loggedIn = await _loginTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
                if (loggedIn)
                {
                    JsonOutput.AuthSuccess(username);
                    return true;
                }
                JsonOutput.Warn("Saved session expired, performing fresh login...");

                // Delete the stale session so future attempts skip this path
                try { File.Delete(sessionFile); } catch { /* ignore */ }

                // Steam disconnects after a failed login — wait for it to settle,
                // then reconnect with a fresh TCS before attempting credential auth
                _client.Disconnect();
                await Task.Delay(500);
                _connectTcs = new TaskCompletionSource<bool>();
                _client.Connect();
                var reconnected = await _connectTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
                if (!reconnected)
                {
                    JsonOutput.Error("CONNECTION_FAILED", "Failed to reconnect to Steam after expired session");
                    return false;
                }
            }

            // Fresh login with credential authentication
            // Load previous GuardData to skip Steam Guard on repeat logins
            string? previousGuardData = savedSession?.GuardData;

            var authSession = await _client.Authentication.BeginAuthSessionViaCredentialsAsync(
                new AuthSessionDetails
                {
                    Username = username,
                    Password = password,
                    IsPersistentSession = true,
                    GuardData = previousGuardData,
                    Authenticator = new JsonAuthenticator(guardCode),
                }
            );

            var pollResult = await authSession.PollingWaitForResultAsync();

            _loginTcs = new TaskCompletionSource<bool>();

            _user.LogOn(new SteamUser.LogOnDetails
            {
                Username = pollResult.AccountName,
                AccessToken = pollResult.RefreshToken,
                ShouldRememberPassword = true,
            });

            var result = await _loginTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
            if (result)
            {
                // Save session with RefreshToken and GuardData for future use
                SaveSession(sessionFile, new SavedSession
                {
                    Username = pollResult.AccountName,
                    RefreshToken = pollResult.RefreshToken,
                    GuardData = pollResult.NewGuardData,
                });
                JsonOutput.AuthSuccess(pollResult.AccountName);
            }

            return result;
        }
        finally
        {
            cts.Cancel();
            try { await callbackTask; } catch (OperationCanceledException) { }
        }
    }

    /// <summary>
    /// Check if a saved session exists for any user and attempt silent login.
    /// Returns the username on success, or null if no valid session exists.
    /// </summary>
    public async Task<string?> CheckSessionAsync()
    {
        var sessionDir = GetSessionDir();
        if (!Directory.Exists(sessionDir))
            return null;

        // Find the most recently modified session file
        var sessionFiles = Directory.GetFiles(sessionDir, "*.json");
        if (sessionFiles.Length == 0)
            return null;

        var latestFile = sessionFiles
            .Select(f => new FileInfo(f))
            .OrderByDescending(f => f.LastWriteTimeUtc)
            .First();

        var savedSession = LoadSession(latestFile.FullName);
        if (savedSession == null || string.IsNullOrEmpty(savedSession.RefreshToken))
            return null;

        JsonOutput.Info($"Found saved session for {savedSession.Username}, attempting silent login...");

        _client.Connect();

        var cts = new CancellationTokenSource();
        var callbackTask = Task.Run(() =>
        {
            while (!cts.Token.IsCancellationRequested)
            {
                _manager.RunWaitCallbacks(TimeSpan.FromMilliseconds(100));
            }
        }, cts.Token);

        try
        {
            var connected = await _connectTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
            if (!connected)
                return null;

            _loginTcs = new TaskCompletionSource<bool>();

            _user.LogOn(new SteamUser.LogOnDetails
            {
                Username = savedSession.Username,
                AccessToken = savedSession.RefreshToken,
            });

            var loggedIn = await _loginTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
            if (loggedIn)
            {
                return savedSession.Username;
            }

            // Session expired — delete the stale file
            try { File.Delete(latestFile.FullName); } catch { /* ignore */ }
            return null;
        }
        finally
        {
            cts.Cancel();
            try { await callbackTask; } catch (OperationCanceledException) { }
        }
    }

    /// <summary>
    /// Start a long-running callback processing loop. Returns a CancellationTokenSource
    /// that the caller should cancel when the session is no longer needed.
    ///
    /// In daemon mode, this is called after successful login and runs for the
    /// lifetime of the session. SteamKit2 requires continuous callback processing
    /// to maintain the connection and handle events from the Steam network.
    /// </summary>
    public CancellationTokenSource StartCallbackLoop()
    {
        var cts = new CancellationTokenSource();
        _ = Task.Run(() =>
        {
            while (!cts.Token.IsCancellationRequested)
            {
                _manager.RunWaitCallbacks(TimeSpan.FromMilliseconds(100));
            }
        }, cts.Token);
        return cts;
    }

    private void OnConnected(SteamClient.ConnectedCallback callback)
    {
        _isConnected = true;
        _connectTcs.TrySetResult(true);
    }

    private void OnDisconnected(SteamClient.DisconnectedCallback callback)
    {
        _isConnected = false;
        _isLoggedIn = false;
        _connectTcs.TrySetResult(false);
        _loginTcs?.TrySetResult(false);
    }

    private void OnLoggedOn(SteamUser.LoggedOnCallback callback)
    {
        if (callback.Result == EResult.OK)
        {
            _isLoggedIn = true;
            _loginTcs?.TrySetResult(true);
        }
        else if (callback.Result == EResult.TryAnotherCM)
        {
            JsonOutput.Warn("Steam said TryAnotherCM, reconnecting...");
            // Don't resolve loginTcs yet — reconnect and the caller will retry
            _loginTcs?.TrySetResult(false);
        }
        else
        {
            JsonOutput.Error("AUTH_FAILED", $"Login failed: {callback.Result} ({callback.ExtendedResult})");
            _loginTcs?.TrySetResult(false);
        }
    }

    private void OnLoggedOff(SteamUser.LoggedOffCallback callback)
    {
        _isLoggedIn = false;
    }

    private static string GetSessionDir()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "rewind",
            "sessions"
        );
        Directory.CreateDirectory(dir);
        return dir;
    }

    private static SavedSession? LoadSession(string path)
    {
        try
        {
            if (!File.Exists(path)) return null;
            var json = File.ReadAllText(path);
            return JsonSerializer.Deserialize<SavedSession>(json);
        }
        catch
        {
            return null;
        }
    }

    private static void SaveSession(string path, SavedSession session)
    {
        try
        {
            var json = JsonSerializer.Serialize(session);
            File.WriteAllText(path, json);
        }
        catch (Exception ex)
        {
            JsonOutput.Warn($"Failed to save session: {ex.Message}");
        }
    }

    public void Dispose()
    {
        if (_isConnected)
        {
            _user.LogOff();
            _client.Disconnect();
        }
    }
}

internal class SavedSession
{
    public string Username { get; set; } = "";
    public string RefreshToken { get; set; } = "";
    public string? GuardData { get; set; }
}

/// <summary>
/// SteamKit2 authenticator that handles Steam Guard via JSON stdin/stdout.
/// </summary>
internal class JsonAuthenticator : IAuthenticator
{
    private readonly string? _presetCode;

    public JsonAuthenticator(string? presetCode = null)
    {
        _presetCode = presetCode;
    }

    public Task<string> GetDeviceCodeAsync(bool previousCodeWasIncorrect)
    {
        if (!string.IsNullOrEmpty(_presetCode) && !previousCodeWasIncorrect)
        {
            return Task.FromResult(_presetCode);
        }

        JsonOutput.GuardPrompt("device", null);
        var code = Console.ReadLine()?.Trim() ?? "";
        return Task.FromResult(code);
    }

    public Task<string> GetEmailCodeAsync(string email, bool previousCodeWasIncorrect)
    {
        if (!string.IsNullOrEmpty(_presetCode) && !previousCodeWasIncorrect)
        {
            return Task.FromResult(_presetCode);
        }

        JsonOutput.GuardPrompt("email", email);
        var code = Console.ReadLine()?.Trim() ?? "";
        return Task.FromResult(code);
    }

    public Task<bool> AcceptDeviceConfirmationAsync()
    {
        JsonOutput.GuardPrompt("device_confirm", null);
        // Return true immediately — SteamKit2 polls the Steam backend for
        // the phone approval; we don't need to block on stdin.
        return Task.FromResult(true);
    }
}

/// <summary>
/// Thrown when the sidecar cannot authenticate because no saved session
/// exists and no password was provided. Caught by command handlers to
/// emit a structured AUTH_REQUIRED error code.
/// </summary>
public class AuthRequiredException : Exception
{
    public AuthRequiredException(string message) : base(message) { }
}
