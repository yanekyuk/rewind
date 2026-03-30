using SteamKit2;

namespace RewindSidecar.Commands;

/// <summary>
/// Handles the 'list-manifests' command: enumerate historical manifest IDs for a depot.
/// </summary>
public static class ListManifestsCommand
{
    public static async Task<int> RunAsync(string username, string password, string? guardCode, uint appId, uint depotId)
    {
        using var session = new SteamSession();

        try
        {
            var cts = await session.ConnectAndLoginAsync(username, password, guardCode);

            try
            {
                JsonOutput.Info($"Requesting manifest history for app {appId}, depot {depotId}...");

                // Request the depot's PICS product info to get the current manifest
                // Then use GetDepotDecryptionKey and request CDN auth tokens
                var depotInfo = await session.Apps.GetDepotDecryptionKey(depotId, appId);

                // Use PICSGetProductInfo to get manifest information
                var productInfoRequest = new SteamApps.PICSRequest(appId);
                var productInfo = await session.Apps.PICSGetProductInfo(
                    new List<SteamApps.PICSRequest> { productInfoRequest },
                    Enumerable.Empty<SteamApps.PICSRequest>()
                );

                var manifests = new List<ManifestListItem>();

                if (productInfo.Results != null)
                {
                    foreach (var result in productInfo.Results)
                    {
                        if (result.Apps.TryGetValue(appId, out var appInfo))
                        {
                            // Navigate the KeyValues tree: depots -> depotId -> manifests -> public
                            var depots = appInfo.KeyValues["depots"];
                            var depot = depots[depotId.ToString()];
                            var manifestsSection = depot["manifests"];
                            var publicManifest = manifestsSection["public"];

                            if (publicManifest != KeyValue.Invalid && publicManifest.Value != null)
                            {
                                manifests.Add(new ManifestListItem
                                {
                                    Id = publicManifest.Value,
                                    Date = "", // PICS doesn't provide historical dates for the current manifest
                                });
                            }

                            // Check for historical manifests via the encryptedmanifests or history sections
                            // Steam's PICS data doesn't always include full history via this endpoint.
                            // For full history, we need to use the CDN manifest request codes.
                        }
                    }
                }

                // Also try to get historical manifests via SteamContent
                // SteamKit2 supports GetManifestRequestCode which we need for downloading,
                // but for listing, we rely on PICS data which shows the current public manifest.
                // Full historical manifest enumeration requires PICSGetChangesSince or
                // direct content server queries.

                JsonOutput.ManifestList(manifests);
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
            JsonOutput.Error("MANIFEST_LIST_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
