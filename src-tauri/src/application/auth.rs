//! In-memory credential store for the application session.
//!
//! Only the username is persisted to disk. The sidecar manages its own
//! session token. If the session expires, the user is prompted to re-login.

use std::sync::Mutex;

use crate::domain::auth::Credentials;

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

/// Thread-safe, in-memory credential store.
///
/// Managed as Tauri application state. The `Mutex` ensures safe concurrent
/// access from multiple IPC command handlers.
#[derive(Default)]
pub struct AuthStore {
    credentials: Mutex<Option<Credentials>>,
    /// Stored username from a previous session (no password).
    saved_username: Mutex<Option<String>>,
}

impl AuthStore {
    /// Create an AuthStore, optionally pre-loaded with a saved username.
    pub fn with_saved_username(username: Option<String>) -> Self {
        Self {
            credentials: Mutex::new(None),
            saved_username: Mutex::new(username),
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

    /// Clear stored credentials.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.credentials.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.saved_username.lock() {
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

    #[test]
    fn saved_username_session() {
        let store = AuthStore::with_saved_username(Some("saveduser".to_string()));
        assert!(!store.is_set()); // No full credentials
        assert!(store.has_saved_session());
        assert_eq!(store.username(), Some("saveduser".to_string()));
    }
}
