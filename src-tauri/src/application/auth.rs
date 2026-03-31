//! In-memory credential store for the application session, with
//! OS keychain persistence (fallback to file-based storage).

use std::sync::Mutex;

use crate::domain::auth::Credentials;

const KEYCHAIN_SERVICE: &str = "rewind";
const KEYCHAIN_ACCOUNT: &str = "steam-credentials";

fn credentials_file() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("rewind").join("credentials.json"))
}

/// Persist credentials. Tries OS keychain first, falls back to file.
pub fn save_to_keychain(credentials: &Credentials) {
    let payload = match serde_json::to_string(credentials) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Try OS keychain
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        if entry.set_password(&payload).is_ok() {
            eprintln!("[auth] saved credentials to OS keychain");
            return;
        }
    }

    // Fallback: file-based
    if let Some(path) = credentials_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&path, &payload) {
            Ok(_) => eprintln!("[auth] saved credentials to {}", path.display()),
            Err(e) => eprintln!("[auth] failed to save credentials file: {}", e),
        }
    }
}

/// Load credentials. Tries OS keychain first, falls back to file.
pub fn load_from_keychain() -> Option<Credentials> {
    // Try OS keychain
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        if let Ok(payload) = entry.get_password() {
            if let Ok(creds) = serde_json::from_str(&payload) {
                eprintln!("[auth] loaded credentials from OS keychain");
                return Some(creds);
            }
        }
    }

    // Fallback: file-based
    let path = credentials_file()?;
    match std::fs::read_to_string(&path) {
        Ok(payload) => match serde_json::from_str(&payload) {
            Ok(creds) => {
                eprintln!("[auth] loaded credentials from {}", path.display());
                Some(creds)
            }
            Err(e) => {
                eprintln!("[auth] failed to parse credentials file: {}", e);
                None
            }
        },
        Err(_) => {
            eprintln!("[auth] no credentials file found");
            None
        }
    }
}

/// Remove saved credentials from keychain and file.
pub fn clear_from_keychain() {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        let _ = entry.delete_credential();
    }
    if let Some(path) = credentials_file() {
        let _ = std::fs::remove_file(&path);
    }
    eprintln!("[auth] cleared saved credentials");
}

/// Thread-safe, in-memory credential store.
///
/// Managed as Tauri application state. The `Mutex` ensures safe concurrent
/// access from multiple IPC command handlers.
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
