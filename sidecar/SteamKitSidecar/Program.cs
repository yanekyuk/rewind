using SteamKitSidecar.Commands;

namespace SteamKitSidecar;

/// <summary>
/// Entry point for the SteamKitSidecar .NET console application.
/// Routes CLI subcommands to their respective handlers.
///
/// Usage:
///   SteamKitSidecar login --username <user> --password <pass> [--guard-code <code>]
///   SteamKitSidecar list-manifests --username <user> --password <pass> --app <id> --depot <id>
///   SteamKitSidecar get-manifest --username <user> --password <pass> --app <id> --depot <id> --manifest <id>
///   SteamKitSidecar download --username <user> --password <pass> --app <id> --depot <id> --manifest <id> --dir <path> [--filelist <path>]
/// </summary>
public static class Program
{
    public static async Task<int> Main(string[] args)
    {
        if (args.Length == 0)
        {
            JsonOutput.Error("USAGE", "No command specified. Available commands: login, list-manifests, get-manifest, download");
            return 1;
        }

        var command = args[0];
        var options = ParseArgs(args.Skip(1).ToArray());

        try
        {
            return command switch
            {
                "login" => await HandleLogin(options),
                "list-manifests" => await HandleListManifests(options),
                "get-manifest" => await HandleGetManifest(options),
                "download" => await HandleDownload(options),
                _ => HandleUnknown(command),
            };
        }
        catch (Exception ex)
        {
            JsonOutput.Error("UNHANDLED_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }

    private static async Task<int> HandleLogin(Dictionary<string, string> options)
    {
        var username = GetRequired(options, "username");
        var password = GetRequired(options, "password");
        var guardCode = GetOptional(options, "guard-code");

        return await LoginCommand.RunAsync(username, password, guardCode);
    }

    private static async Task<int> HandleListManifests(Dictionary<string, string> options)
    {
        var username = GetRequired(options, "username");
        var password = GetRequired(options, "password");
        var appId = uint.Parse(GetRequired(options, "app"));
        var depotId = uint.Parse(GetRequired(options, "depot"));
        var guardCode = GetOptional(options, "guard-code");

        return await ListManifestsCommand.RunAsync(username, password, guardCode, appId, depotId);
    }

    private static async Task<int> HandleGetManifest(Dictionary<string, string> options)
    {
        var username = GetRequired(options, "username");
        var password = GetRequired(options, "password");
        var appId = uint.Parse(GetRequired(options, "app"));
        var depotId = uint.Parse(GetRequired(options, "depot"));
        var manifestId = ulong.Parse(GetRequired(options, "manifest"));
        var guardCode = GetOptional(options, "guard-code");

        return await GetManifestCommand.RunAsync(username, password, guardCode, appId, depotId, manifestId);
    }

    private static async Task<int> HandleDownload(Dictionary<string, string> options)
    {
        var username = GetRequired(options, "username");
        var password = GetRequired(options, "password");
        var appId = uint.Parse(GetRequired(options, "app"));
        var depotId = uint.Parse(GetRequired(options, "depot"));
        var manifestId = ulong.Parse(GetRequired(options, "manifest"));
        var outputDir = GetRequired(options, "dir");
        var guardCode = GetOptional(options, "guard-code");
        var filelist = GetOptional(options, "filelist");

        return await DownloadCommand.RunAsync(username, password, guardCode, appId, depotId, manifestId, outputDir, filelist);
    }

    private static int HandleUnknown(string command)
    {
        JsonOutput.Error("UNKNOWN_COMMAND", $"Unknown command: {command}. Available: login, list-manifests, get-manifest, download");
        return 1;
    }

    private static Dictionary<string, string> ParseArgs(string[] args)
    {
        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        for (int i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            if (arg.StartsWith("--") && i + 1 < args.Length)
            {
                var key = arg[2..]; // Strip leading --
                var value = args[i + 1];
                result[key] = value;
                i++; // Skip the value
            }
        }

        return result;
    }

    private static string GetRequired(Dictionary<string, string> options, string key)
    {
        if (!options.TryGetValue(key, out var value) || string.IsNullOrEmpty(value))
        {
            throw new ArgumentException($"Missing required option: --{key}");
        }
        return value;
    }

    private static string? GetOptional(Dictionary<string, string> options, string key)
    {
        options.TryGetValue(key, out var value);
        return string.IsNullOrEmpty(value) ? null : value;
    }
}
