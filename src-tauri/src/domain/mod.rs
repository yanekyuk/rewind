//! Domain layer — pure business logic and type definitions.
//!
//! This layer contains Steam types, VDF/ACF parsing logic, manifest diffing,
//! and filelist generation. It has no I/O and no external dependencies beyond
//! the standard library and serialization crates.
//!
//! # Architecture Rule
//!
//! The domain layer **must not** import from the application or infrastructure layers.
//!
//! # Planned Submodules
//!
//! - Steam types (App, Depot, Manifest, BuildId)
//! - VDF/ACF parser
//! - Manifest diffing algorithm
//! - Filelist generation
//! - Trait interfaces for infrastructure (implemented in the infrastructure layer)

pub mod game;
pub mod vdf;
