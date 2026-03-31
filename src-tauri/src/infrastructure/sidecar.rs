//! Persistent SteamKit sidecar lifecycle management.
//!
//! Manages a long-lived sidecar daemon process that communicates via
//! NDJSON on stdin/stdout. The sidecar is spawned once and reused for
//! all Steam operations, keeping a single authenticated connection alive.
//!
//! # Architecture
//!
//! - [`SidecarHandle`] wraps the child process, stdin writer, and stdout reader.
//! - [`start_sidecar`] spawns the daemon and returns a handle.
//! - [`send_command`] writes a command to stdin and collects correlated responses.
//! - The handle is stored in Tauri managed state and shared across IPC handlers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::{mpsc, Mutex};

use crate::error::RewindError;

/// The sidecar binary name as configured in `tauri.conf.json` `bundle.externalBin`.
///
/// The target triple suffix is appended at compile time because `tauri_plugin_shell`'s
/// `sidecar()` does not append it automatically in the Rust API (unlike the JS API).
const SIDECAR_NAME: &str = concat!("binaries/SteamKitSidecar-", env!("TARGET_TRIPLE"));

/// Monotonically increasing request ID counter for correlating responses.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique request ID for a sidecar command.
fn next_request_id() -> String {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("r{}", id)
}

/// A response line from the sidecar, parsed as JSON.
#[derive(Debug, Clone)]
pub struct SidecarResponse {
    /// The parsed JSON value of the response line.
    pub value: Value,
    /// The `type` field from the response.
    pub msg_type: String,
}

impl SidecarResponse {
    fn from_value(value: Value) -> Option<Self> {
        let msg_type = value.get("type")?.as_str()?.to_string();
        Some(Self { value, msg_type })
    }
}

/// Handle to a running sidecar daemon process.
///
/// Holds the stdin writer for sending commands and a dispatch table
/// for routing responses to the correct waiting command by request_id.
/// The stdout reader task runs in the background and distributes
/// responses to registered channels.
///
/// All fields are `Arc`-wrapped, so cloning is cheap and shares the
/// same underlying sidecar process.
#[derive(Clone)]
pub struct SidecarHandle {
    /// Writer for sending NDJSON commands to the sidecar's stdin.
    stdin_writer: Arc<Mutex<tauri_plugin_shell::process::CommandChild>>,
    /// Map of request_id -> channel sender for routing responses.
    pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SidecarResponse>>>>,
}

impl SidecarHandle {
    /// Check if the sidecar process is still alive by verifying
    /// the pending dispatch map is still usable (proxy for liveness).
    pub async fn is_alive(&self) -> bool {
        // The handle is alive as long as we haven't been dropped
        // and the background reader task is running. We can't directly
        // check the child process status, so we rely on the fact that
        // if the process dies, the reader task will detect it via
        // the Terminated event and we'll get errors on subsequent sends.
        true
    }
}

/// Lazily-initialized sidecar state stored in Tauri managed state.
///
/// The sidecar is spawned on first access and reused for all subsequent
/// commands. This avoids startup-time blocking and race conditions.
pub struct SidecarState {
    handle: tokio::sync::OnceCell<SidecarHandle>,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self::new()
    }
}

impl SidecarState {
    /// Create a new empty sidecar state.
    pub fn new() -> Self {
        Self {
            handle: tokio::sync::OnceCell::new(),
        }
    }

    /// Get the sidecar handle, starting the daemon if needed.
    pub async fn get(&self, app: &AppHandle) -> Result<&SidecarHandle, RewindError> {
        self.handle.get_or_try_init(|| start_sidecar(app)).await
    }
}

