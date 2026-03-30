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
            var success = await session.LoginAsync(username, password, guardCode);
            JsonOutput.Done(success);
            return success ? 0 : 1;
        }
        catch (Exception ex)
        {
            JsonOutput.Error("AUTH_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
