//! Infrastructure layer — external world interactions.
//!
//! This layer implements the trait interfaces defined in the domain layer.
//! It handles all I/O, subprocess management, and OS-level operations.
//!
//! # Architecture Rule
//!
//! The infrastructure layer **implements interfaces defined in the domain layer**.
//! It is the only layer that performs I/O or interacts with the operating system.
//!
//! # Submodules
//!
//! - [`sidecar`] — SteamKit sidecar binary resolution
//! - [`depot_downloader`] — SteamKit sidecar manifest operations
//! - `steam` — Steam installation path detection, library folder discovery, appmanifest scanning
//!
//! # Planned Submodules
//!
//! - Filesystem I/O (reading ACF files, copying game files)
//! - SteamKit sidecar subprocess management (spawn, JSON communication, cancellation)
//! - Manifest file locking (chattr on Linux, chflags on macOS, SetFileAttributes on Windows)
//! - OS-level notifications

pub mod depot_downloader;
pub mod sidecar;
pub mod steam;
