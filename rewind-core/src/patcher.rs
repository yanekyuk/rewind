use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatcherError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Patch the StateFlags, buildid, and manifest fields in an appmanifest ACF file.
pub fn patch_acf_file(
    path: &Path,
    buildid: &str,
    manifest_id: &str,
    depot_id: u32,
) -> Result<(), PatcherError> {
    let content = std::fs::read_to_string(path)?;
    let patched = patch_acf(&content, buildid, manifest_id, depot_id);
    std::fs::write(path, patched)?;
    Ok(())
}

/// Pure function: takes ACF text and returns patched ACF text.
/// Updates StateFlags → "4", buildid, and the manifest ID for the given depot.
pub fn patch_acf(content: &str, buildid: &str, manifest_id: &str, depot_id: u32) -> String {
    let depot_key = format!("\"{}\"", depot_id);
    let mut in_depot_section = false;
    let mut manifest_patched = false;
    let mut depot_brace_depth: i32 = -1;
    let mut depth: i32 = 0;

    let lines: Vec<String> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();

            if trimmed == "{" {
                depth += 1;
                if in_depot_section && depot_brace_depth < 0 {
                    depot_brace_depth = depth;
                }
                return line.to_string();
            }
            if trimmed == "}" {
                if in_depot_section && depth == depot_brace_depth {
                    in_depot_section = false;
                    depot_brace_depth = -1;
                }
                depth -= 1;
                return line.to_string();
            }

            // Patch StateFlags (top-level only, depth == 1)
            if depth == 1 && trimmed.starts_with("\"StateFlags\"") {
                let indent = leading_whitespace(line);
                return format!("{}\"StateFlags\"\t\t\"4\"", indent);
            }

            // Patch buildid (top-level only)
            if depth == 1 && trimmed.starts_with("\"buildid\"") {
                let indent = leading_whitespace(line);
                return format!("{}\"buildid\"\t\t\"{}\"", indent, buildid);
            }

            // Clear TargetBuildID so Steam doesn't think an update is queued.
            if depth == 1 && trimmed.starts_with("\"TargetBuildID\"") {
                let indent = leading_whitespace(line);
                return format!("{}\"TargetBuildID\"\t\t\"0\"", indent);
            }

            // Prevent Steam from doing a full file validation on next launch.
            if depth == 1 && trimmed.starts_with("\"FullValidateAfterNextUpdate\"") {
                let indent = leading_whitespace(line);
                return format!("{}\"FullValidateAfterNextUpdate\"\t\t\"0\"", indent);
            }

            // Detect depot section by its key
            if trimmed == depot_key {
                in_depot_section = true;
                return line.to_string();
            }

            // Patch manifest within the depot section
            if in_depot_section && !manifest_patched && trimmed.starts_with("\"manifest\"") {
                manifest_patched = true;
                let indent = leading_whitespace(line);
                return format!("{}\"manifest\"\t\t\"{}\"", indent, manifest_id);
            }

            line.to_string()
        })
        .collect();

    lines.join("\n")
}

fn leading_whitespace(line: &str) -> &str {
    let trimmed_len = line.trim_start().len();
    &line[..line.len() - trimmed_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ACF: &str = "\"AppState\"\n{\n\t\"appid\"\t\t\"3321460\"\n\t\"Universe\"\t\"1\"\n\t\"name\"\t\t\"Crimson Desert\"\n\t\"StateFlags\"\t\"6\"\n\t\"installdir\"\t\"CrimsonDesert\"\n\t\"buildid\"\t\"12345\"\n\t\"LastOwner\"\t\"76561198000000000\"\n\t\"InstalledDepots\"\n\t{\n\t\t\"3321461\"\n\t\t{\n\t\t\t\"manifest\"\t\"OLD_MANIFEST\"\n\t\t\t\"size\"\t\t\"85899345920\"\n\t\t}\n\t}\n}";

    #[test]
    fn patch_state_flags() {
        let result = patch_acf(SAMPLE_ACF, "99999", "NEW_MANIFEST", 3321461);
        assert!(result.contains("\"StateFlags\"\t\t\"4\""));
        assert!(!result.contains("\"StateFlags\"\t\"6\""));
    }

    #[test]
    fn patch_buildid() {
        let result = patch_acf(SAMPLE_ACF, "99999", "NEW_MANIFEST", 3321461);
        assert!(result.contains("\"buildid\"\t\t\"99999\""));
        assert!(!result.contains("\"12345\""));
    }

    #[test]
    fn patch_manifest_id() {
        let result = patch_acf(SAMPLE_ACF, "99999", "NEW_MANIFEST", 3321461);
        assert!(result.contains("\"manifest\"\t\t\"NEW_MANIFEST\""));
        assert!(!result.contains("OLD_MANIFEST"));
    }

    #[test]
    fn preserves_other_fields() {
        let result = patch_acf(SAMPLE_ACF, "99999", "NEW_MANIFEST", 3321461);
        assert!(result.contains("\"appid\""));
        assert!(result.contains("3321460"));
        assert!(result.contains("Crimson Desert"));
    }

    #[test]
    fn patch_acf_file_roundtrip() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", SAMPLE_ACF).unwrap();
        patch_acf_file(f.path(), "77777", "PATCHED_ID", 3321461).unwrap();
        let result = std::fs::read_to_string(f.path()).unwrap();
        assert!(result.contains("\"buildid\"\t\t\"77777\""));
        assert!(result.contains("PATCHED_ID"));
    }
}
