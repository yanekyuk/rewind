//! SteamKit sidecar manifest operations.
//!
//! Higher-level operations that use the sidecar helper to spawn the SteamKit
//! sidecar and parse its JSON output. Each function handles argument construction,
//! output collection, and parsing.

use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;

use crate::domain::auth::Credentials;
use crate::domain::manifest::{parse_manifest_list, ManifestListEntry};
use crate::error::RewindError;

use super::sidecar::spawn_sidecar;

/// List available manifests for a depot using the SteamKit sidecar.
///
/// Spawns the SteamKit sidecar with stored credentials to fetch the manifest
/// history for the specified app and depot. Collects stdout (newline-delimited JSON)
/// and parses manifest entries from the output.
///
/// # Arguments
///
/// * `app` - Tauri application handle (needed to resolve the sidecar binary)
/// * `app_id` - Steam application ID
/// * `depot_id` - Steam depot ID
/// * `credentials` - Steam credentials (username, password, optional 2FA code)
///
/// # Errors
///
/// Returns `RewindError::Infrastructure` if the sidecar cannot be spawned
/// or if the process exits with an error.
pub async fn list_manifests(
    app: &AppHandle,
    app_id: &str,
    depot_id: &str,
    credentials: &Credentials,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    // Credentials are serialized and sent to the sidecar via stdin.
    // For now, the credentials are serialized but sent via a separate channel (future implementation).
    let _creds_json = serde_json::to_string(credentials)
        .map_err(|e| RewindError::Infrastructure(format!("Failed to serialize credentials: {}", e)))?;

    let args = vec![
        "list-manifests".to_string(),
        app_id.to_string(),
        depot_id.to_string(),
    ];

    let (mut rx, _child) =
        spawn_sidecar(app, args).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to spawn SteamKit sidecar: {}", e))
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
                            "SteamKit sidecar exited with code {:?}",
                            payload.code
                        )
                    } else {
                        format!(
                            "SteamKit sidecar exited with code {:?}: {}",
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

    // Parse newline-delimited JSON from stdout
    let entries = parse_manifest_list(&stdout_buffer);
    Ok(entries)
}
