//! SteamKit sidecar manifest operations.
//!
//! Higher-level operations that use the sidecar helper to spawn the SteamKit
//! sidecar and parse its JSON output. Each function handles argument construction,
//! output collection, and parsing.

use tauri::AppHandle;
use tauri::Emitter;
use tauri_plugin_shell::process::CommandEvent;

use crate::domain::auth::Credentials;
use crate::domain::downgrade::DowngradeProgress;
use crate::domain::manifest::{parse_manifest_json, parse_manifest_list, DepotManifest, ManifestListEntry};
use crate::error::RewindError;

use super::sidecar::spawn_sidecar;

/// Extract a human-readable error message from sidecar NDJSON stderr.
///
/// The sidecar emits errors as JSON lines like:
/// `{"type":"error","code":"AUTH_ERROR","message":"Authentication failed with result RateLimitExceeded."}`
///
/// This function parses each line and returns the last `message` field found,
/// falling back to the raw text if parsing fails.
fn extract_sidecar_error(stderr: &str) -> String {
    let mut last_message: Option<String> = None;
    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                last_message = Some(msg.to_string());
            }
        }
    }
    last_message.unwrap_or_else(|| stderr.trim().to_string())
}

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
                        extract_sidecar_error(&stderr_buffer)
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
                        extract_sidecar_error(&stderr_buffer)
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

/// Fetch manifest metadata for a specific depot manifest.
///
/// Spawns the SteamKit sidecar with the `get-manifest` command to download
/// and parse manifest metadata (file listings with SHA hashes, sizes, chunks).
///
/// # Arguments
///
/// * `app` - Tauri application handle
/// * `app_id` - Steam application ID
/// * `depot_id` - Steam depot ID
/// * `manifest_id` - Target manifest ID to fetch
/// * `credentials` - Steam credentials
pub async fn get_manifest(
    app: &AppHandle,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    credentials: &Credentials,
) -> Result<DepotManifest, RewindError> {
    let mut args = vec![
        "get-manifest".to_string(),
        "--username".to_string(),
        credentials.username.clone(),
        "--password".to_string(),
        credentials.password.clone(),
        "--app".to_string(),
        app_id.to_string(),
        "--depot".to_string(),
        depot_id.to_string(),
        "--manifest".to_string(),
        manifest_id.to_string(),
    ];

    if let Some(ref code) = credentials.guard_code {
        args.push("--guard-code".to_string());
        args.push(code.clone());
    }

    let (mut rx, _child) = spawn_sidecar(app, args).map_err(|e| {
        RewindError::Infrastructure(format!("Failed to spawn SteamKit sidecar: {}", e))
    })?;

    let mut stdout_buffer = String::new();
    let mut stderr_buffer = String::new();

    eprintln!(
        "[sidecar get-manifest] fetching manifest {} for depot {}",
        manifest_id, depot_id
    );
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar get-manifest stdout] {}", line.trim());
                    stdout_buffer.push_str(&line);
                }
            }
            CommandEvent::Stderr(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar get-manifest stderr] {}", line.trim());
                    stderr_buffer.push_str(&line);
                }
            }
            CommandEvent::Terminated(payload) => {
                eprintln!(
                    "[sidecar get-manifest] terminated with code {:?}",
                    payload.code
                );
                if payload.code != Some(0) {
                    let detail = if stderr_buffer.is_empty() {
                        format!(
                            "SteamKit sidecar exited with code {:?}",
                            payload.code
                        )
                    } else {
                        extract_sidecar_error(&stderr_buffer)
                    };
                    return Err(RewindError::Infrastructure(detail));
                }
                break;
            }
            _ => {}
        }
    }

    parse_manifest_json(&stdout_buffer).map_err(|e| {
        RewindError::Infrastructure(format!("Failed to parse manifest output: {}", e))
    })
}

/// Download depot files using the SteamKit sidecar.
///
/// Spawns the sidecar `download` command with the specified filelist and target
/// manifest ID. Streams progress events to the frontend via Tauri event emission
/// on the `downgrade-progress` channel.
///
/// # Arguments
///
/// * `app` - Tauri application handle (also used for event emission)
/// * `app_id` - Steam application ID
/// * `depot_id` - Steam depot ID
/// * `manifest_id` - Target manifest ID to download from
/// * `output_dir` - Directory to write downloaded files to
/// * `filelist_path` - Path to a file containing newline-separated file names
/// * `credentials` - Steam credentials
pub async fn download(
    app: &AppHandle,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &str,
    filelist_path: &str,
    credentials: &Credentials,
) -> Result<(), RewindError> {
    let mut args = vec![
        "download".to_string(),
        "--username".to_string(),
        credentials.username.clone(),
        "--password".to_string(),
        credentials.password.clone(),
        "--app".to_string(),
        app_id.to_string(),
        "--depot".to_string(),
        depot_id.to_string(),
        "--manifest".to_string(),
        manifest_id.to_string(),
        "--dir".to_string(),
        output_dir.to_string(),
        "--filelist".to_string(),
        filelist_path.to_string(),
    ];

    if let Some(ref code) = credentials.guard_code {
        args.push("--guard-code".to_string());
        args.push(code.clone());
    }

    let (mut rx, _child) = spawn_sidecar(app, args).map_err(|e| {
        RewindError::Infrastructure(format!("Failed to spawn SteamKit sidecar: {}", e))
    })?;

    let mut stderr_buffer = String::new();

    eprintln!(
        "[sidecar download] downloading manifest {} for depot {}",
        manifest_id, depot_id
    );
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    let trimmed = line.trim();
                    eprintln!("[sidecar download stdout] {}", trimmed);

                    // Try to parse progress events and forward to frontend
                    if let Ok(progress) =
                        serde_json::from_str::<serde_json::Value>(trimmed)
                    {
                        if progress.get("type").and_then(|t| t.as_str()) == Some("progress") {
                            let percent = progress
                                .get("percent")
                                .and_then(|p| p.as_f64())
                                .unwrap_or(0.0);
                            let bytes_downloaded = progress
                                .get("bytes_downloaded")
                                .and_then(|b| b.as_u64())
                                .unwrap_or(0);
                            let bytes_total = progress
                                .get("bytes_total")
                                .and_then(|b| b.as_u64())
                                .unwrap_or(0);

                            let _ = app.emit(
                                "downgrade-progress",
                                DowngradeProgress::Downloading {
                                    percent,
                                    bytes_downloaded,
                                    bytes_total,
                                },
                            );
                        }
                    }
                }
            }
            CommandEvent::Stderr(data) => {
                if let Ok(line) = String::from_utf8(data) {
                    eprintln!("[sidecar download stderr] {}", line.trim());
                    stderr_buffer.push_str(&line);
                }
            }
            CommandEvent::Terminated(payload) => {
                eprintln!(
                    "[sidecar download] terminated with code {:?}",
                    payload.code
                );
                if payload.code != Some(0) {
                    let detail = if stderr_buffer.is_empty() {
                        format!(
                            "SteamKit sidecar download exited with code {:?}",
                            payload.code
                        )
                    } else {
                        extract_sidecar_error(&stderr_buffer)
                    };
                    return Err(RewindError::Infrastructure(detail));
                }
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
