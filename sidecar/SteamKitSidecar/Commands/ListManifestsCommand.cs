using SteamKit2;

namespace SteamKitSidecar.Commands;

/// <summary>
/// Handles the 'list-manifests' command: enumerate manifest IDs for a depot.
/// Uses PICSGetProductInfo to retrieve the current public manifest and any
/// branch-specific manifests from the depot's PICS data.
///
/// In daemon mode, receives a shared SteamSession that is already connected
/// and authenticated.
/// </summary>
public static class ListManifestsCommand
{
    public static async Task RunAsync(SteamSession session, uint appId, uint depotId, string? requestId)
    {
        try
        {
            JsonOutput.Info($"Requesting product info for app {appId}...", requestId);

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
                            JsonOutput.Warn($"Depot {depotId} not found in app {appId} PICS data", requestId);
                            continue;
                        }

                        // Collect branch metadata (timeupdated, pwdrequired) from
                        // the app-level "branches" section under depots.
                        var branchMeta = new Dictionary<string, (ulong? timeUpdated, bool pwdRequired)>();
                        var branchesSection = depots["branches"];
                        if (branchesSection != KeyValue.Invalid)
                        {
                            foreach (var branchKv in branchesSection.Children)
                            {
                                var branchName = branchKv.Name;
                                if (branchName == null) continue;

                                ulong? timeUpdated = null;
                                var timeVal = branchKv["timeupdated"]?.Value;
                                if (timeVal != null && ulong.TryParse(timeVal, out var ts))
                                    timeUpdated = ts;

                                var pwdVal = branchKv["pwdrequired"]?.Value;
                                var pwdRequired = pwdVal == "1";

                                branchMeta[branchName] = (timeUpdated, pwdRequired);
                            }
                        }

                        var manifestsSection = depot["manifests"];
                        if (manifestsSection != KeyValue.Invalid)
                        {
                            foreach (var branch in manifestsSection.Children)
                            {
                                // Branch may have Value directly or a nested "gid" child
                                var manifestId = branch.Value ?? branch["gid"]?.Value;
                                if (manifestId != null)
                                {
                                    var branchName = branch.Name ?? "unknown";
                                    var item = new ManifestListItem
                                    {
                                        Id = manifestId,
                                        Branch = branchName,
                                    };

                                    if (branchMeta.TryGetValue(branchName, out var meta))
                                    {
                                        item.TimeUpdated = meta.timeUpdated;
                                        item.PwdRequired = meta.pwdRequired;
                                    }

                                    manifests.Add(item);
                                }
                            }
                        }
                        else
                        {
                            JsonOutput.Warn($"No 'manifests' section found in depot {depotId}", requestId);
                        }
                    }
                }
            }

            JsonOutput.ManifestList(manifests, requestId);
            JsonOutput.Done(true, requestId);
        }
        catch (AuthRequiredException ex)
        {
            JsonOutput.Error("AUTH_REQUIRED", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
        catch (Exception ex)
        {
            JsonOutput.Error("MANIFEST_LIST_ERROR", ex.Message, requestId);
            JsonOutput.Done(false, requestId);
        }
    }
}
