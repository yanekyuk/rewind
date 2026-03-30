namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'login' command: authenticate with Steam and persist the session.
/// </summary>
public static class LoginCommand
{
    public static async Task<int> RunAsync(string username, string password, string? guardCode)
    {
        using var session = new SteamSession();

        try
        {
            var cts = await session.ConnectAndLoginAsync(username, password, guardCode);
            cts.Cancel(); // Stop callback processing
            JsonOutput.AuthSuccess(username);
            JsonOutput.Done(true);
            return 0;
        }
        catch (Exception ex)
        {
            JsonOutput.Error("AUTH_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
