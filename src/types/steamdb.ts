/**
 * A manifest entry extracted from SteamDB's depot manifests page.
 * These are historical manifests that may not be available via the SteamKit PICS API.
 */
export interface SteamDBManifest {
  manifest_id: string;
  /** Date string from SteamDB (e.g., "15 March 2026 - 14:30:00 UTC"). */
  date?: string;
  /** Branch label if the manifest belongs to a specific branch. */
  branch?: string;
}
