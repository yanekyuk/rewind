//! SteamKit sidecar manifest and depot operations.
//!
//! Higher-level operations that send commands to the persistent sidecar daemon
//! and parse its NDJSON responses. Each function constructs a JSON command,
//! sends it via the sidecar handle, and parses the response.

use serde_json::json;
use tauri::AppHandle;
use tauri::Emitter;

use crate::domain::auth::Credentials;
use crate::domain::downgrade::DowngradeProgress;
use crate::domain::game::SteamDepotInfo;
use crate::domain::manifest::{
    parse_depot_list, parse_manifest_json, parse_manifest_list, DepotManifest, ManifestListEntry,
};
use crate::error::RewindError;

use super::sidecar::{send_command, send_command_streaming, SidecarHandle, SidecarResponse};

/// Check responses for errors and map to the appropriate `RewindError`.
///
/// Scans all responses for "error" type messages and checks the "done"
/// message for success. Auth-related error codes are mapped to specific
/// error variants so the frontend can show the appropriate UI.
fn check_responses(responses: &[SidecarResponse]) -> Result<(), RewindError> {
    for resp in responses {
        if resp.msg_type == "error" {
            let code = resp
                .value
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = resp
                .value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            if code == "AUTH_REQUIRED" {
                return Err(RewindError::AuthRequired(
                    "Session expired. Please sign in again.".to_string(),
                ));
            }
            if code == "AUTH_FAILED" || code == "AUTH_ERROR" {
                return Err(RewindError::AuthFailed(message.to_string()));
            }
            return Err(RewindError::Infrastructure(message.to_string()));
        }
    }
    if let Some(done) = responses.iter().find(|r| r.msg_type == "done") {
        if done.value.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err(RewindError::Infrastructure(
                "Sidecar command failed".to_string(),
            ));
        }
    }
    Ok(())
}

