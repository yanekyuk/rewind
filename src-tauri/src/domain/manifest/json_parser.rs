//! Parse SteamKit sidecar JSON manifest output into domain types.
//!
//! The sidecar's `get-manifest` command outputs NDJSON. The manifest data
//! arrives in a message with `"type":"manifest"` containing file listings.

use super::{DepotManifest, ManifestEntry};

/// Envelope for the sidecar's manifest JSON message.
#[derive(serde::Deserialize)]
struct ManifestMessage {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    depot_id: u64,
    #[serde(default)]
    manifest_id: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    total_files: u64,
    #[serde(default)]
    total_chunks: u64,
    #[serde(default)]
    total_bytes_on_disk: u64,
    #[serde(default)]
    total_bytes_compressed: u64,
    #[serde(default)]
    files: Vec<ManifestFileEntry>,
}

/// A single file entry from the sidecar's JSON output.
#[derive(serde::Deserialize)]
struct ManifestFileEntry {
    name: String,
    sha: String,
    size: u64,
    chunks: u32,
    flags: u32,
}

/// Parse the SteamKit sidecar's `get-manifest` NDJSON output into a [`DepotManifest`].
///
/// Scans all lines for a message with `"type":"manifest"` and parses it.
/// Other message types (log, done, error) are silently ignored.
///
/// # Errors
///
/// Returns an error string if no manifest message is found or if JSON parsing fails.
pub fn parse_manifest_json(output: &str) -> Result<DepotManifest, String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<ManifestMessage>(trimmed) {
            if msg.r#type == "manifest" {
                let manifest_id = msg
                    .manifest_id
                    .parse::<u64>()
                    .map_err(|e| format!("invalid manifest_id '{}': {}", msg.manifest_id, e))?;

                let entries = msg
                    .files
                    .into_iter()
                    .map(|f| ManifestEntry {
                        name: f.name,
                        sha: f.sha,
                        size: f.size,
                        chunks: f.chunks,
                        flags: f.flags,
                    })
                    .collect();

                return Ok(DepotManifest {
                    depot_id: msg.depot_id,
                    manifest_id,
                    date: msg.date,
                    total_files: msg.total_files,
                    total_chunks: msg.total_chunks,
                    total_bytes_on_disk: msg.total_bytes_on_disk,
                    total_bytes_compressed: msg.total_bytes_compressed,
                    entries,
                });
            }
        }
    }

    Err("no manifest message found in sidecar output".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest_json() {
        let output = r#"{"type":"log","level":"info","message":"Fetching manifest..."}
{"type":"manifest","depot_id":3321461,"manifest_id":"7446650175280810671","total_files":2,"total_chunks":903,"total_bytes_on_disk":919001843,"total_bytes_compressed":700000000,"date":"2026-03-22T16:01:45","files":[{"name":"0000/0.pamt","sha":"8a11847b3e22b2fb909b57787ed94d1bb139bcb2","size":6740755,"chunks":7,"flags":0},{"name":"0000/0.paz","sha":"3e6800918fef5f8880cf601e5b60bff031465e60","size":912261088,"chunks":896,"flags":0}]}
{"type":"done","success":true}"#;

        let manifest = parse_manifest_json(output).unwrap();
        assert_eq!(manifest.depot_id, 3321461);
        assert_eq!(manifest.manifest_id, 7446650175280810671);
        assert_eq!(manifest.total_files, 2);
        assert_eq!(manifest.total_chunks, 903);
        assert_eq!(manifest.total_bytes_on_disk, 919001843);
        assert_eq!(manifest.total_bytes_compressed, 700000000);
        assert_eq!(manifest.date, "2026-03-22T16:01:45");
        assert_eq!(manifest.entries.len(), 2);

        assert_eq!(manifest.entries[0].name, "0000/0.pamt");
        assert_eq!(
            manifest.entries[0].sha,
            "8a11847b3e22b2fb909b57787ed94d1bb139bcb2"
        );
        assert_eq!(manifest.entries[0].size, 6740755);
        assert_eq!(manifest.entries[0].chunks, 7);
        assert_eq!(manifest.entries[0].flags, 0);

        assert_eq!(manifest.entries[1].name, "0000/0.paz");
        assert_eq!(manifest.entries[1].size, 912261088);
    }

    #[test]
    fn parse_manifest_json_no_manifest_message() {
        let output = r#"{"type":"log","level":"info","message":"hello"}
{"type":"done","success":true}"#;

        let result = parse_manifest_json(output);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no manifest message found"));
    }

    #[test]
    fn parse_manifest_json_empty_files() {
        let output = r#"{"type":"manifest","depot_id":12345,"manifest_id":"999","total_files":0,"total_chunks":0,"total_bytes_on_disk":0,"total_bytes_compressed":0,"date":"2026-01-01","files":[]}"#;

        let manifest = parse_manifest_json(output).unwrap();
        assert_eq!(manifest.depot_id, 12345);
        assert_eq!(manifest.manifest_id, 999);
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn parse_manifest_json_invalid_manifest_id() {
        let output = r#"{"type":"manifest","depot_id":12345,"manifest_id":"not_a_number","total_files":0,"total_chunks":0,"total_bytes_on_disk":0,"total_bytes_compressed":0,"date":"","files":[]}"#;

        let result = parse_manifest_json(output);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid manifest_id"));
    }

    #[test]
    fn parse_manifest_json_empty_input() {
        let result = parse_manifest_json("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_manifest_json_ignores_non_manifest_lines() {
        let output = r#"{"type":"error","code":"SOMETHING","message":"oops"}
{"type":"manifest","depot_id":55555,"manifest_id":"1234567890123456789","total_files":1,"total_chunks":42,"total_bytes_on_disk":1048576,"total_bytes_compressed":524288,"date":"2026-12-31","files":[{"name":"game.exe","sha":"abcdef01","size":1048576,"chunks":42,"flags":0}]}
{"type":"done","success":true}"#;

        let manifest = parse_manifest_json(output).unwrap();
        assert_eq!(manifest.depot_id, 55555);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].name, "game.exe");
    }
}
