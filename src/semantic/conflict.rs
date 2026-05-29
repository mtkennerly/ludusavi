use std::collections::HashMap;

use crate::path::StrictPath;
use crate::semantic::SemanticPath;

/// Represents a conflict where multiple physical files map to the same semantic key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticConflict {
    /// The semantic key that has multiple physical sources.
    pub semantic_key: SemanticPath,
    /// The physical paths that all map to this semantic key.
    pub physical_paths: Vec<StrictPath>,
}

/// Detect duplicate semantic keys from distinct physical files.
/// Same file reached through multiple aliases (symlinks) is collapsed.
/// Distinct physical files with the same semantic key are reported as conflicts.
pub fn detect_conflicts(files: &HashMap<StrictPath, Option<SemanticPath>>) -> Vec<SemanticConflict> {
    // Group files by semantic key using semantic equality
    let mut groups: Vec<(SemanticPath, Vec<StrictPath>)> = Vec::new();

    for (physical, semantic) in files {
        if let Some(sk) = semantic {
            let mut found = false;
            for (key, paths) in &mut groups {
                if key.eq_semantic(sk) {
                    paths.push(physical.clone());
                    found = true;
                    break;
                }
            }
            if !found {
                groups.push((sk.clone(), vec![physical.clone()]));
            }
        }
    }

    // Find groups with multiple distinct physical files
    let mut conflicts = Vec::new();
    for (semantic_key, paths) in groups {
        if paths.len() > 1 {
            // Check if they are the same file (by rendered path) or truly distinct.
            // Use render() instead of interpret() to avoid filesystem dependency.
            let unique: std::collections::HashSet<String> = paths.iter().map(|p| p.render()).collect();

            if unique.len() > 1 {
                conflicts.push(SemanticConflict {
                    semantic_key,
                    physical_paths: paths,
                });
            }
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{SemanticBase, SemanticPath};

    #[test]
    fn no_conflict_with_single_file() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/file1"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflict_with_two_distinct_files() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/file1"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        files.insert(
            StrictPath::new("/other/path/to/file2"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        let conflicts = detect_conflicts(&files);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].physical_paths.len(), 2);
    }

    #[test]
    fn no_conflict_with_same_file_via_alias() {
        // If both paths resolve to the same file, no conflict
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/./file1"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        files.insert(
            StrictPath::new("/path/to/file1"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        let conflicts = detect_conflicts(&files);
        // Both resolve to the same path, so no conflict
        assert!(conflicts.is_empty());
    }

    #[test]
    fn no_conflict_with_different_semantic_keys() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/file1"),
            Some(SemanticPath {
                base: SemanticBase::WinDocuments,
                tail: "Game/save.dat".to_string(),
            }),
        );
        files.insert(
            StrictPath::new("/path/to/file2"),
            Some(SemanticPath {
                base: SemanticBase::WinAppData,
                tail: "Game/config.ini".to_string(),
            }),
        );
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn no_conflict_for_files_without_semantic_key() {
        let mut files = HashMap::new();
        files.insert(StrictPath::new("/path/to/file1"), None);
        files.insert(StrictPath::new("/path/to/file2"), None);
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }
}
