use super::{DepotManifest, ManifestEntry, ManifestError};

/// Parse a DepotDownloader `-manifest-only` text output into a [`DepotManifest`].
///
/// The input is expected to be the full text content of the manifest file.
/// This function performs no I/O — the caller is responsible for reading the file.
pub fn parse_manifest(input: &str) -> Result<DepotManifest, ManifestError> {
    let mut lines = input.lines();

    // --- First line: "Content Manifest for Depot <id>" ---
    let depot_id = parse_depot_line(lines.next())?;

    // --- Header key-value lines (skip blank lines, stop at the column header) ---
    let mut manifest_id: Option<u64> = None;
    let mut date: Option<String> = None;
    let mut total_files: Option<u64> = None;
    let mut total_chunks: Option<u64> = None;
    let mut total_bytes_on_disk: Option<u64> = None;
    let mut total_bytes_compressed: Option<u64> = None;
    let mut found_table_header = false;

    for line in &mut lines {
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // Detect the file table column header
        if trimmed.starts_with("Size") && trimmed.contains("Chunks") && trimmed.contains("Name") {
            found_table_header = true;
            break;
        }

        // Parse key-value header line with " : " separator
        if let Some((key, value)) = trimmed.split_once(" : ") {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Manifest ID / date" => {
                    // Format: "<id> / <date>"
                    if let Some((id_str, date_str)) = value.split_once(" / ") {
                        manifest_id = Some(parse_u64(id_str.trim(), "Manifest ID")?);
                        date = Some(date_str.trim().to_string());
                    } else {
                        return Err(ManifestError::InvalidField {
                            field: "Manifest ID / date".into(),
                            detail: format!("expected '<id> / <date>', got '{}'", value),
                        });
                    }
                }
                "Total number of files" => {
                    total_files = Some(parse_u64(value, "Total number of files")?);
                }
                "Total number of chunks" => {
                    total_chunks = Some(parse_u64(value, "Total number of chunks")?);
                }
                "Total bytes on disk" => {
                    total_bytes_on_disk = Some(parse_u64(value, "Total bytes on disk")?);
                }
                "Total bytes compressed" => {
                    total_bytes_compressed = Some(parse_u64(value, "Total bytes compressed")?);
                }
                _ => {
                    // Ignore unknown header fields for forward compatibility
                }
            }
        }
    }

    let manifest_id = manifest_id
        .ok_or_else(|| ManifestError::MissingField("Manifest ID / date".into()))?;
    let date = date
        .ok_or_else(|| ManifestError::MissingField("Manifest ID / date".into()))?;
    let total_files = total_files
        .ok_or_else(|| ManifestError::MissingField("Total number of files".into()))?;
    let total_chunks = total_chunks
        .ok_or_else(|| ManifestError::MissingField("Total number of chunks".into()))?;
    let total_bytes_on_disk = total_bytes_on_disk
        .ok_or_else(|| ManifestError::MissingField("Total bytes on disk".into()))?;
    let total_bytes_compressed = total_bytes_compressed
        .ok_or_else(|| ManifestError::MissingField("Total bytes compressed".into()))?;

    // --- File table entries ---
    let mut entries = Vec::new();

    if found_table_header {
        for (i, line) in lines.enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry = parse_entry(trimmed, i + 1)?;
            entries.push(entry);
        }
    }

    Ok(DepotManifest {
        depot_id,
        manifest_id,
        date,
        total_files,
        total_chunks,
        total_bytes_on_disk,
        total_bytes_compressed,
        entries,
    })
}

/// Parse the first line: `Content Manifest for Depot <id>`
fn parse_depot_line(line: Option<&str>) -> Result<u64, ManifestError> {
    let line = line
        .ok_or_else(|| ManifestError::InvalidHeader("empty input".into()))?
        .trim();

    let prefix = "Content Manifest for Depot ";
    if !line.starts_with(prefix) {
        return Err(ManifestError::InvalidHeader(format!(
            "expected line starting with '{}', got '{}'",
            prefix, line
        )));
    }

    let id_str = &line[prefix.len()..];
    id_str.parse::<u64>().map_err(|e| ManifestError::InvalidField {
        field: "Depot ID".into(),
        detail: format!("'{}' is not a valid u64: {}", id_str, e),
    })
}

/// Parse a string as u64, returning a descriptive [`ManifestError`].
fn parse_u64(s: &str, field: &str) -> Result<u64, ManifestError> {
    s.parse::<u64>().map_err(|e| ManifestError::InvalidField {
        field: field.into(),
        detail: format!("'{}' is not a valid number: {}", s, e),
    })
}

