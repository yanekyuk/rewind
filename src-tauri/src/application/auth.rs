//! Lightweight auth state for the application session.
//!
//! Tracks whether the sidecar daemon is logged in and the authenticated
//! username. All actual credential handling (RefreshToken persistence,
//! Steam Guard, session files) lives in the sidecar process.

use std::sync::Mutex;

/// Thread-safe auth state tracker.
///
/// Managed as Tauri application state. Tracks whether the sidecar has
/// an active authenticated session and the username of that session.
/// No credentials are stored in the Rust backend -- the sidecar owns
/// session persistence via RefreshToken files on disk.
#[derive(Default)]
pub struct AuthState {
    /// The username of the currently authenticated session, if any.
    username: Mutex<Option<String>>,
}

impl AuthState {
    /// Record a successful login.
    pub fn set_logged_in(&self, username: &str) {
        if let Ok(mut guard) = self.username.lock() {
            *guard = Some(username.to_string());
        }
    }

    /// Check whether a user is logged in.
    pub fn is_logged_in(&self) -> bool {
        self.username
            .lock()
            .ok()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Get the username of the logged-in user, if any.
    pub fn username(&self) -> Option<String> {
        self.username.lock().ok().and_then(|guard| guard.clone())
    }

    /// Clear the auth state (logout).
    pub fn clear(&self) {
        if let Ok(mut guard) = self.username.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_not_logged_in() {
        let state = AuthState::default();
        assert!(!state.is_logged_in());
        assert!(state.username().is_none());
    }

    #[test]
    fn set_logged_in_tracks_username() {
        let state = AuthState::default();
        state.set_logged_in("testuser");
        assert!(state.is_logged_in());
        assert_eq!(state.username(), Some("testuser".to_string()));
    }

    #[test]
    fn clear_removes_login_state() {
        let state = AuthState::default();
        state.set_logged_in("testuser");
        assert!(state.is_logged_in());

        state.clear();
        assert!(!state.is_logged_in());
        assert!(state.username().is_none());
    }

    #[test]
    fn set_logged_in_replaces_previous_user() {
        let state = AuthState::default();
        state.set_logged_in("user1");
        state.set_logged_in("user2");
        assert_eq!(state.username(), Some("user2".to_string()));
    }
}
