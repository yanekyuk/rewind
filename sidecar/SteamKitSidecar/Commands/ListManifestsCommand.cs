using SteamKit2;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'list-manifests' command: enumerate manifest IDs for a depot.
/// Uses PICSGetProductInfo to retrieve the current public manifest and any
/// branch-specific manifests from the depot's PICS data.
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
                JsonOutput.Info($"Requesting product info for app {appId}...");

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
                            var depots = appInfo.KeyValues["depots"];
                            var depot = depots[depotId.ToString()];

                            if (depot == KeyValue.Invalid)
                            {
                                JsonOutput.Warn($"Depot {depotId} not found in app {appId} PICS data");
                                continue;
                            }

                            // Debug: dump depot keys to see structure
                            var depotKeys = string.Join(", ", depot.Children.Select(c => c.Name));
                            JsonOutput.Info($"Depot {depotId} keys: [{depotKeys}]");

                            var manifestsSection = depot["manifests"];
                            if (manifestsSection != KeyValue.Invalid)
                            {
                                var childDescs = manifestsSection.Children.Select(c =>
                                {
                                    if (c.Value != null)
                                        return $"{c.Name}={c.Value}";
                                    var nested = string.Join(",", c.Children.Select(cc => $"{cc.Name}={cc.Value}"));
                                    return $"{c.Name}=({nested})";
                                });
                                JsonOutput.Info($"manifests children: [{string.Join(", ", childDescs)}]");

                                foreach (var branch in manifestsSection.Children)
                                {
                                    // Branch may have Value directly or a nested "gid" child
                                    var manifestId = branch.Value ?? branch["gid"]?.Value;
                                    if (manifestId != null)
                                    {
                                        manifests.Add(new ManifestListItem
                                        {
                                            Id = manifestId,
                                            Date = branch.Name ?? "unknown",
                                        });
                                    }
                                }
                            }
                            else
                            {
                                JsonOutput.Warn($"No 'manifests' section found in depot {depotId}");
                            }
                        }
                    }
                }

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
