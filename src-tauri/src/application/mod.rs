//! Application layer — workflow orchestration and coordination.
//!
//! This layer coordinates domain logic with infrastructure capabilities.
//! It contains the downgrade workflow (the 9-step process), progress tracking,
//! event emission, and error aggregation.
//!
//! # Architecture Rule
//!
//! The application layer **must not** import from the infrastructure layer directly.
//! It depends on trait interfaces defined in the domain layer, which the
//! infrastructure layer implements. Dependencies are injected at the composition
//! root (Tauri command handlers).
//!
//! # Submodules
//!
//! - [`auth`] — In-memory credential store for session-scoped authentication
//!
//! # Planned Submodules
//!
//! - Downgrade workflow (state machine for the 9-step process)
//! - Progress tracking and event emission
//! - Error aggregation and user-facing error construction

pub mod auth;
pub mod downgrade;
