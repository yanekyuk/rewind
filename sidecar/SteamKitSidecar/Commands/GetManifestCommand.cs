using SteamKit2;
using SteamKit2.CDN;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'get-manifest' command: download and parse a specific manifest.
/// Returns file listings with SHA hashes, sizes, and chunk counts.
/// </summary>
public static class GetManifestCommand
{
    public static async Task<int> RunAsync(string username, string password, string? guardCode, uint appId, uint depotId, ulong manifestId)
    {
        using var session = new SteamSession();

        try
        {
            var cts = await session.ConnectAndLoginAsync(username, password, guardCode);

            try
            {
                JsonOutput.Info($"Fetching manifest {manifestId} for depot {depotId}...");

                // Get depot decryption key
                var depotKeyResult = await session.Apps.GetDepotDecryptionKey(depotId, appId);
                if (depotKeyResult.Result != EResult.OK)
                {
                    JsonOutput.Error("DEPOT_KEY_ERROR", $"Failed to get depot key: {depotKeyResult.Result}");
                    JsonOutput.Done(false);
                    return 1;
                }

                var depotKey = depotKeyResult.DepotKey;

                // Get CDN servers
                var cdnServers = await session.Content.GetServersForSteamPipe();
                if (cdnServers == null || cdnServers.Count == 0)
                {
                    JsonOutput.Error("CDN_ERROR", "No CDN servers available");
                    JsonOutput.Done(false);
                    return 1;
                }

                var server = cdnServers.First();

                // Get manifest request code via SteamContent
                var manifestRequestCode = await session.Content.GetManifestRequestCode(depotId, appId, manifestId, "public");

                // Download the manifest
                var cdnClient = new Client(session.Client);

                var manifest = await cdnClient.DownloadManifestAsync(
                    depotId,
                    manifestId,
                    manifestRequestCode,
                    server,
                    depotKey
                );

                // Convert to our output format
                var files = new List<ManifestFileEntry>();
                ulong totalChunks = 0;
                ulong totalBytesOnDisk = 0;
                ulong totalBytesCompressed = 0;

                foreach (var file in manifest.Files!)
                {
                    var sha = file.FileHash != null
                        ? BitConverter.ToString(file.FileHash).Replace("-", "").ToLowerInvariant()
                        : "";

                    files.Add(new ManifestFileEntry
                    {
                        Name = file.FileName,
                        Sha = sha,
                        Size = file.TotalSize,
                        Chunks = (uint)file.Chunks.Count,
                        Flags = (uint)file.Flags,
                    });

                    totalChunks += (ulong)file.Chunks.Count;
                    totalBytesOnDisk += file.TotalSize;
                    foreach (var chunk in file.Chunks)
                    {
                        totalBytesCompressed += chunk.CompressedLength;
                    }
                }

                var metadata = new ManifestMetadata
                {
                    TotalFiles = (ulong)files.Count,
                    TotalChunks = totalChunks,
                    TotalBytesOnDisk = totalBytesOnDisk,
                    TotalBytesCompressed = totalBytesCompressed,
                    Date = manifest.CreationTime.ToString("o"),
                };

                JsonOutput.Manifest(depotId, manifestId.ToString(), metadata, files);
                JsonOutput.Done(true);
                return 0;
            }
            finally
            {
                cts.Cancel();
            }
        }
        catch (Exception ex)
        {
            JsonOutput.Error("MANIFEST_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
