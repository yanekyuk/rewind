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
//! - [`auth`] — Lightweight auth state tracker (logged-in status + username)
//! - [`downgrade`] — Downgrade workflow orchestration

pub mod auth;
pub mod downgrade;
