/**
 * Information about a single installed depot, mirroring the Rust DepotInfo struct.
 */
export interface DepotInfo {
  depot_id: string;
  manifest: string;
  size: string;
}

/**
 * Information about a depot as reported by Steam's PICS API,
 * mirroring the Rust SteamDepotInfo struct.
 * Returned by the `list_depots` Tauri IPC command.
 */
export interface SteamDepotInfo {
  depot_id: string;
  name: string | null;
  max_size: number | null;
  dlc_app_id: string | null;
}

/**
 * Information about a single installed game, mirroring the Rust GameInfo struct.
 * This is the shape returned by the `list_games` Tauri IPC command.
 */
export interface GameInfo {
  appid: string;
  name: string;
  buildid: string;
  installdir: string;
  depots: DepotInfo[];
  install_path: string;
  state_flags: number;
  update_pending: boolean;
  target_build_id: string | null;
  bytes_to_download: string | null;
  size_on_disk: string;
  last_updated: string | null;
}
