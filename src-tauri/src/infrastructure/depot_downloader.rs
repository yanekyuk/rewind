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

/// Authenticate with Steam via the SteamKit sidecar.
///
/// Spawns the sidecar `login` command which handles the full auth flow
/// including Steam Guard / phone approval. On success, the sidecar saves
/// a session token so subsequent commands can reuse it.
pub async fn login(
    app: &AppHandle,
    credentials: &Credentials,
) -> Result<(), RewindError> {
    let mut args = vec![
        "login".to_string(),
        "--username".to_string(),
        credentials.username.clone(),
        "--password".to_string(),
        credentials.password.clone(),
    ];

    if let Some(ref code) = credentials.guard_code {
        args.push("--guard-code".to_string());
        args.push(code.clone());
    }

    let (mut rx, _child) =
        spawn_sidecar(app, args).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to spawn SteamKit sidecar: {}", e))
        })?;

    let mut stderr_buffer = String::new();

    eprintln!("[sidecar login] waiting for events...");
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar login stdout] {}", line.trim());
                }
            }
            CommandEvent::Stderr(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar login stderr] {}", line.trim());
                    stderr_buffer.push_str(&line);
                }
            }
            CommandEvent::Terminated(payload) => {
                eprintln!("[sidecar login] terminated with code {:?}", payload.code);
                if payload.code != Some(0) {
                    let detail = if stderr_buffer.is_empty() {
                        "Steam authentication failed".to_string()
                    } else {
                        stderr_buffer.trim().to_string()
                    };
                    return Err(RewindError::AuthFailed(detail));
                }
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

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
    let mut args = vec![
        "list-manifests".to_string(),
        "--username".to_string(),
        credentials.username.clone(),
        "--password".to_string(),
        credentials.password.clone(),
        "--app".to_string(),
        app_id.to_string(),
        "--depot".to_string(),
        depot_id.to_string(),
    ];

    if let Some(ref code) = credentials.guard_code {
        args.push("--guard-code".to_string());
        args.push(code.clone());
    }

    let (mut rx, _child) =
        spawn_sidecar(app, args).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to spawn SteamKit sidecar: {}", e))
        })?;

    let mut stdout_buffer = String::new();
    let mut stderr_buffer = String::new();

    eprintln!("[sidecar] waiting for events...");
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar stdout] {}", line.trim());
                    stdout_buffer.push_str(&line);
                }
            }
            CommandEvent::Stderr(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar stderr] {}", line.trim());
                    stderr_buffer.push_str(&line);
                }
            }
            CommandEvent::Terminated(payload) => {
                eprintln!("[sidecar] terminated with code {:?}", payload.code);
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
