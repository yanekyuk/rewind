use super::ManifestListEntry;

/// Parse DepotDownloader's manifest listing output into a list of entries.
///
/// When DepotDownloader is run to list available manifests for a depot, it
/// outputs lines containing manifest IDs and dates. This function extracts
/// those entries from the raw stdout output.
///
/// Expected line format (one per manifest):
/// ```text
/// Manifest 1234567890123456789 / 2026-03-22 16:01:45
/// ```
///
/// Lines that don't match this format are silently ignored (e.g., status
/// messages, blank lines, "Total N manifests" summary).
pub fn parse_manifest_list(output: &str) -> Vec<ManifestListEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Match lines like: "Manifest <id> / <date>"
        if let Some(rest) = trimmed.strip_prefix("Manifest ") {
            if let Some((id_str, date_str)) = rest.split_once(" / ") {
                let id = id_str.trim();
                let date = date_str.trim();

                // Validate that the ID looks numeric
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                    entries.push(ManifestListEntry {
                        manifest_id: id.to_string(),
                        date: date.to_string(),
                    });
                }
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_manifest_list() {
        let output = "\
Got depot key for depot 3321461
Looking for manifests for depot 3321461...
Manifest 3559081655545104676 / 2026-03-22 16:01:45
Manifest 8876543210987654321 / 2026-03-01 12:30:00
Manifest 1234567890123456789 / 2026-02-15 08:00:00
Total 3 manifests";

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
    fn parse_no_manifests_found() {
        let output = "\
Got depot key for depot 12345
Looking for manifests for depot 12345...
Total 0 manifests";

        let entries = parse_manifest_list(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_single_manifest() {
        let output = "Manifest 9999999999 / 2026-01-01 00:00:00";
        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest_id, "9999999999");
        assert_eq!(entries[0].date, "2026-01-01 00:00:00");
    }

    #[test]
    fn ignores_non_manifest_lines() {
        let output = "\
Some status message
Connected to Steam
Manifest 1111111111 / 2026-06-01 10:00:00
Downloading something
Manifest 2222222222 / 2026-05-01 09:00:00
Done.";

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "1111111111");
        assert_eq!(entries[1].manifest_id, "2222222222");
    }

    #[test]
    fn ignores_malformed_manifest_lines() {
        let output = "\
Manifest not-a-number / 2026-01-01 00:00:00
Manifest 1111111111 / 2026-06-01 10:00:00
Manifest / missing id
Manifest 2222222222 / 2026-05-01 09:00:00";

        let entries = parse_manifest_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].manifest_id, "1111111111");
        assert_eq!(entries[1].manifest_id, "2222222222");
    }

    #[test]
    fn handles_whitespace_variations() {
        let output = "  Manifest 5555555555 / 2026-04-15 14:30:00  ";
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
}
