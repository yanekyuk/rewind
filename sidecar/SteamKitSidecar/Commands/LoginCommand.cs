namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'login' command: authenticate with Steam and persist the session.
///
/// In daemon mode, this is handled directly by Program.cs since it needs to
/// manage the shared SteamSession lifecycle. This class is retained for
/// encapsulation but is no longer the primary entry point for login.
/// </summary>
public static class LoginCommand
{
    // Login is now handled directly by Program.HandleLogin since it manages
    // the shared session lifecycle (create, authenticate, start callback loop).
}
