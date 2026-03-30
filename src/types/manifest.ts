/**
 * A single entry from a manifest listing, mirroring the Rust ManifestListEntry struct.
 * Returned by the `list_manifests` Tauri IPC command.
 */
export interface ManifestListEntry {
  manifest_id: string;
  date: string;
}
