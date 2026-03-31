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
{"type":"manifest_list","manifests":[{"id":"123456","branch":"public"},{"id":"789012","branch":"beta"}]}
{"type":"done","success":true}"#;

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "123456");
        assert_eq!(entries[0].branch, Some("public".to_string()));
        assert_eq!(entries[1].manifest_id, "789012");
        assert_eq!(entries[1].branch, Some("beta".to_string()));
    }

    #[test]
    fn parse_empty_output() {
        assert!(parse_manifest_list("").is_empty());
    }

    #[test]
    fn parse_bare_entries() {
        let output = r#"{"manifest_id":"111","branch":"public"}
{"manifest_id":"222","branch":"beta"}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_id_alias() {
        let output = r#"{"id":"999","branch":"public"}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "999");
        assert_eq!(entries[0].branch, Some("public".to_string()));
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

    #[test]
    fn parse_branch_field() {
        let output = r#"{"type":"manifest_list","manifests":[{"id":"123","branch":"public"},{"id":"456","branch":"beta"}]}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].branch, Some("public".to_string()));
        assert_eq!(entries[1].branch, Some("beta".to_string()));
    }

    #[test]
    fn parse_time_updated_field() {
        let output = r#"{"type":"manifest_list","manifests":[{"id":"123","branch":"public","time_updated":1711123305}]}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].time_updated, Some(1711123305));
    }

    #[test]
    fn parse_pwd_required_field() {
        let output = r#"{"type":"manifest_list","manifests":[{"id":"123","branch":"beta","pwd_required":true}]}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pwd_required, Some(true));
    }

    #[test]
    fn parse_all_new_fields_together() {
        let output = r#"{"type":"manifest_list","manifests":[{"id":"789","branch":"bleeding-edge","time_updated":1711123305,"pwd_required":false}]}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "789");
        assert_eq!(entries[0].branch, Some("bleeding-edge".to_string()));
        assert_eq!(entries[0].time_updated, Some(1711123305));
        assert_eq!(entries[0].pwd_required, Some(false));
    }

    #[test]
    fn parse_missing_optional_fields() {
        let output = r#"{"type":"manifest_list","manifests":[{"id":"999"}]}"#;
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "999");
        assert_eq!(entries[0].branch, None);
        assert_eq!(entries[0].time_updated, None);
        assert_eq!(entries[0].pwd_required, None);
    }
}
