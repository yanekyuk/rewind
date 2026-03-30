use super::ManifestListEntry;

/// Parse SteamKit sidecar's manifest listing JSON output into a list of entries.
///
/// The SteamKit sidecar outputs newline-delimited JSON (NDJSON). Each line is
/// a manifest entry in JSON format with `manifest_id` and `date` fields.
///
/// Expected format (one JSON object per line):
/// ```json
/// {"manifest_id":"1234567890123456789","date":"2026-03-22 16:01:45"}
/// {"manifest_id":"8876543210987654321","date":"2026-03-01 12:30:00"}
/// ```
///
/// Lines that are not valid JSON or don't contain the required fields are
/// silently ignored for robustness.
pub fn parse_manifest_list(output: &str) -> Vec<ManifestListEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try to parse each line as a JSON object
        if let Ok(entry) = serde_json::from_str::<ManifestListEntry>(trimmed) {
            entries.push(entry);
        }
        // Lines that fail to parse are silently ignored
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_manifest_list_ndjson() {
        let output = "\
{\"manifest_id\":\"3559081655545104676\",\"date\":\"2026-03-22 16:01:45\"}
{\"manifest_id\":\"8876543210987654321\",\"date\":\"2026-03-01 12:30:00\"}
{\"manifest_id\":\"1234567890123456789\",\"date\":\"2026-02-15 08:00:00\"}";

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].manifest_id, "3559081655545104676");
        assert_eq!(entries[0].date, "2026-03-22 16:01:45");
        assert_eq!(entries[1].manifest_id, "8876543210987654321");
        assert_eq!(entries[1].date, "2026-03-01 12:30:00");
        assert_eq!(entries[2].manifest_id, "1234567890123456789");
        assert_eq!(entries[2].date, "2026-02-15 08:00:00");
    }

    #[test]
    fn parse_empty_output() {
        let output = "";
        let entries = parse_manifest_list(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_blank_lines() {
        let output = "";
        let entries = parse_manifest_list(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_single_manifest_json() {
        let output = "{\"manifest_id\":\"9999999999\",\"date\":\"2026-01-01 00:00:00\"}";
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "9999999999");
        assert_eq!(entries[0].date, "2026-01-01 00:00:00");
    }

    #[test]
    fn ignores_invalid_json_lines() {
        let output = "\
{\"manifest_id\":\"1111111111\",\"date\":\"2026-06-01 10:00:00\"}
not valid json
{\"manifest_id\":\"2222222222\",\"date\":\"2026-05-01 09:00:00\"}";

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "1111111111");
        assert_eq!(entries[1].manifest_id, "2222222222");
    }

    #[test]
    fn ignores_malformed_json_objects() {
        let output = "\
{\"wrong_field\":\"value\"}
{\"manifest_id\":\"1111111111\",\"date\":\"2026-06-01 10:00:00\"}
{\"manifest_id\":\"2222222222\"}
{\"manifest_id\":\"3333333333\",\"date\":\"2026-05-01 09:00:00\"}";

        let entries = parse_manifest_list(output);
        // Only valid entries with both fields are parsed
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "1111111111");
        assert_eq!(entries[1].manifest_id, "3333333333");
    }

    #[test]
    fn handles_whitespace_around_json() {
        let output = "  {\"manifest_id\":\"5555555555\",\"date\":\"2026-04-15 14:30:00\"}  ";
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "5555555555");
        assert_eq!(entries[0].date, "2026-04-15 14:30:00");
    }

    #[test]
    fn manifest_list_entry_serializes() {
        let entry = ManifestListEntry {
            manifest_id: "1234567890".to_string(),
            date: "2026-01-01 00:00:00".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"manifest_id\":\"1234567890\""));
        assert!(json.contains("\"date\":\"2026-01-01 00:00:00\""));
    }

    #[test]
    fn manifest_list_entry_deserializes() {
        let json = "{\"manifest_id\":\"9876543210\",\"date\":\"2025-12-25 12:00:00\"}";
        let entry: ManifestListEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.manifest_id, "9876543210");
        assert_eq!(entry.date, "2025-12-25 12:00:00");
    }
}
