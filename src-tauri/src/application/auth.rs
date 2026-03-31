//! Credential store for the application session.
//!
//! The username is persisted to a plaintext file on disk. The password is
//! stored in the OS keychain via the `keyring` crate. On startup, both are
//! loaded to restore the previous session without requiring re-authentication.
//! The sidecar manages its own session token separately.

use std::sync::Mutex;

use crate::domain::auth::Credentials;

const KEYRING_SERVICE: &str = "rewind";

fn username_file() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("rewind").join("username"))
}

/// Save the username to disk so the app can restore the session on restart.
pub fn save_username(username: &str) {
    if let Some(path) = username_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&path, username) {
            Ok(_) => eprintln!("[auth] saved username to {}", path.display()),
            Err(e) => eprintln!("[auth] failed to save username: {}", e),
        }
    }
}

/// Load the saved username, if any.
pub fn load_username() -> Option<String> {
    let path = username_file()?;
    match std::fs::read_to_string(&path) {
        Ok(u) if !u.trim().is_empty() => {
            let username = u.trim().to_string();
            eprintln!("[auth] loaded saved username: {}", username);
            Some(username)
        }
        _ => {
            eprintln!("[auth] no saved username found");
            None
        }
    }
}

/// Remove saved username from disk.
pub fn clear_saved_username() {
    if let Some(path) = username_file() {
        let _ = std::fs::remove_file(&path);
    }
    eprintln!("[auth] cleared saved username");
}

/// Save the password to the OS keychain.
///
/// Uses the `keyring` crate for cross-platform keychain access:
/// - Linux: libsecret / kwallet
/// - macOS: Keychain
/// - Windows: Credential Manager (DPAPI)
///
/// Fails silently with a log message if the keychain is unavailable.
pub fn save_to_keychain(username: &str, password: &str) {
    match keyring::Entry::new(KEYRING_SERVICE, username) {
        Ok(entry) => match entry.set_password(password) {
            Ok(_) => eprintln!("[auth] saved password to OS keychain for {}", username),
            Err(e) => eprintln!("[auth] failed to save password to keychain: {}", e),
        },
        Err(e) => eprintln!("[auth] failed to create keyring entry: {}", e),
    }
}

/// Load the password from the OS keychain for the given username.
///
/// Returns `None` if the keychain is unavailable or no entry exists.
pub fn load_from_keychain(username: &str) -> Option<String> {
    match keyring::Entry::new(KEYRING_SERVICE, username) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => {
                eprintln!("[auth] loaded password from OS keychain for {}", username);
                Some(password)
            }
            Err(keyring::Error::NoEntry) => {
                eprintln!("[auth] no keychain entry found for {}", username);
                None
            }
            Err(e) => {
                eprintln!("[auth] failed to load password from keychain: {}", e);
                None
            }
        },
        Err(e) => {
            eprintln!("[auth] failed to create keyring entry for load: {}", e);
            None
        }
    }
}

/// Delete the password from the OS keychain for the given username.
///
/// Fails silently if the keychain is unavailable or no entry exists.
pub fn delete_from_keychain(username: &str) {
    match keyring::Entry::new(KEYRING_SERVICE, username) {
        Ok(entry) => match entry.delete_credential() {
            Ok(_) => eprintln!("[auth] deleted password from OS keychain for {}", username),
            Err(keyring::Error::NoEntry) => {
                eprintln!("[auth] no keychain entry to delete for {}", username);
            }
            Err(e) => eprintln!("[auth] failed to delete keychain entry: {}", e),
        },
        Err(e) => eprintln!("[auth] failed to create keyring entry for delete: {}", e),
    }
}

/// Thread-safe, in-memory credential store.
///
/// Managed as Tauri application state. The `Mutex` ensures safe concurrent
/// access from multiple IPC command handlers.
#[derive(Default)]
pub struct AuthStore {
    credentials: Mutex<Option<Credentials>>,
    /// Stored username from a previous session (no password).
    saved_username: Mutex<Option<String>>,
    /// Whether the store has a password (from keychain or current session).
    stored_password: Mutex<bool>,
}

impl AuthStore {
    /// Create an AuthStore, optionally pre-loaded with a saved username.
    pub fn with_saved_username(username: Option<String>) -> Self {
        Self {
            credentials: Mutex::new(None),
            saved_username: Mutex::new(username),
            stored_password: Mutex::new(false),
        }
    }

