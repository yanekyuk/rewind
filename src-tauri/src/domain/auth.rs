//! Authentication types for Steam credential handling.
//!
//! Defines the credential structure passed to the SteamKit sidecar as JSON.
//! Credentials are held in-memory only — never persisted to disk.

use serde::{Deserialize, Serialize};

/// Steam credentials for SteamKit authentication.
///
/// Contains the username, password, and an optional Steam Guard 2FA code.
/// These are serialized as JSON and sent to the SteamKit sidecar.
/// The sidecar handles authentication natively, including 2FA.
/// Credentials must never be persisted to disk by Rewind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// Optional Steam Guard code (email or mobile authenticator).
    /// Only required when 2FA is enabled on the account.
    pub guard_code: Option<String>,
}

impl Credentials {
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
    fn credentials_serializes_to_json() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: Some("ABC123".to_string()),
        };
        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"password\":\"testpass\""));
        assert!(json.contains("\"guard_code\":\"ABC123\""));
    }

    #[test]
    fn credentials_serializes_without_guard_code() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"password\":\"testpass\""));
        assert!(json.contains("\"guard_code\":null"));
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
