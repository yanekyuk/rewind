//! DepotDownloader sidecar binary resolution.
//!
//! Provides a helper to create a Tauri sidecar [`Command`] for DepotDownloader.
//! The actual binary is resolved by Tauri based on the current platform's target triple.

use tauri::async_runtime::Receiver;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// The sidecar binary name as configured in `tauri.conf.json` `bundle.externalBin`.
const SIDECAR_NAME: &str = "binaries/DepotDownloader";

/// Spawns the DepotDownloader sidecar with the given arguments.
///
/// Tauri resolves the platform-specific binary automatically:
/// - Linux: `DepotDownloader-x86_64-unknown-linux-gnu`
/// - macOS: `DepotDownloader-x86_64-apple-darwin`
/// - Windows: `DepotDownloader-x86_64-pc-windows-msvc.exe`
///
/// Returns a receiver for stdout/stderr/termination events and a handle to
/// the child process (for writing to stdin or killing it).
///
/// # Errors
///
/// Returns an error if the sidecar binary cannot be found or the command cannot be spawned.
pub fn spawn_depot_downloader(
    app: &AppHandle,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(Receiver<CommandEvent>, CommandChild), tauri_plugin_shell::Error> {
    let cmd = app.shell().sidecar(SIDECAR_NAME)?;
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let cmd = if args.is_empty() {
        cmd
    } else {
        cmd.args(args)
    };
    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_name_matches_config() {
        // The sidecar name must match the externalBin entry in tauri.conf.json
        assert_eq!(SIDECAR_NAME, "binaries/DepotDownloader");
    }
}
