//! In-memory credential store for the application session.
//!
//! Wraps [`Credentials`] in a thread-safe container managed by Tauri state.
//! Credentials exist only in memory — they are never persisted to disk.

use std::sync::Mutex;

use crate::domain::auth::Credentials;

/// Thread-safe, in-memory credential store.
///
/// Managed as Tauri application state. The `Mutex` ensures safe concurrent
/// access from multiple IPC command handlers.
///
/// # Lifecycle
///
/// - Created empty when the app starts (via `Default`)
/// - Populated when the user submits credentials via `set_credentials`
/// - Read when spawning DepotDownloader
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
