//! Authentication types for Steam credential handling.
//!
//! Defines the credential structure passed to DepotDownloader.
//! Credentials are held in-memory only — never persisted to disk.

use serde::{Deserialize, Serialize};

/// Steam credentials for DepotDownloader authentication.
///
/// Contains the username, password, and an optional Steam Guard 2FA code.
/// These are passed as CLI arguments to DepotDownloader and must never be
/// persisted to disk by Rewind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// Optional Steam Guard code (email or mobile authenticator).
    /// Only required when 2FA is enabled on the account.
    pub guard_code: Option<String>,
}

impl Credentials {
    /// Build the CLI arguments for DepotDownloader authentication.
    ///
    /// Always includes `-username`, `-password`, and `-remember-password`.
    /// If a Steam Guard code is set, it is **not** included here — it is
    /// written to DepotDownloader's stdin when the process prompts for it.
    /// See [`infrastructure::sidecar::write_guard_code`] and
    /// [`infrastructure::sidecar::is_guard_prompt`].
    pub fn to_depot_args(&self) -> Vec<String> {
        vec![
            "-username".to_string(),
            self.username.clone(),
            "-password".to_string(),
            self.password.clone(),
            "-remember-password".to_string(),
        ]
    }

    /// Validate that required fields are non-empty.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.username.trim().is_empty() {
            return Err("Username must not be empty");
        }
        if self.password.is_empty() {
            return Err("Password must not be empty");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_depot_args_includes_username_password_remember() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        let args = creds.to_depot_args();
        assert_eq!(
            args,
            vec![
                "-username",
                "testuser",
                "-password",
                "testpass",
                "-remember-password",
            ]
        );
    }

    #[test]
    fn to_depot_args_does_not_include_guard_code() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: Some("ABC123".to_string()),
        };
        let args = creds.to_depot_args();
        // Guard code is provided via stdin, not CLI args
        assert!(!args.contains(&"-2fa".to_string()));
        assert!(!args.contains(&"ABC123".to_string()));
    }

    #[test]
    fn validate_rejects_empty_username() {
        let creds = Credentials {
            username: "  ".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        assert_eq!(creds.validate(), Err("Username must not be empty"));
    }

    #[test]
    fn validate_rejects_empty_password() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "".to_string(),
            guard_code: None,
        };
        assert_eq!(creds.validate(), Err("Password must not be empty"));
    }

    #[test]
    fn validate_accepts_valid_credentials() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        assert!(creds.validate().is_ok());
    }

    #[test]
    fn validate_accepts_credentials_with_guard_code() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: Some("ABC123".to_string()),
        };
        assert!(creds.validate().is_ok());
    }
}
