using SteamKit2;
using SteamKit2.CDN;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'get-manifest' command: download and parse a specific manifest.
/// Returns file listings with SHA hashes, sizes, and chunk counts.
///
/// In daemon mode, receives a shared SteamSession that is already connected
/// and authenticated.
/// </summary>
public static class GetManifestCommand
{
    public static async Task RunAsync(SteamSession session, uint appId, uint depotId, ulong manifestId, string? requestId)
    {
        try
        {
            JsonOutput.Info($"Fetching manifest {manifestId} for depot {depotId}...", requestId);

            // Get depot decryption key
            var depotKeyResult = await session.Apps.GetDepotDecryptionKey(depotId, appId);
            if (depotKeyResult.Result != EResult.OK)
            {
                JsonOutput.Error("DEPOT_KEY_ERROR", $"Failed to get depot key: {depotKeyResult.Result}", requestId);
                JsonOutput.Done(false, requestId);
                return;
            }

            var depotKey = depotKeyResult.DepotKey;

            // Get CDN servers
            var cdnServers = await session.Content.GetServersForSteamPipe();
            if (cdnServers == null || cdnServers.Count == 0)
            {
                JsonOutput.Error("CDN_ERROR", "No CDN servers available", requestId);
                JsonOutput.Done(false, requestId);
                return;
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

            JsonOutput.Manifest(depotId, manifestId.ToString(), metadata, files, requestId);
            JsonOutput.Done(true, requestId);
        }
        catch (AuthRequiredException ex)
        {
            JsonOutput.Error("AUTH_REQUIRED", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
        catch (Exception ex)
        {
            JsonOutput.Error("MANIFEST_ERROR", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
    }
}
