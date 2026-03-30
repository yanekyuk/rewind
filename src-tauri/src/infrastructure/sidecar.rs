//! DepotDownloader sidecar binary resolution.
//!
//! Provides a helper to create a Tauri sidecar [`Command`] for DepotDownloader.
//! The actual binary is resolved by Tauri based on the current platform's target triple.

use tauri::async_runtime::Receiver;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// The sidecar binary name as configured in `tauri.conf.json` `bundle.externalBin`.
///
/// The target triple suffix is appended at compile time because `tauri_plugin_shell`'s
/// `sidecar()` does not append it automatically in the Rust API (unlike the JS API).
const SIDECAR_NAME: &str = concat!("binaries/DepotDownloader-", env!("TARGET_TRIPLE"));

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

/// Stdout patterns that indicate DepotDownloader is prompting for a Steam Guard code.
const GUARD_PROMPT_PATTERNS: &[&str] = &[
    "Please enter the 2 factor auth code",
    "Please enter your Steam Guard Mobile Authenticator code",
    "Two-factor code:",
    "Enter the current code from your Steam Guard",
];

/// Check whether a stdout line from DepotDownloader is a Steam Guard 2FA prompt.
pub fn is_guard_prompt(line: &str) -> bool {
    GUARD_PROMPT_PATTERNS
        .iter()
        .any(|pattern| line.contains(pattern))
}

/// Write a Steam Guard code to DepotDownloader's stdin.
///
/// Appends a newline so DepotDownloader reads the code as a complete line.
pub fn write_guard_code(
    child: &mut CommandChild,
    code: &str,
) -> Result<(), tauri_plugin_shell::Error> {
    let input = format!("{}\n", code);
    child.write(input.as_bytes())
}

/// Build the full argument list for an authenticated DepotDownloader invocation.
///
/// Prepends authentication arguments (from [`Credentials::to_depot_args`]) to
/// the caller-supplied operation-specific arguments.
///
/// # Example
///
/// ```ignore
/// let args = build_authenticated_args(&credentials, &[
///     "-app", "3321460",
///     "-depot", "3321461",
///     "-manifest", "12345",
///     "-manifest-only",
///     "-dir", "/tmp/output",
/// ]);
/// spawn_depot_downloader(&app, args)?;
/// ```
pub fn build_authenticated_args(
    credentials: &crate::domain::auth::Credentials,
    operation_args: &[&str],
) -> Vec<String> {
    let mut args = credentials.to_depot_args();
    args.extend(operation_args.iter().map(|s| s.to_string()));
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::Credentials;

    #[test]
    fn sidecar_name_matches_config() {
        // The name includes the target triple because tauri_plugin_shell's Rust API
        // does not append it automatically (unlike the JS API).
        assert!(SIDECAR_NAME.starts_with("binaries/DepotDownloader-"));
        assert!(SIDECAR_NAME.contains(env!("TARGET_TRIPLE")));
    }

    #[test]
    fn build_authenticated_args_prepends_auth() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        let args = build_authenticated_args(&creds, &["-app", "3321460", "-manifest-only"]);
        assert_eq!(
            args,
            vec![
                "-username",
                "testuser",
                "-password",
                "testpass",
                "-remember-password",
                "-app",
                "3321460",
                "-manifest-only",
            ]
        );
    }

    #[test]
    fn is_guard_prompt_detects_email_2fa() {
        assert!(is_guard_prompt(
            "Please enter the 2 factor auth code sent to your email at t***@example.com:"
        ));
    }

    #[test]
    fn is_guard_prompt_detects_mobile_authenticator() {
        assert!(is_guard_prompt(
            "Please enter your Steam Guard Mobile Authenticator code:"
        ));
    }

    #[test]
    fn is_guard_prompt_detects_two_factor_code() {
        assert!(is_guard_prompt("Two-factor code:"));
    }

    #[test]
    fn is_guard_prompt_rejects_unrelated_output() {
        assert!(!is_guard_prompt("Downloading depot 3321461..."));
        assert!(!is_guard_prompt("Connected to Steam"));
        assert!(!is_guard_prompt(""));
    }

    #[test]
    fn build_authenticated_args_with_empty_operation_args() {
        let creds = Credentials {
            username: "user".to_string(),
            password: "pass".to_string(),
            guard_code: None,
        };
        let args = build_authenticated_args(&creds, &[]);
        assert_eq!(
            args,
            vec!["-username", "user", "-password", "pass", "-remember-password"]
        );
    }
}
