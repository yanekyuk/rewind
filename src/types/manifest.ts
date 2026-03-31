/**
 * A single entry from a manifest listing, mirroring the Rust ManifestListEntry struct.
 * Returned by the `list_manifests` Tauri IPC command.
 */
export interface ManifestListEntry {
  manifest_id: string;
  /** Steam branch name (e.g., "public", "beta", "bleeding-edge"). */
  branch?: string;
  /** Unix timestamp of when the branch was last updated. */
  time_updated?: number;
  /** Whether the branch requires a password to access. */
  pwd_required?: boolean;
}
