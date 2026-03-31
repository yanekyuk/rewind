using SteamKit2;
using SteamKit2.CDN;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'download' command: download depot files for a specific manifest via Steam CDN.
/// Supports optional filelist filtering.
///
/// In daemon mode, receives a shared SteamSession that is already connected
/// and authenticated.
/// </summary>
public static class DownloadCommand
{
    public static async Task RunAsync(
        SteamSession session,
        uint appId, uint depotId, ulong manifestId,
        string outputDir, string? filelistPath, string? requestId)
    {
        try
        {
            JsonOutput.Info($"Downloading depot {depotId} manifest {manifestId}...", requestId);

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

            JsonOutput.Info($"Downloading {totalFiles} files ({totalBytes} bytes)...", requestId);

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
                    JsonOutput.Progress(Math.Round(percent, 1), downloadedBytes, totalBytes, requestId);
                }

                completedFiles++;
            }

            JsonOutput.Info($"Download complete: {completedFiles}/{totalFiles} files", requestId);
            JsonOutput.Done(true, requestId);
        }
        catch (AuthRequiredException ex)
        {
            JsonOutput.Error("AUTH_REQUIRED", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
        catch (Exception ex)
        {
            JsonOutput.Error("DOWNLOAD_ERROR", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
    }
}
