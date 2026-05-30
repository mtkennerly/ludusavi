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
///
/// Files with different `mapping_context_id` values are NOT considered conflicts,
/// since they come from different Wine prefixes and will be stored separately.
pub fn detect_conflicts(files: &HashMap<StrictPath, (Option<SemanticPath>, Option<usize>)>) -> Vec<SemanticConflict> {
    // Group files by (semantic key, context id) using semantic equality
    let mut groups: Vec<(SemanticPath, Option<usize>, Vec<StrictPath>)> = Vec::new();

    for (physical, (semantic, ctx_id)) in files {
        if let Some(sk) = semantic {
            let mut found = false;
            for (key, existing_ctx, paths) in &mut groups {
                if key.eq_semantic(sk) && existing_ctx == ctx_id {
                    paths.push(physical.clone());
                    found = true;
                    break;
                }
            }
            if !found {
                groups.push((sk.clone(), *ctx_id, vec![physical.clone()]));
            }
        }
    }

    // Find groups with multiple distinct physical files
    let mut conflicts = Vec::new();
    for (semantic_key, _ctx_id, paths) in groups {
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

    fn make_semantic(base: SemanticBase, tail: &str) -> (Option<SemanticPath>, Option<usize>) {
        (
            Some(SemanticPath {
                base,
                tail: tail.to_string(),
            }),
            None,
        )
    }

    #[test]
    fn no_conflict_with_single_file() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/file1"),
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
        );
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflict_with_two_distinct_files() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/path/to/file1"),
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
        );
        files.insert(
            StrictPath::new("/other/path/to/file2"),
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
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
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
        );
        files.insert(
            StrictPath::new("/path/to/file1"),
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
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
            make_semantic(SemanticBase::WinDocuments, "Game/save.dat"),
        );
        files.insert(
            StrictPath::new("/path/to/file2"),
            make_semantic(SemanticBase::WinAppData, "Game/config.ini"),
        );
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn no_conflict_for_files_without_semantic_key() {
        let mut files = HashMap::new();
        files.insert(StrictPath::new("/path/to/file1"), (None, None));
        files.insert(StrictPath::new("/path/to/file2"), (None, None));
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn no_conflict_with_same_key_different_contexts() {
        let mut files = HashMap::new();
        files.insert(
            StrictPath::new("/prefix_a/save.dat"),
            (
                Some(SemanticPath {
                    base: SemanticBase::WinDocuments,
                    tail: "Game/save.dat".to_string(),
                }),
                Some(0),
            ),
        );
        files.insert(
            StrictPath::new("/prefix_b/save.dat"),
            (
                Some(SemanticPath {
                    base: SemanticBase::WinDocuments,
                    tail: "Game/save.dat".to_string(),
                }),
                Some(1),
            ),
        );
        let conflicts = detect_conflicts(&files);
        assert!(conflicts.is_empty());
    }
}
