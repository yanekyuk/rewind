//! SteamKit sidecar binary resolution.
//!
//! Provides a helper to create a Tauri sidecar [`Command`] for the SteamKit binary.
//! The actual binary is resolved by Tauri based on the current platform's target triple.
//! The sidecar handles Steam authentication natively and communicates via
//! newline-delimited JSON (NDJSON) on stdout.

use tauri::async_runtime::Receiver;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// The sidecar binary name as configured in `tauri.conf.json` `bundle.externalBin`.
///
/// The target triple suffix is appended at compile time because `tauri_plugin_shell`'s
/// `sidecar()` does not append it automatically in the Rust API (unlike the JS API).
const SIDECAR_NAME: &str = concat!("binaries/SteamKitSidecar-", env!("TARGET_TRIPLE"));

/// Spawns the SteamKit sidecar with the given arguments.
///
/// Tauri resolves the platform-specific binary automatically:
/// - Linux: `SteamKit-x86_64-unknown-linux-gnu`
/// - macOS: `SteamKit-x86_64-apple-darwin`
/// - Windows: `SteamKit-x86_64-pc-windows-msvc.exe`
///
/// Returns a receiver for stdout/stderr/termination events and a handle to
/// the child process.
///
/// # Errors
///
/// Returns an error if the sidecar binary cannot be found or the command cannot be spawned.
pub fn spawn_sidecar(
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
        // The name includes the target triple because tauri_plugin_shell's Rust API
        // does not append it automatically (unlike the JS API).
        assert!(SIDECAR_NAME.starts_with("binaries/SteamKitSidecar-"));
        assert!(SIDECAR_NAME.contains(env!("TARGET_TRIPLE")));
    }
}
