/**
 * Progress event from the Tauri downgrade-progress event stream.
 * Mirrors the Rust DowngradeProgress enum.
 */
export interface DowngradeProgressEvent {
  phase: "comparing" | "downloading" | "applying" | "complete" | "error";
  percent?: number;
  bytes_downloaded?: number;
  bytes_total?: number;
  message?: string;
}
