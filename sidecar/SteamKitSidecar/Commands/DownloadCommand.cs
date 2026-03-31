using SteamKit2;
using SteamKit2.CDN;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'download' command: download depot files for a specific manifest via Steam CDN.
/// Supports optional filelist filtering.
/// </summary>
public static class DownloadCommand
{
    public static async Task<int> RunAsync(
        string username, string? password, string? guardCode,
        uint appId, uint depotId, ulong manifestId,
        string outputDir, string? filelistPath)
    {
        using var session = new SteamSession();

        try
        {
            var cts = await session.ConnectAndLoginAsync(username, password, guardCode);

            try
            {
                JsonOutput.Info($"Downloading depot {depotId} manifest {manifestId}...");

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
                var cdnClient = new Client(session.Client);

                // Get manifest request code via SteamContent
                var manifestRequestCode = await session.Content.GetManifestRequestCode(depotId, appId, manifestId, "public");

                // Download the manifest first
                var manifest = await cdnClient.DownloadManifestAsync(
                    depotId,
                    manifestId,
                    manifestRequestCode,
                    server,
                    depotKey
                );

                // Filter files if filelist provided
                var filesToDownload = manifest.Files!.ToList();
                if (filelistPath != null && File.Exists(filelistPath))
                {
                    var allowedFiles = new HashSet<string>(
                        File.ReadAllLines(filelistPath)
                            .Select(l => l.Trim())
                            .Where(l => !string.IsNullOrEmpty(l)),
                        StringComparer.OrdinalIgnoreCase
                    );
                    filesToDownload = filesToDownload
                        .Where(f => allowedFiles.Contains(f.FileName))
                        .ToList();
                }

                // Calculate total bytes
                ulong totalBytes = 0;
                foreach (var file in filesToDownload)
                {
                    totalBytes += file.TotalSize;
                }

                ulong downloadedBytes = 0;
                var totalFiles = filesToDownload.Count;
                var completedFiles = 0;

                JsonOutput.Info($"Downloading {totalFiles} files ({totalBytes} bytes)...");

                Directory.CreateDirectory(outputDir);

                foreach (var file in filesToDownload)
                {
                    // Skip directories (they have no chunks)
                    if (file.Chunks.Count == 0)
                    {
                        var dirPath = Path.Combine(outputDir, file.FileName);
                        if (file.Flags.HasFlag(EDepotFileFlag.Directory))
                        {
                            Directory.CreateDirectory(dirPath);
                        }
                        completedFiles++;
                        continue;
                    }

                    var filePath = Path.Combine(outputDir, file.FileName);
                    var fileDir = Path.GetDirectoryName(filePath);
                    if (fileDir != null)
                    {
                        Directory.CreateDirectory(fileDir);
                    }

                    using var fs = File.Create(filePath);

                    foreach (var chunk in file.Chunks)
                    {
                        // Allocate buffer for the decompressed chunk data
                        var buffer = new byte[chunk.UncompressedLength];

                        var bytesWritten = await cdnClient.DownloadDepotChunkAsync(
                            depotId,
                            chunk,
                            server,
                            buffer,
                            depotKey
                        );

                        await fs.WriteAsync(buffer.AsMemory(0, bytesWritten));
                        downloadedBytes += chunk.UncompressedLength;

                        var percent = totalBytes > 0 ? (double)downloadedBytes / totalBytes * 100.0 : 0;
                        JsonOutput.Progress(Math.Round(percent, 1), downloadedBytes, totalBytes);
                    }

                    completedFiles++;
                }

                JsonOutput.Info($"Download complete: {completedFiles}/{totalFiles} files");
                JsonOutput.Done(true);
                return 0;
            }
            finally
            {
                cts.Cancel();
            }
        }
        catch (AuthRequiredException ex)
        {
            JsonOutput.Error("AUTH_REQUIRED", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
        catch (Exception ex)
        {
            JsonOutput.Error("DOWNLOAD_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