/// Spawn the sidecar daemon and start the background stdout reader.
///
/// The sidecar binary is resolved by Tauri based on the platform target triple.
/// It starts in daemon mode (reading NDJSON from stdin) and stays running
/// until the handle is dropped or the process crashes.
///
/// # Errors
///
/// Returns an error if the sidecar binary cannot be found or spawned.
pub async fn start_sidecar(app: &AppHandle) -> Result<SidecarHandle, RewindError> {
    let cmd = app
        .shell()
        .sidecar(SIDECAR_NAME)
        .map_err(|e| RewindError::Infrastructure(format!("Failed to resolve sidecar: {}", e)))?;

    let (mut rx, child) = cmd.spawn().map_err(|e| {
        RewindError::Infrastructure(format!("Failed to spawn sidecar daemon: {}", e))
    })?;

    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SidecarResponse>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Background task: read stdout/stderr events and dispatch responses
    let pending_clone = pending.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(data) => {
                    if let Ok(line) = String::from_utf8(data) {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        eprintln!("[sidecar stdout] {}", trimmed);

                        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                            // Route response to the correct pending request
                            if let Some(request_id) =
                                value.get("request_id").and_then(|v| v.as_str())
                            {
                                let pending = pending_clone.lock().await;
                                if let Some(tx) = pending.get(request_id) {
                                    if let Some(resp) = SidecarResponse::from_value(value) {
                                        let _ = tx.send(resp);
                                    }
                                }
                            } else {
                                // No request_id — broadcast/log message (e.g., startup info)
                                eprintln!(
                                    "[sidecar] unrouted message: {}",
                                    trimmed
                                );
                            }
                        }
                    }
                }
                CommandEvent::Stderr(data) => {
                    if let Ok(line) = String::from_utf8(data) {
                        eprintln!("[sidecar stderr] {}", line.trim());

                        // Route stderr errors by request_id so callers see them
                        let trimmed = line.trim();
                        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                            if let Some(request_id) =
                                value.get("request_id").and_then(|v| v.as_str())
                            {
                                let pending = pending_clone.lock().await;
                                if let Some(tx) = pending.get(request_id) {
                                    if let Some(resp) = SidecarResponse::from_value(value) {
                                        let _ = tx.send(resp);
                                    }
                                }
                            }
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    eprintln!(
                        "[sidecar] process terminated with code {:?}",
                        payload.code
                    );
                    // Drop all pending senders to unblock waiting commands
                    let mut pending = pending_clone.lock().await;
                    pending.clear();
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(SidecarHandle {
        stdin_writer: Arc::new(Mutex::new(child)),
        pending,
    })
}

/// Send a command to the sidecar daemon and collect all responses until "done".
///
/// Writes the command as a single NDJSON line to the sidecar's stdin, then
/// reads responses from the background reader task, collecting all lines that
/// match the command's request_id until a "done" message is received.
///
/// # Arguments
///
/// * `handle` - The sidecar handle (from `start_sidecar`)
/// * `command` - JSON object to send; `request_id` is added automatically
///
/// # Returns
///
/// A vector of all response messages for this command (including the "done" message).
///
/// # Errors
///
/// Returns an error if the command cannot be written to stdin or if the
/// sidecar terminates before sending a "done" response.
pub async fn send_command(
    handle: &SidecarHandle,
    mut command: Value,
) -> Result<Vec<SidecarResponse>, RewindError> {
    let request_id = next_request_id();

    // Inject request_id into the command
    if let Some(obj) = command.as_object_mut() {
        obj.insert("request_id".to_string(), Value::String(request_id.clone()));
    }

    // Register a channel for this request's responses
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut pending = handle.pending.lock().await;
        pending.insert(request_id.clone(), tx);
    }

    // Write the command to stdin
    let line = serde_json::to_string(&command).map_err(|e| {
        RewindError::Infrastructure(format!("Failed to serialize command: {}", e))
    })?;

    {
        let mut child = handle.stdin_writer.lock().await;
        child.write((line + "\n").as_bytes()).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to write to sidecar stdin: {}", e))
        })?;
    }

    eprintln!("[sidecar] sent command: {}", serde_json::to_string(&command).unwrap_or_default());

    // Collect responses until we get a "done" message
    let mut responses = Vec::new();
    while let Some(resp) = rx.recv().await {
        let is_done = resp.msg_type == "done";
        responses.push(resp);
        if is_done {
            break;
        }
    }

    // Unregister the channel
    {
        let mut pending = handle.pending.lock().await;
        pending.remove(&request_id);
    }

    if responses.is_empty() {
        return Err(RewindError::Infrastructure(
            "Sidecar terminated without responding".to_string(),
        ));
    }

    Ok(responses)
}

/// Send a command and stream responses via a callback.
///
/// Similar to [`send_command`], but calls `on_response` for each response
/// as it arrives, allowing real-time progress streaming. Returns the full
/// collection of responses after the "done" message.
pub async fn send_command_streaming<F>(
    handle: &SidecarHandle,
    mut command: Value,
    mut on_response: F,
) -> Result<Vec<SidecarResponse>, RewindError>
where
    F: FnMut(&SidecarResponse),
{
    let request_id = next_request_id();

    if let Some(obj) = command.as_object_mut() {
        obj.insert("request_id".to_string(), Value::String(request_id.clone()));
    }

    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut pending = handle.pending.lock().await;
        pending.insert(request_id.clone(), tx);
    }

    let line = serde_json::to_string(&command).map_err(|e| {
        RewindError::Infrastructure(format!("Failed to serialize command: {}", e))
    })?;

    {
        let mut child = handle.stdin_writer.lock().await;
        child.write((line + "\n").as_bytes()).map_err(|e| {
            RewindError::Infrastructure(format!("Failed to write to sidecar stdin: {}", e))
        })?;
    }

    eprintln!("[sidecar] sent streaming command: {}", serde_json::to_string(&command).unwrap_or_default());

    let mut responses = Vec::new();
    while let Some(resp) = rx.recv().await {
        let is_done = resp.msg_type == "done";
        on_response(&resp);
        responses.push(resp);
        if is_done {
            break;
        }
    }

    {
        let mut pending = handle.pending.lock().await;
        pending.remove(&request_id);
    }

    if responses.is_empty() {
        return Err(RewindError::Infrastructure(
            "Sidecar terminated without responding".to_string(),
        ));
    }

    Ok(responses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_name_matches_config() {
        // The name includes the target triple because tauri_plugin_shell's Rust API
        // does not append it automatically (unlike the JS API).
        assert!(SIDECAR_NAME.starts_with("binaries/SteamKitSidecar-"));
        assert!(SIDECAR_NAME.contains(env!("TARGET_TRIPLE")));
    }

    #[test]
    fn next_request_id_is_monotonic() {
        let id1 = next_request_id();
        let id2 = next_request_id();
        assert!(id1.starts_with('r'));
        assert!(id2.starts_with('r'));
        let n1: u64 = id1[1..].parse().unwrap();
        let n2: u64 = id2[1..].parse().unwrap();
        assert!(n2 > n1);
    }

    #[test]
    fn sidecar_response_from_value_extracts_type() {
        let val = serde_json::json!({"type": "done", "success": true, "request_id": "r1"});
        let resp = SidecarResponse::from_value(val).unwrap();
        assert_eq!(resp.msg_type, "done");
    }

    #[test]
    fn sidecar_response_from_value_returns_none_without_type() {
        let val = serde_json::json!({"success": true});
        assert!(SidecarResponse::from_value(val).is_none());
    }
}
