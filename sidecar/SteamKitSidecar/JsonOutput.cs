using System.Text.Json;
using System.Text.Json.Serialization;

namespace SteamKitSidecar;

/// <summary>
/// NDJSON output helpers. Each method writes a single JSON line to stdout.
/// The Rust infrastructure layer reads these lines and deserializes by "type" field.
/// </summary>
public static class JsonOutput
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static void Log(string level, string message)
    {
        var obj = new { Type = "log", Level = level, Message = message };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void Info(string message) => Log("info", message);
    public static void Warn(string message) => Log("warn", message);

    public static void GuardPrompt(string method, string? hint = null)
    {
        var obj = new { Type = "guard_prompt", Method = method, Hint = hint };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void AuthSuccess(string sessionFile)
    {
        var obj = new { Type = "auth_success", SessionFile = sessionFile };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void ManifestList(List<ManifestListItem> manifests)
    {
        var obj = new { Type = "manifest_list", Manifests = manifests };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void DepotList(List<DepotListItem> depots)
    {
        var obj = new { Type = "depot_list", Depots = depots };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void Manifest(ulong depotId, string manifestId, ManifestMetadata metadata, List<ManifestFileEntry> files)
    {
        var obj = new
        {
            Type = "manifest",
            DepotId = depotId,
            ManifestId = manifestId,
            TotalFiles = metadata.TotalFiles,
            TotalChunks = metadata.TotalChunks,
            TotalBytesOnDisk = metadata.TotalBytesOnDisk,
            TotalBytesCompressed = metadata.TotalBytesCompressed,
            Date = metadata.Date,
            Files = files,
        };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void Progress(double percent, ulong bytesDownloaded, ulong bytesTotal)
    {
        var obj = new { Type = "progress", Percent = percent, BytesDownloaded = bytesDownloaded, BytesTotal = bytesTotal };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }

    public static void Error(string code, string message)
    {
        var obj = new { Type = "error", Code = code, Message = message };
        Console.Error.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Error.Flush();
    }

    public static void Done(bool success)
    {
        var obj = new { Type = "done", Success = success };
        Console.WriteLine(JsonSerializer.Serialize(obj, Options));
        Console.Out.Flush();
    }
}

public class ManifestListItem
{
    public string Id { get; set; } = "";
    public string Branch { get; set; } = "";
    public ulong? TimeUpdated { get; set; }
    public bool? PwdRequired { get; set; }
}

public class ManifestMetadata
{
    public ulong TotalFiles { get; set; }
    public ulong TotalChunks { get; set; }
    public ulong TotalBytesOnDisk { get; set; }
    public ulong TotalBytesCompressed { get; set; }
    public string Date { get; set; } = "";
}

public class ManifestFileEntry
{
    public string Name { get; set; } = "";
    public string Sha { get; set; } = "";
    public ulong Size { get; set; }
    public uint Chunks { get; set; }
    public uint Flags { get; set; }
}

public class DepotListItem
{
    public uint DepotId { get; set; }
    public string? Name { get; set; }
    public ulong? MaxSize { get; set; }
    public uint? DlcAppId { get; set; }
}
