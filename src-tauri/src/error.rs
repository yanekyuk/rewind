//! Shared error types for the Rewind application.
//!
//! This module defines the top-level error enum used across all layers.
//! Each layer may define its own specific error types that convert into
//! the shared [`RewindError`] for propagation to the frontend via Tauri IPC.

use serde::Serialize;
use thiserror::Error;

/// Top-level error type for the Rewind application.
///
/// Variants will be added as each layer is implemented:
/// - Domain errors (VDF parse failures, manifest diff errors)
/// - Application errors (workflow state errors, orchestration failures)
/// - Infrastructure errors (filesystem I/O, subprocess failures, path detection)
#[derive(Debug, Error, Serialize)]
pub enum RewindError {
    /// A domain-layer error (e.g., parsing, validation, diffing).
    #[error("Domain error: {0}")]
    Domain(String),

    /// An application-layer error (e.g., workflow orchestration failure).
    #[error("Application error: {0}")]
    Application(String),

    /// An infrastructure-layer error (e.g., filesystem I/O, subprocess failure).
    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    /// Authentication is required but credentials have not been provided.
    #[error("Authentication required: {0}")]
    AuthRequired(String),

    /// Authentication failed (invalid credentials, expired 2FA, etc.).
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}
