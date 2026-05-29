use crate::semantic::SemanticPath;

/// Signal types for comparing current scan to existing backup.
#[derive(Clone, Debug)]
pub enum SemanticSignal {
    /// Current scan and existing backup describe the same save location.
    SameSemanticKey { semantic_key: SemanticPath },
    /// Existing backup has entries in a namespace the current platform understands,
    /// but current scan has no match.
    SameNamespaceMissing { semantic_key: SemanticPath },
    /// Existing backup has entries in a namespace the current platform cannot materialize.
    ForeignNamespace { semantic_key: SemanticPath },
    /// Key is understood but multiple physical targets exist.
    AmbiguousMaterialization {
        semantic_key: SemanticPath,
        candidates: Vec<String>,
    },
}

impl SemanticSignal {
    pub fn semantic_key(&self) -> &SemanticPath {
        match self {
            Self::SameSemanticKey { semantic_key } => semantic_key,
            Self::SameNamespaceMissing { semantic_key } => semantic_key,
            Self::ForeignNamespace { semantic_key } => semantic_key,
            Self::AmbiguousMaterialization { semantic_key, .. } => semantic_key,
        }
    }

    pub fn is_same_key(&self) -> bool {
        matches!(self, Self::SameSemanticKey { .. })
    }

    pub fn is_foreign(&self) -> bool {
        matches!(self, Self::ForeignNamespace { .. })
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Self::AmbiguousMaterialization { .. })
    }
}

/// Compare current scan semantic keys against existing backup semantic keys
/// and produce signals.
pub fn compare_semantic_keys(
    current: &[SemanticPath],
    backup: &[SemanticPath],
    current_can_materialize: impl Fn(&SemanticPath) -> bool,
) -> Vec<SemanticSignal> {
    let mut signals = Vec::new();

    // Use semantic equality for case-insensitive comparison
    let _current_set: std::collections::HashSet<String> = current.iter().map(|s| s.serialize()).collect();
    let _backup_set: std::collections::HashSet<String> = backup.iter().map(|s| s.serialize()).collect();

    // Find keys in backup that the current platform understands
    for bk in backup {
        // Check if any current key matches semantically (case-insensitive for Windows/Wine)
        let current_match = current.iter().find(|c| c.eq_semantic(bk));
        if current_match.is_some() {
            signals.push(SemanticSignal::SameSemanticKey {
                semantic_key: bk.clone(),
            });
        } else if current_can_materialize(bk) {
            // Current platform can materialize this key but no current scan match
            signals.push(SemanticSignal::SameNamespaceMissing {
                semantic_key: bk.clone(),
            });
        } else {
            // Current platform cannot materialize this key
            signals.push(SemanticSignal::ForeignNamespace {
                semantic_key: bk.clone(),
            });
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::SemanticBase;

    fn win_docs(tail: &str) -> SemanticPath {
        SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: tail.to_string(),
        }
    }

    fn win_drive_d(tail: &str) -> SemanticPath {
        SemanticPath {
            base: SemanticBase::WinDrive('d'),
            tail: tail.to_string(),
        }
    }

    #[test]
    fn same_semantic_key() {
        let current = vec![win_docs("Game/save.dat")];
        let backup = vec![win_docs("Game/save.dat")];
        let signals = compare_semantic_keys(&current, &backup, |_| true);
        assert_eq!(signals.len(), 1);
        assert!(signals[0].is_same_key());
    }

    #[test]
    fn same_namespace_missing() {
        let current = vec![];
        let backup = vec![win_docs("Game/save.dat")];
        let signals = compare_semantic_keys(&current, &backup, |_| true);
        assert_eq!(signals.len(), 1);
        assert!(matches!(signals[0], SemanticSignal::SameNamespaceMissing { .. }));
    }

    #[test]
    fn foreign_namespace() {
        let current = vec![];
        let backup = vec![win_docs("Game/save.dat")];
        let signals = compare_semantic_keys(&current, &backup, |_| false);
        assert_eq!(signals.len(), 1);
        assert!(signals[0].is_foreign());
    }

    #[test]
    fn mixed_signals() {
        let current = vec![win_docs("Game/save.dat")];
        let backup = vec![
            win_docs("Game/save.dat"),
            win_docs("Other/game.dat"),
            win_drive_d("Games/save.dat"),
        ];
        let signals = compare_semantic_keys(&current, &backup, |sk| sk.base != SemanticBase::WinDrive('d'));
        assert_eq!(signals.len(), 3);

        let same_count = signals.iter().filter(|s| s.is_same_key()).count();
        let missing_count = signals
            .iter()
            .filter(|s| matches!(s, SemanticSignal::SameNamespaceMissing { .. }))
            .count();
        let foreign_count = signals.iter().filter(|s| s.is_foreign()).count();

        assert_eq!(same_count, 1);
        assert_eq!(missing_count, 1);
        assert_eq!(foreign_count, 1);
    }

    #[test]
    fn empty_inputs() {
        let signals = compare_semantic_keys(&[], &[], |_| true);
        assert!(signals.is_empty());
    }
}
