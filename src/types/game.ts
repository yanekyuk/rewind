/**
 * Information about a single installed depot, mirroring the Rust DepotInfo struct.
 */
export interface DepotInfo {
  depot_id: string;
  manifest: string;
  size: string;
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
}
