use super::ManifestListEntry;

/// Envelope for the sidecar's `manifest_list` JSON message.
#[derive(serde::Deserialize)]
struct ManifestListMessage {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    manifests: Vec<ManifestListEntry>,
}

/// Parse SteamKit sidecar's manifest listing output into a list of entries.
///
/// The sidecar outputs newline-delimited JSON (NDJSON). The manifest data
/// arrives in a message with `"type":"manifest_list"` containing a `manifests`
/// array. Other message types (log, done, etc.) are silently ignored.
///
/// Expected format:
/// ```json
/// {"type":"log","level":"info","message":"..."}
/// {"type":"manifest_list","manifests":[{"id":"123","date":"public"}]}
/// {"type":"done","success":true}
/// ```
pub fn parse_manifest_list(output: &str) -> Vec<ManifestListEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try to parse as envelope with manifests array
        if let Ok(msg) = serde_json::from_str::<ManifestListMessage>(trimmed) {
            if msg.r#type == "manifest_list" && !msg.manifests.is_empty() {
                entries.extend(msg.manifests);
                continue;
            }
        }

        // Also try parsing as a bare ManifestListEntry (for flexibility)
        if let Ok(entry) = serde_json::from_str::<ManifestListEntry>(trimmed) {
            entries.push(entry);
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_envelope_format() {
        let output = r#"{"type":"log","level":"info","message":"Connected"}
{"type":"manifest_list","manifests":[{"id":"123456","date":"public"},{"id":"789012","date":"beta"}]}
{"type":"done","success":true}"#;

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "123456");
        assert_eq!(entries[0].date, "public");
        assert_eq!(entries[1].manifest_id, "789012");
        assert_eq!(entries[1].date, "beta");
    }

    #[test]
    fn parse_empty_output() {
        assert!(parse_manifest_list("").is_empty());
    }

    #[test]
    fn parse_bare_entries() {
        let output = r#"{"manifest_id":"111","date":"2026-01-01"}
{"manifest_id":"222","date":"2026-02-01"}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_id_alias() {
        let output = r#"{"id":"999","date":"public"}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "999");
    }

    #[test]
    fn ignores_non_manifest_lines() {
        let output = r#"{"type":"log","level":"info","message":"hello"}
{"type":"done","success":true}
not json at all"#;
        assert!(parse_manifest_list(output).is_empty());
    }

    #[test]
    fn parse_empty_manifests_array() {
        let output = r#"{"type":"manifest_list","manifests":[]}"#;
        assert!(parse_manifest_list(output).is_empty());
    }
}