/// Convert sidecar responses to NDJSON string for existing domain parsers.
///
/// Filters out control messages (done, log, error) and serializes
/// data responses back to JSON lines. This allows reuse of the existing
/// `parse_depot_list`, `parse_manifest_list`, etc. parsers unchanged.
fn responses_to_ndjson(responses: &[SidecarResponse]) -> String {
    responses
        .iter()
        .filter(|r| r.msg_type != "done" && r.msg_type != "log" && r.msg_type != "error")
        .filter_map(|r| serde_json::to_string(&r.value).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check for a saved sidecar session (RefreshToken on disk).
///
/// Sends a `check-session` command to the sidecar daemon. If a valid
/// RefreshToken exists, the sidecar logs in silently and returns the
/// username. Returns an error if no valid session exists.
pub async fn check_session(handle: &SidecarHandle) -> Result<String, RewindError> {
    let command = json!({
        "command": "check-session",
    });

    eprintln!("[sidecar check-session] checking for saved session...");
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;

    // Extract username from auth_success response
    for resp in &responses {
        if resp.msg_type == "auth_success" {
            if let Some(username) = resp.value.get("username").and_then(|v| v.as_str()) {
                eprintln!("[sidecar check-session] found session for {}", username);
                return Ok(username.to_string());
            }
        }
    }

    Err(RewindError::AuthRequired(
        "No valid saved session found".to_string(),
    ))
}

/// Send a logout command to the sidecar daemon.
///
/// Disposes the current SteamSession. The sidecar will require a fresh
/// `login` or `check-session` before accepting further commands.
pub async fn logout(handle: &SidecarHandle) -> Result<(), RewindError> {
    let command = json!({
        "command": "logout",
    });

    eprintln!("[sidecar logout] sending logout command...");
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;
    eprintln!("[sidecar logout] logged out");
    Ok(())
}

/// Authenticate with Steam via the sidecar daemon.
///
/// Sends a `login` command to the already-running sidecar daemon.
/// On success, the sidecar's SteamSession stays authenticated for
/// all subsequent commands -- no re-auth needed. The sidecar also
/// persists a RefreshToken to disk for future silent logins.
pub async fn login(handle: &SidecarHandle, credentials: &Credentials) -> Result<(), RewindError> {
    let mut command = json!({
        "command": "login",
        "username": credentials.username,
        "password": credentials.password,
    });
    if let Some(ref code) = credentials.guard_code {
        command
            .as_object_mut()
            .unwrap()
            .insert("guard_code".to_string(), json!(code));
    }

    eprintln!("[sidecar login] sending login command...");
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;
    eprintln!("[sidecar login] authentication successful");
    Ok(())
}

/// List all depots for an app using the sidecar daemon.
///
/// Sends a `list-depots` command and parses the response into
/// depot metadata (name, max size, DLC app ID).
pub async fn list_depots(
    handle: &SidecarHandle,
    app_id: &str,
) -> Result<Vec<SteamDepotInfo>, RewindError> {
    let app_id_num: u64 = app_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid app_id: {}", app_id)))?;
    let command = json!({
        "command": "list-depots",
        "app_id": app_id_num,
    });

    eprintln!("[sidecar list-depots] listing depots for app={}", app_id);
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;
    let ndjson = responses_to_ndjson(&responses);
    Ok(parse_depot_list(&ndjson))
}

/// List available manifests for a depot using the sidecar daemon.
///
/// Sends a `list-manifests` command and parses the response into
/// manifest entries with timestamps and branch info.
pub async fn list_manifests(
    handle: &SidecarHandle,
    app_id: &str,
    depot_id: &str,
) -> Result<Vec<ManifestListEntry>, RewindError> {
    let app_id_num: u64 = app_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid app_id: {}", app_id)))?;
    let depot_id_num: u64 = depot_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid depot_id: {}", depot_id)))?;
    let command = json!({
        "command": "list-manifests",
        "app_id": app_id_num,
        "depot_id": depot_id_num,
    });

    eprintln!(
        "[sidecar list-manifests] listing manifests for app={} depot={}",
        app_id, depot_id
    );
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;
    let ndjson = responses_to_ndjson(&responses);
    Ok(parse_manifest_list(&ndjson))
}

/// Fetch manifest metadata for a specific depot manifest.
///
/// Sends a `get-manifest` command and parses the full manifest
/// including file listings with SHA hashes, sizes, and chunks.
pub async fn get_manifest(
    handle: &SidecarHandle,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
) -> Result<DepotManifest, RewindError> {
    let app_id_num: u64 = app_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid app_id: {}", app_id)))?;
    let depot_id_num: u64 = depot_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid depot_id: {}", depot_id)))?;
    let manifest_id_num: u64 = manifest_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid manifest_id: {}", manifest_id)))?;
    let command = json!({
        "command": "get-manifest",
        "app_id": app_id_num,
        "depot_id": depot_id_num,
        "manifest_id": manifest_id_num,
    });

    eprintln!(
        "[sidecar get-manifest] fetching manifest {} for depot {}",
        manifest_id, depot_id
    );
    let responses = send_command(handle, command).await?;
    check_responses(&responses)?;
    let ndjson = responses_to_ndjson(&responses);
    parse_manifest_json(&ndjson)
        .map_err(|e| RewindError::Infrastructure(format!("Failed to parse manifest output: {}", e)))
}

/// Download depot files using the sidecar daemon.
///
/// Sends a `download` command and streams progress events to the frontend
/// via Tauri event emission on the `downgrade-progress` channel.
pub async fn download(
    handle: &SidecarHandle,
    app: &AppHandle,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &str,
    filelist_path: &str,
) -> Result<(), RewindError> {
    let app_id_num: u64 = app_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid app_id: {}", app_id)))?;
    let depot_id_num: u64 = depot_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid depot_id: {}", depot_id)))?;
    let manifest_id_num: u64 = manifest_id
        .parse()
        .map_err(|_| RewindError::Infrastructure(format!("Invalid manifest_id: {}", manifest_id)))?;
    let command = json!({
        "command": "download",
        "app_id": app_id_num,
        "depot_id": depot_id_num,
        "manifest_id": manifest_id_num,
        "dir": output_dir,
        "filelist": filelist_path,
    });

    eprintln!(
        "[sidecar download] downloading manifest {} for depot {}",
        manifest_id, depot_id
    );

    let app_clone = app.clone();
    let responses = send_command_streaming(handle, command, |resp| {
        if resp.msg_type == "progress" {
            let percent = resp
                .value
                .get("percent")
                .and_then(|p| p.as_f64())
                .unwrap_or(0.0);
            let bytes_downloaded = resp
                .value
                .get("bytes_downloaded")
                .and_then(|b| b.as_u64())
                .unwrap_or(0);
            let bytes_total = resp
                .value
                .get("bytes_total")
                .and_then(|b| b.as_u64())
                .unwrap_or(0);
            let _ = app_clone.emit(
                "downgrade-progress",
                DowngradeProgress::Downloading {
                    percent,
                    bytes_downloaded,
                    bytes_total,
                },
            );
        }
    })
    .await?;

    check_responses(&responses)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_responses_detects_auth_required() {
        let resp = SidecarResponse {
            value: serde_json::json!({"type": "error", "code": "AUTH_REQUIRED", "message": "Not logged in"}),
            msg_type: "error".to_string(),
        };
        let result = check_responses(&[resp]);
        assert!(matches!(result, Err(RewindError::AuthRequired(_))));
    }

    #[test]
    fn check_responses_detects_auth_failed() {
        let resp = SidecarResponse {
            value: serde_json::json!({"type": "error", "code": "AUTH_FAILED", "message": "Bad password"}),
            msg_type: "error".to_string(),
        };
        let result = check_responses(&[resp]);
        assert!(matches!(result, Err(RewindError::AuthFailed(_))));
    }

    #[test]
    fn check_responses_detects_auth_error() {
        let resp = SidecarResponse {
            value: serde_json::json!({"type": "error", "code": "AUTH_ERROR", "message": "Rate limited"}),
            msg_type: "error".to_string(),
        };
        let result = check_responses(&[resp]);
        assert!(matches!(result, Err(RewindError::AuthFailed(_))));
    }

    #[test]
    fn check_responses_detects_infrastructure_error() {
        let resp = SidecarResponse {
            value: serde_json::json!({"type": "error", "code": "STEAM_ERROR", "message": "Connection lost"}),
            msg_type: "error".to_string(),
        };
        let result = check_responses(&[resp]);
        assert!(matches!(result, Err(RewindError::Infrastructure(_))));
    }

    #[test]
    fn check_responses_detects_done_failure() {
        let resp = SidecarResponse {
            value: serde_json::json!({"type": "done", "success": false}),
            msg_type: "done".to_string(),
        };
        let result = check_responses(&[resp]);
        assert!(result.is_err());
    }

    #[test]
    fn check_responses_passes_on_success() {
        let data = SidecarResponse {
            value: serde_json::json!({"type": "depot_list", "depots": []}),
            msg_type: "depot_list".to_string(),
        };
        let done = SidecarResponse {
            value: serde_json::json!({"type": "done", "success": true}),
            msg_type: "done".to_string(),
        };
        assert!(check_responses(&[data, done]).is_ok());
    }

    #[test]
    fn responses_to_ndjson_filters_control_messages() {
        let responses = vec![
            SidecarResponse {
                value: serde_json::json!({"type": "log", "message": "info"}),
                msg_type: "log".to_string(),
            },
            SidecarResponse {
                value: serde_json::json!({"type": "depot_list", "depots": []}),
                msg_type: "depot_list".to_string(),
            },
            SidecarResponse {
                value: serde_json::json!({"type": "done", "success": true}),
                msg_type: "done".to_string(),
            },
        ];
        let ndjson = responses_to_ndjson(&responses);
        assert!(ndjson.contains("depot_list"));
        assert!(!ndjson.contains("\"done\""));
        assert!(!ndjson.contains("\"log\""));
    }
}
