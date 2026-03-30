mod diff;
mod list_parser;
mod parser;

pub use diff::{diff_manifests, ManifestDiff};
pub use list_parser::parse_manifest_list;
pub use parser::parse_manifest;

/// Metadata and file entries from a DepotDownloader `-manifest-only` output.
#[derive(Debug, Clone, PartialEq)]
pub struct DepotManifest {
    /// Steam depot ID (e.g., `3321461`).
    pub depot_id: u64,
    /// Unique manifest identifier.
    pub manifest_id: u64,
    /// Date string from the manifest header (e.g., `"03/22/2026 16:01:45"`).
    pub date: String,
    /// Total number of files listed in the manifest.
    pub total_files: u64,
    /// Total number of download chunks across all files.
    pub total_chunks: u64,
    /// Total uncompressed size in bytes.
    pub total_bytes_on_disk: u64,
    /// Total compressed size in bytes.
    pub total_bytes_compressed: u64,
    /// Per-file entries parsed from the file table.
    pub entries: Vec<ManifestEntry>,
}

/// A single file entry from the manifest file table.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestEntry {
    /// File size in bytes.
    pub size: u64,
    /// Number of download chunks.
    pub chunks: u32,
    /// SHA-1 hash, hex-encoded (40 characters).
    pub sha: String,
    /// File flags.
    pub flags: u32,
    /// Relative file path.
    pub name: String,
}

/// A single entry from a manifest listing (available versions for a depot).
///
/// Returned by the SteamKit sidecar when listing manifests as newline-delimited JSON.
/// Both `manifest_id` and `date` are required fields.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ManifestListEntry {
    /// The manifest identifier (large numeric ID as string for JSON safety).
    pub manifest_id: String,
    /// Date/time string from the sidecar output.
    pub date: String,
}

/// Errors that can occur when parsing a depot manifest.
#[derive(Debug, Clone, PartialEq)]
pub enum ManifestError {
    /// The first line does not match the expected format.
    InvalidHeader(String),
    /// A required header field is missing.
    MissingField(String),
    /// A header field value could not be parsed.
    InvalidField { field: String, detail: String },
    /// A file table row could not be parsed.
    InvalidEntry { line: usize, detail: String },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::InvalidHeader(msg) => write!(f, "invalid manifest header: {}", msg),
            ManifestError::MissingField(field) => {
                write!(f, "missing required manifest field: {}", field)
            }
            ManifestError::InvalidField { field, detail } => {
                write!(f, "invalid manifest field '{}': {}", field, detail)
            }
            ManifestError::InvalidEntry { line, detail } => {
                write!(f, "invalid manifest entry at line {}: {}", line, detail)
            }
        }
    }
}

impl std::error::Error for ManifestError {}
