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
//! # Planned Submodules
//!
//! - Filesystem I/O (reading ACF files, copying game files)
//! - DepotDownloader subprocess management (spawn, stdin/stdout, cancellation)
//! - Steam installation path detection (platform-specific)
//! - Manifest file locking (chattr on Linux, chflags on macOS, SetFileAttributes on Windows)
//! - OS-level notifications