/// Parse a single file table row.
///
/// Columns are whitespace-separated: Size, Chunks, SHA, Flags, Name.
/// The Name field consumes the rest of the line (may contain spaces).
fn parse_entry(line: &str, line_number: usize) -> Result<ManifestEntry, ManifestError> {
    let mut tokens = Vec::new();
    let mut rest = line;

    // Extract the first 4 whitespace-delimited tokens (size, chunks, sha, flags)
    for field in &["size", "chunks", "sha", "flags"] {
        rest = rest.trim_start();
        if rest.is_empty() {
            return Err(ManifestError::InvalidEntry {
                line: line_number,
                detail: format!("missing {}", field),
            });
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        tokens.push(&rest[..end]);
        rest = &rest[end..];
    }

    // The remainder (after trimming leading whitespace) is the file name
    let name = rest.trim_start();
    if name.is_empty() {
        return Err(ManifestError::InvalidEntry {
            line: line_number,
            detail: "missing file name".into(),
        });
    }

    let size = tokens[0].parse::<u64>().map_err(|e| ManifestError::InvalidEntry {
        line: line_number,
        detail: format!("invalid size '{}': {}", tokens[0], e),
    })?;
    let chunks = tokens[1].parse::<u32>().map_err(|e| ManifestError::InvalidEntry {
        line: line_number,
        detail: format!("invalid chunks '{}': {}", tokens[1], e),
    })?;
    let flags = tokens[3].parse::<u32>().map_err(|e| ManifestError::InvalidEntry {
        line: line_number,
        detail: format!("invalid flags '{}': {}", tokens[3], e),
    })?;

    Ok(ManifestEntry {
        size,
        chunks,
        sha: tokens[2].to_string(),
        flags,
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full example from docs/domain/depotdownloader.md.
    const EXAMPLE_MANIFEST: &str = "\
Content Manifest for Depot 3321461

Manifest ID / date     : 3559081655545104676 / 03/22/2026 16:01:45
Total number of files  : 257
Total number of chunks : 130874
Total bytes on disk    : 133352312992
Total bytes compressed : 100116131120

          Size Chunks File SHA                                 Flags Name
       6740755      7 8a11847b3e22b2fb909b57787ed94d1bb139bcb2     0 0000/0.pamt
     912261088    896 3e6800918fef5f8880cf601e5b60bff031465e60     0 0000/0.paz";

    #[test]
    fn parse_example_manifest_header() {
        let manifest = parse_manifest(EXAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.depot_id, 3321461);
        assert_eq!(manifest.manifest_id, 3559081655545104676);
        assert_eq!(manifest.date, "03/22/2026 16:01:45");
        assert_eq!(manifest.total_files, 257);
        assert_eq!(manifest.total_chunks, 130874);
        assert_eq!(manifest.total_bytes_on_disk, 133352312992);
        assert_eq!(manifest.total_bytes_compressed, 100116131120);
    }

    #[test]
    fn parse_example_manifest_entries() {
        let manifest = parse_manifest(EXAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.entries.len(), 2);

        let first = &manifest.entries[0];
        assert_eq!(first.size, 6740755);
        assert_eq!(first.chunks, 7);
        assert_eq!(first.sha, "8a11847b3e22b2fb909b57787ed94d1bb139bcb2");
        assert_eq!(first.flags, 0);
        assert_eq!(first.name, "0000/0.pamt");

        let second = &manifest.entries[1];
        assert_eq!(second.size, 912261088);
        assert_eq!(second.chunks, 896);
        assert_eq!(second.sha, "3e6800918fef5f8880cf601e5b60bff031465e60");
        assert_eq!(second.flags, 0);
        assert_eq!(second.name, "0000/0.paz");
    }

    #[test]
    fn parse_empty_manifest() {
        let input = "\
Content Manifest for Depot 12345

Manifest ID / date     : 9999999999 / 01/01/2026 00:00:00
Total number of files  : 0
Total number of chunks : 0
Total bytes on disk    : 0
Total bytes compressed : 0
";
        let manifest = parse_manifest(input).unwrap();
        assert_eq!(manifest.depot_id, 12345);
        assert_eq!(manifest.total_files, 0);
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn parse_single_file_manifest() {
        let input = "\
Content Manifest for Depot 55555

Manifest ID / date     : 1234567890123456789 / 12/31/2025 23:59:59
Total number of files  : 1
Total number of chunks : 42
Total bytes on disk    : 1048576
Total bytes compressed : 524288

          Size Chunks File SHA                                 Flags Name
       1048576     42 abcdef0123456789abcdef0123456789abcdef01     5 path/to/file.bin";
        let manifest = parse_manifest(input).unwrap();
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].size, 1048576);
        assert_eq!(manifest.entries[0].chunks, 42);
        assert_eq!(manifest.entries[0].sha, "abcdef0123456789abcdef0123456789abcdef01");
        assert_eq!(manifest.entries[0].flags, 5);
        assert_eq!(manifest.entries[0].name, "path/to/file.bin");
    }

    #[test]
    fn parse_error_missing_depot_line() {
        let input = "Some garbage\nMore garbage";
        let result = parse_manifest(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::InvalidHeader(_) => {}
            e => panic!("expected InvalidHeader, got {:?}", e),
        }
    }

    #[test]
    fn parse_error_missing_manifest_id() {
        let input = "\
Content Manifest for Depot 12345

Total number of files  : 0
Total number of chunks : 0
Total bytes on disk    : 0
Total bytes compressed : 0
";
        let result = parse_manifest(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::MissingField(f) => assert_eq!(f, "Manifest ID / date"),
            e => panic!("expected MissingField, got {:?}", e),
        }
    }

    #[test]
    fn parse_error_invalid_size_in_entry() {
        let input = "\
Content Manifest for Depot 12345

Manifest ID / date     : 9999999999 / 01/01/2026 00:00:00
Total number of files  : 1
Total number of chunks : 1
Total bytes on disk    : 100
Total bytes compressed : 50

          Size Chunks File SHA                                 Flags Name
       notanum      1 abcdef0123456789abcdef0123456789abcdef01     0 file.txt";
        let result = parse_manifest(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::InvalidEntry { .. } => {}
            e => panic!("expected InvalidEntry, got {:?}", e),
        }
    }

    #[test]
    fn parse_file_name_with_spaces() {
        let input = "\
Content Manifest for Depot 12345

Manifest ID / date     : 9999999999 / 01/01/2026 00:00:00
Total number of files  : 1
Total number of chunks : 1
Total bytes on disk    : 100
Total bytes compressed : 50

          Size Chunks File SHA                                 Flags Name
          100      1 abcdef0123456789abcdef0123456789abcdef01     0 path/to/my file with spaces.txt";
        let manifest = parse_manifest(input).unwrap();
        assert_eq!(manifest.entries[0].name, "path/to/my file with spaces.txt");
    }
}
