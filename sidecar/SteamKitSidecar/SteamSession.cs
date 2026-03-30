using System.Text.Json;
using SteamKit2;
using SteamKit2.Authentication;

namespace SteamKitSidecar;

/// <summary>
/// Manages a Steam client connection and authentication session.
/// Handles login, Steam Guard 2FA, and session persistence.
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
    private readonly TaskCompletionSource<bool> _connectTcs = new();
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
    /// </summary>
    public async Task<bool> LoginAsync(string username, string password, string? guardCode = null)
    {
        JsonOutput.Info("Connecting to Steam...");

        _client.Connect();

        // Run callbacks in background
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
                    JsonOutput.AuthSuccess(sessionFile);
                    return true;
                }
                JsonOutput.Warn("Saved session expired, performing fresh login...");
            }

            // Fresh login with credential authentication
            var authSession = await _client.Authentication.BeginAuthSessionViaCredentialsAsync(
                new AuthSessionDetails
                {
                    Username = username,
                    Password = password,
                    Authenticator = new JsonAuthenticator(guardCode),
                }
            );

            var pollResult = await authSession.PollingWaitForResultAsync();

            _loginTcs = new TaskCompletionSource<bool>();

            _user.LogOn(new SteamUser.LogOnDetails
            {
                Username = pollResult.AccountName,
                AccessToken = pollResult.RefreshToken,
            });

            var result = await _loginTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
            if (result)
            {
                // Save session for future use
                SaveSession(sessionFile, new SavedSession
                {
                    Username = pollResult.AccountName,
                    RefreshToken = pollResult.RefreshToken,
                });
                JsonOutput.AuthSuccess(sessionFile);
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
    /// Connect to Steam and start callback processing. Returns a CancellationTokenSource
    /// that should be cancelled when done.
    /// </summary>
    public async Task<CancellationTokenSource> ConnectAndLoginAsync(string username, string password, string? guardCode = null)
    {
        _client.Connect();

        var cts = new CancellationTokenSource();
        _ = Task.Run(() =>
        {
            while (!cts.Token.IsCancellationRequested)
            {
                _manager.RunWaitCallbacks(TimeSpan.FromMilliseconds(100));
            }
        }, cts.Token);

        // Wait for connection
        var connected = await _connectTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
        if (!connected)
        {
            cts.Cancel();
            throw new Exception("Failed to connect to Steam");
        }

        JsonOutput.Info("Connected to Steam");

        var sessionDir = GetSessionDir();
        var sessionFile = Path.Combine(sessionDir, $"{username}.json");

        // Try saved session first
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
                return cts;
            }
            JsonOutput.Warn("Saved session expired, performing fresh login...");
        }

        // Fresh credential auth
        var authSession = await _client.Authentication.BeginAuthSessionViaCredentialsAsync(
            new AuthSessionDetails
            {
                Username = username,
                Password = password,
                Authenticator = new JsonAuthenticator(guardCode),
            }
        );

        var pollResult = await authSession.PollingWaitForResultAsync();

        _loginTcs = new TaskCompletionSource<bool>();

        _user.LogOn(new SteamUser.LogOnDetails
        {
            Username = pollResult.AccountName,
            AccessToken = pollResult.RefreshToken,
        });

        var result = await _loginTcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
        if (!result)
        {
            cts.Cancel();
            throw new Exception("Login failed");
        }

        SaveSession(sessionFile, new SavedSession
        {
            Username = pollResult.AccountName,
            RefreshToken = pollResult.RefreshToken,
        });

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
        // Wait for confirmation from stdin (any line = confirmed)
        Console.ReadLine();
        return Task.FromResult(true);
    }
}
