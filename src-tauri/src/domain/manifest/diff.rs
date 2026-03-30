use std::collections::HashMap;

use super::{DepotManifest, ManifestEntry};

/// Result of comparing two depot manifests.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDiff {
    /// Files present in both manifests but with different SHA hashes.
    pub changed: Vec<ManifestEntry>,
    /// Files in the target manifest but not in the current manifest.
    pub added: Vec<ManifestEntry>,
    /// Files in the current manifest but not in the target manifest.
    pub removed: Vec<ManifestEntry>,
}

impl ManifestDiff {
    /// Generate a filelist of paths that need downloading.
    ///
    /// Returns the file names of all **changed** and **added** entries,
    /// suitable for DepotDownloader's `-filelist` flag.
    pub fn filelist(&self) -> Vec<String> {
        self.changed
            .iter()
            .chain(self.added.iter())
            .map(|entry| entry.name.clone())
            .collect()
    }
}

/// Compare two depot manifests and classify files as changed, added, or removed.
///
/// Uses a `HashMap` keyed by file name for O(n + m) comparison.
pub fn diff_manifests(current: &DepotManifest, target: &DepotManifest) -> ManifestDiff {
    let mut current_map: HashMap<&str, &ManifestEntry> = HashMap::new();
    for entry in &current.entries {
        current_map.insert(&entry.name, entry);
    }

    let mut changed = Vec::new();
    let mut added = Vec::new();

    for target_entry in &target.entries {
        match current_map.remove(target_entry.name.as_str()) {
            Some(current_entry) => {
                if current_entry.sha != target_entry.sha {
                    changed.push(target_entry.clone());
                }
            }
            None => {
                added.push(target_entry.clone());
            }
        }
    }

    let removed: Vec<ManifestEntry> = current_map
        .into_values()
        .cloned()
        .collect();

    ManifestDiff {
        changed,
        added,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal DepotManifest with the given entries.
    fn make_manifest(entries: Vec<ManifestEntry>) -> DepotManifest {
        DepotManifest {
            depot_id: 12345,
            manifest_id: 1,
            date: "01/01/2026 00:00:00".to_string(),
            total_files: entries.len() as u64,
            total_chunks: 0,
            total_bytes_on_disk: 0,
            total_bytes_compressed: 0,
            entries,
        }
    }

    /// Helper to create a ManifestEntry with the given name and SHA.
    fn make_entry(name: &str, sha: &str) -> ManifestEntry {
        ManifestEntry {
            size: 100,
            chunks: 1,
            sha: sha.to_string(),
            flags: 0,
            name: name.to_string(),
        }
    }

    #[test]
    fn empty_manifests_produce_empty_diff() {
        let current = make_manifest(vec![]);
        let target = make_manifest(vec![]);

        let diff = diff_manifests(&current, &target);

        assert!(diff.changed.is_empty());
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn identical_manifests_produce_empty_diff() {
        let entries = vec![
            make_entry("file_a.txt", "aaaa"),
            make_entry("file_b.txt", "bbbb"),
        ];
        let current = make_manifest(entries.clone());
        let target = make_manifest(entries);

        let diff = diff_manifests(&current, &target);

        assert!(diff.changed.is_empty());
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn files_with_different_sha_detected_as_changed() {
        let current = make_manifest(vec![
            make_entry("game.exe", "aaaa"),
            make_entry("data.pak", "bbbb"),
        ]);
        let target = make_manifest(vec![
            make_entry("game.exe", "cccc"), // different SHA
            make_entry("data.pak", "bbbb"), // same SHA
        ]);

        let diff = diff_manifests(&current, &target);

        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].name, "game.exe");
        assert_eq!(diff.changed[0].sha, "cccc");
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn files_only_in_target_detected_as_added() {
        let current = make_manifest(vec![]);
        let target = make_manifest(vec![
            make_entry("new_file.txt", "aaaa"),
            make_entry("another_new.txt", "bbbb"),
        ]);

        let diff = diff_manifests(&current, &target);

        assert!(diff.changed.is_empty());
        assert_eq!(diff.added.len(), 2);
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn files_only_in_current_detected_as_removed() {
        let current = make_manifest(vec![
            make_entry("old_file.txt", "aaaa"),
            make_entry("deprecated.txt", "bbbb"),
        ]);
        let target = make_manifest(vec![]);

        let diff = diff_manifests(&current, &target);

        assert!(diff.changed.is_empty());
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 2);
    }

    #[test]
    fn completely_different_manifests() {
        let current = make_manifest(vec![
            make_entry("old_a.txt", "aaaa"),
            make_entry("old_b.txt", "bbbb"),
        ]);
        let target = make_manifest(vec![
            make_entry("new_a.txt", "cccc"),
            make_entry("new_b.txt", "dddd"),
        ]);

        let diff = diff_manifests(&current, &target);

        assert!(diff.changed.is_empty());
        assert_eq!(diff.added.len(), 2);
        assert_eq!(diff.removed.len(), 2);
    }

    #[test]
    fn mixed_scenario() {
        let current = make_manifest(vec![
            make_entry("unchanged.txt", "aaaa"),  // same SHA in both
            make_entry("modified.txt", "bbbb"),   // different SHA in target
            make_entry("deleted.txt", "cccc"),     // not in target
        ]);
        let target = make_manifest(vec![
            make_entry("unchanged.txt", "aaaa"),  // same SHA
            make_entry("modified.txt", "xxxx"),   // changed SHA
            make_entry("brand_new.txt", "yyyy"),  // not in current
        ]);

        let diff = diff_manifests(&current, &target);

        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].name, "modified.txt");
        assert_eq!(diff.changed[0].sha, "xxxx");

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "brand_new.txt");

        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].name, "deleted.txt");
    }

    #[test]
    fn filelist_includes_changed_and_added_excludes_removed() {
        let current = make_manifest(vec![
            make_entry("unchanged.txt", "aaaa"),
            make_entry("modified.txt", "bbbb"),
            make_entry("deleted.txt", "cccc"),
        ]);
        let target = make_manifest(vec![
            make_entry("unchanged.txt", "aaaa"),
            make_entry("modified.txt", "xxxx"),
            make_entry("brand_new.txt", "yyyy"),
        ]);

        let diff = diff_manifests(&current, &target);
        let filelist = diff.filelist();

        assert_eq!(filelist.len(), 2);
        assert!(filelist.contains(&"modified.txt".to_string()));
        assert!(filelist.contains(&"brand_new.txt".to_string()));
        assert!(!filelist.contains(&"deleted.txt".to_string()));
        assert!(!filelist.contains(&"unchanged.txt".to_string()));
    }
}
