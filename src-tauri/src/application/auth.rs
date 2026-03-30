//! In-memory credential store for the application session, with optional
//! OS keychain persistence via the `keyring` crate.

use std::sync::Mutex;

use crate::domain::auth::Credentials;

const KEYCHAIN_SERVICE: &str = "rewind";
const KEYCHAIN_ACCOUNT: &str = "steam-credentials";

/// Persist credentials to the OS keychain (macOS Keychain, Windows Credential
/// Manager, Linux libsecret). Stores a JSON payload so a single entry holds
/// both username and password.
///
/// Silently ignores errors so keychain unavailability never blocks the app.
pub fn save_to_keychain(credentials: &Credentials) {
    if let Ok(payload) = serde_json::to_string(credentials) {
        if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
            let _ = entry.set_password(&payload);
        }
    }
}

/// Load credentials from the OS keychain, if any were previously saved.
pub fn load_from_keychain() -> Option<Credentials> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).ok()?;
    let payload = entry.get_password().ok()?;
    serde_json::from_str(&payload).ok()
}

/// Remove saved credentials from the OS keychain.
pub fn clear_from_keychain() {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        let _ = entry.delete_credential();
    }
}

/// Thread-safe, in-memory credential store.
///
/// Managed as Tauri application state. The `Mutex` ensures safe concurrent
/// access from multiple IPC command handlers.
///
/// # Lifecycle
///
/// - Created empty when the app starts (via `Default`)
/// - Populated when the user submits credentials via `set_credentials`
/// - Read when spawning the SteamKit sidecar
/// - Dropped when the app exits (credentials are never persisted)
#[derive(Default)]
pub struct AuthStore {
    credentials: Mutex<Option<Credentials>>,
}

impl AuthStore {
    /// Store credentials after validation.
    ///
    /// Replaces any previously stored credentials.
    pub fn set(&self, credentials: Credentials) -> Result<(), &'static str> {
        credentials.validate()?;
        let mut guard = self
            .credentials
            .lock()
            .map_err(|_| "Failed to acquire credential lock")?;
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

    /// Check whether credentials have been stored.
    pub fn is_set(&self) -> bool {
        self.credentials
            .lock()
            .ok()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Clear stored credentials.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.credentials.lock() {
            *guard = None;
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
}
