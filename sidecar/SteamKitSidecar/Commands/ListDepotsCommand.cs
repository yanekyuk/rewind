using SteamKit2;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'list-depots' command: enumerate all depots for an app.
/// Uses PICSGetProductInfo to retrieve depot metadata from the app's PICS data,
/// iterating depots.Children to list every depot (not just a single one).
/// </summary>
public static class ListDepotsCommand
{
    public static async Task<int> RunAsync(string username, string? password, string? guardCode, uint appId)
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

                var depots = new List<DepotListItem>();

                if (productInfo.Results != null)
                {
                    foreach (var result in productInfo.Results)
                    {
                        if (result.Apps.TryGetValue(appId, out var appInfo))
                        {
                            var depotsSection = appInfo.KeyValues["depots"];

                            if (depotsSection == KeyValue.Invalid)
                            {
                                JsonOutput.Warn($"No depots section found in app {appId} PICS data");
                                continue;
                            }

                            foreach (var child in depotsSection.Children)
                            {
                                // Depot entries have numeric names (depot IDs).
                                // Skip non-numeric children like "branches".
                                if (!uint.TryParse(child.Name, out var depotId))
                                    continue;

                                var name = child["name"]?.Value;
                                var dlcAppIdStr = child["dlcappid"]?.Value;
                                uint? dlcAppId = dlcAppIdStr != null && uint.TryParse(dlcAppIdStr, out var parsed)
                                    ? parsed
                                    : null;

                                // Try to get maxsize from the depot metadata
                                ulong? maxSize = null;
                                var maxSizeStr = child["maxsize"]?.Value;
                                if (maxSizeStr != null && ulong.TryParse(maxSizeStr, out var parsedSize))
                                {
                                    maxSize = parsedSize;
                                }

                                depots.Add(new DepotListItem
                                {
                                    DepotId = depotId,
                                    Name = name,
                                    MaxSize = maxSize,
                                    DlcAppId = dlcAppId,
                                });
                            }
                        }
                    }
                }

                JsonOutput.DepotList(depots);
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
            JsonOutput.Error("DEPOT_LIST_ERROR", ex.Message);
            JsonOutput.Done(false);
            return 1;
        }
    }
}
