//! DepotDownloader subprocess operations.
//!
//! Higher-level operations that use the sidecar helper to spawn DepotDownloader
//! and parse its output. Each function handles argument construction, output
//! collection, and parsing.

use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;

use crate::domain::manifest::{parse_manifest_list, ManifestListEntry};
use crate::error::RewindError;

use super::sidecar::spawn_depot_downloader;

/// List available manifests for a depot using DepotDownloader.
///
/// Spawns DepotDownloader with the given credentials to fetch the manifest
/// history for the specified app and depot. Collects stdout and parses
/// manifest entries from the output.
///
/// # Arguments
///
/// * `app` - Tauri application handle (needed to resolve the sidecar binary)
/// * `app_id` - Steam application ID
/// * `depot_id` - Steam depot ID
/// * `username` - Steam username for authentication
/// * `password` - Steam password for authentication
///
/// # Errors
///
/// Returns `RewindError::Infrastructure` if the sidecar cannot be spawned
/// or if the process exits with an error.
pub async fn list_manifests(
    app: &AppHandle,
    app_id: &str,
    depot_id: &str,
    username: &str,
    password: &str,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    let args = vec![
        "-app".to_string(),
        app_id.to_string(),
        "-depot".to_string(),
        depot_id.to_string(),
        "-username".to_string(),
        username.to_string(),
        "-password".to_string(),
        password.to_string(),
        "-remember-password".to_string(),
    ];

    let (mut rx, _child) =
        spawn_depot_downloader(app, args).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to spawn DepotDownloader: {}", e))
        })?;

    let mut stdout_buffer = String::new();
    let mut stderr_buffer = String::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    stdout_buffer.push_str(&line);
                }
            }
            CommandEvent::Stderr(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    stderr_buffer.push_str(&line);
                }
            }
            CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    let detail = if stderr_buffer.is_empty() {
                        format!(
                            "DepotDownloader exited with code {:?}",
                            payload.code
                        )
                    } else {
                        format!(
                            "DepotDownloader exited with code {:?}: {}",
                            payload.code,
                            stderr_buffer.trim()
                        )
                    };
                    return Err(RewindError::Infrastructure(detail));
                }
                break;
            }
            _ => {}
        }
    }

    let entries = parse_manifest_list(&stdout_buffer);
    Ok(entries)
}