    /// Create an AuthStore pre-loaded with credentials from the OS keychain.
    ///
    /// If both username and password are available, creates full `Credentials`
    /// so that `get_or_saved()` returns usable credentials with a real password
    /// instead of the empty-password fallback.
    pub fn with_saved_credentials(
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        match (&username, &password) {
            (Some(u), Some(p)) => {
                let creds = Credentials {
                    username: u.clone(),
                    password: p.clone(),
                    guard_code: None,
                };
                Self {
                    credentials: Mutex::new(Some(creds)),
                    saved_username: Mutex::new(username),
                    stored_password: Mutex::new(true),
                }
            }
            _ => Self {
                credentials: Mutex::new(None),
                saved_username: Mutex::new(username),
                stored_password: Mutex::new(false),
            },
        }
    }

    /// Store credentials after validation.
    pub fn set(&self, credentials: Credentials) -> Result<(), &'static str> {
        credentials.validate()?;
        let mut guard = self
            .credentials
            .lock()
            .map_err(|_| "Failed to acquire credential lock")?;
        // Also update saved username
        if let Ok(mut uguard) = self.saved_username.lock() {
            *uguard = Some(credentials.username.clone());
        }
        // Mark that we have a stored password
        if let Ok(mut pguard) = self.stored_password.lock() {
            *pguard = true;
        }
        *guard = Some(credentials);
        Ok(())
    }

    /// Retrieve a clone of the stored credentials, if any.
    pub fn get(&self) -> Option<Credentials> {
        self.credentials
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Check whether credentials have been stored (full auth this session).
    pub fn is_set(&self) -> bool {
        self.credentials
            .lock()
            .ok()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Check whether a password is available (from keychain or current session).
    ///
    /// This is used by the frontend to distinguish between:
    /// - Full credentials available (show "Welcome back" UI)
    /// - Username-only saved session (show login form)
    pub fn has_stored_password(&self) -> bool {
        self.stored_password
            .lock()
            .ok()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    /// Check whether there's a saved username from a previous session.
    pub fn has_saved_session(&self) -> bool {
        self.saved_username
            .lock()
            .ok()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Get the saved or active username.
    pub fn username(&self) -> Option<String> {
        // Prefer active credentials
        if let Some(creds) = self.get() {
            return Some(creds.username);
        }
        // Fall back to saved username
        self.saved_username
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Get credentials for sidecar calls, falling back to saved username.
    ///
    /// Returns full credentials if available, or constructs a `Credentials`
    /// with an empty password if only a saved username exists. The empty
    /// password signals the sidecar to attempt session-token authentication
    /// instead of credential-based authentication.
    ///
    /// Returns `None` if neither credentials nor a saved username exist.
    pub fn get_or_saved(&self) -> Option<Credentials> {
        // Prefer full credentials (username + password set this session)
        if let Some(creds) = self.get() {
            return Some(creds);
        }
        // Fall back to saved username with empty password
        let username = self
            .saved_username
            .lock()
            .ok()
            .and_then(|guard| guard.clone())?;
        Some(Credentials {
            username,
            password: String::new(),
            guard_code: None,
        })
    }

    /// Clear stored credentials.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.credentials.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.saved_username.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.stored_password.lock() {
            *guard = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_store_has_no_credentials() {
        let store = AuthStore::default();
        assert!(!store.is_set());
        assert!(store.get().is_none());
    }

    #[test]
    fn set_and_get_credentials() {
        let store = AuthStore::default();
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        store.set(creds).unwrap();
        assert!(store.is_set());

        let retrieved = store.get().unwrap();
        assert_eq!(retrieved.username, "testuser");
        assert_eq!(retrieved.password, "testpass");
        assert!(retrieved.guard_code.is_none());
    }

    #[test]
    fn set_rejects_invalid_credentials() {
        let store = AuthStore::default();
        let creds = Credentials {
            username: "".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        assert!(store.set(creds).is_err());
        assert!(!store.is_set());
    }

    #[test]
    fn set_replaces_previous_credentials() {
        let store = AuthStore::default();
        let creds1 = Credentials {
            username: "user1".to_string(),
            password: "pass1".to_string(),
            guard_code: None,
        };
        let creds2 = Credentials {
            username: "user2".to_string(),
            password: "pass2".to_string(),
            guard_code: Some("ABC".to_string()),
        };
        store.set(creds1).unwrap();
        store.set(creds2).unwrap();

        let retrieved = store.get().unwrap();
        assert_eq!(retrieved.username, "user2");
        assert_eq!(retrieved.guard_code, Some("ABC".to_string()));
    }

    #[test]
    fn clear_removes_credentials() {
        let store = AuthStore::default();
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        store.set(creds).unwrap();
        assert!(store.is_set());

        store.clear();
        assert!(!store.is_set());
        assert!(store.get().is_none());
    }

    #[test]
    fn saved_username_session() {
        let store = AuthStore::with_saved_username(Some("saveduser".to_string()));
        assert!(!store.is_set()); // No full credentials
        assert!(store.has_saved_session());
        assert_eq!(store.username(), Some("saveduser".to_string()));
    }

    #[test]
    fn get_or_saved_returns_full_credentials_when_set() {
        let store = AuthStore::default();
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        };
        store.set(creds).unwrap();
        let result = store.get_or_saved().unwrap();
        assert_eq!(result.username, "testuser");
        assert_eq!(result.password, "testpass");
    }

    #[test]
    fn get_or_saved_returns_saved_username_with_empty_password() {
        let store = AuthStore::with_saved_username(Some("saveduser".to_string()));
        assert!(!store.is_set()); // No full credentials
        let result = store.get_or_saved().unwrap();
        assert_eq!(result.username, "saveduser");
        assert_eq!(result.password, ""); // empty — sidecar will try saved session
        assert!(result.guard_code.is_none());
    }

    #[test]
    fn get_or_saved_returns_none_when_no_credentials_or_saved_username() {
        let store = AuthStore::default();
        assert!(store.get_or_saved().is_none());
    }

    #[test]
    fn with_saved_password_populates_full_credentials() {
        let store = AuthStore::with_saved_credentials(
            Some("saveduser".to_string()),
            Some("savedpass".to_string()),
        );
        // Should have full credentials pre-loaded
        assert!(store.is_set());
        let creds = store.get().unwrap();
        assert_eq!(creds.username, "saveduser");
        assert_eq!(creds.password, "savedpass");
        assert!(creds.guard_code.is_none());
    }

    #[test]
    fn with_saved_password_but_no_username_has_no_credentials() {
        let store = AuthStore::with_saved_credentials(None, Some("savedpass".to_string()));
        assert!(!store.is_set());
        assert!(store.get().is_none());
    }

    #[test]
    fn with_saved_username_only_falls_back_to_empty_password() {
        let store =
            AuthStore::with_saved_credentials(Some("saveduser".to_string()), None);
        assert!(!store.is_set()); // No full credentials
        assert!(store.has_saved_session());
        let result = store.get_or_saved().unwrap();
        assert_eq!(result.username, "saveduser");
        assert_eq!(result.password, ""); // Falls back to empty password
    }

    #[test]
    fn has_stored_password_true_when_keychain_credentials_loaded() {
        let store = AuthStore::with_saved_credentials(
            Some("saveduser".to_string()),
            Some("savedpass".to_string()),
        );
        assert!(store.has_stored_password());
    }

    #[test]
    fn has_stored_password_false_when_only_username() {
        let store = AuthStore::with_saved_username(Some("saveduser".to_string()));
        assert!(!store.has_stored_password());
    }

    #[test]
    fn has_stored_password_false_after_clear() {
        let store = AuthStore::with_saved_credentials(
            Some("saveduser".to_string()),
            Some("savedpass".to_string()),
        );
        assert!(store.has_stored_password());
        store.clear();
        assert!(!store.has_stored_password());
    }

    #[test]
    fn has_stored_password_true_after_set_credentials() {
        let store = AuthStore::default();
        assert!(!store.has_stored_password());
        let creds = Credentials {
            username: "user".to_string(),
            password: "pass".to_string(),
            guard_code: None,
        };
        store.set(creds).unwrap();
        // has_stored_password tracks keychain-sourced credentials specifically,
        // not in-session credentials. After set(), stored_password flag should
        // reflect that this session has credentials but they weren't from keychain.
        // However, the flag is also set to true when set() is called because
        // the password will be saved to keychain by the caller.
        assert!(store.has_stored_password());
    }
}
