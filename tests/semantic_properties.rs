//! Property-based and round-trip tests for semantic paths.

use proptest::prelude::*;

use ludusavi::path::StrictPath;
use ludusavi::semantic::convert::{KnownFolders, windows_physical_to_semantic, wine_physical_to_semantic};
use ludusavi::semantic::materialize::{MaterializeTarget, materialize_semantic};
use ludusavi::semantic::{SemanticBase, SemanticPath};

fn arb_tail() -> impl Strategy<Value = String> {
    // Generate valid tail paths: non-empty, no dots, forward-slash separated
    // Use alphanumeric characters only to avoid edge cases with spaces
    prop::collection::vec("[a-zA-Z0-9_]{1,20}", 1..5).prop_map(|parts| parts.join("/"))
}

fn arb_semantic_base() -> impl Strategy<Value = SemanticBase> {
    prop_oneof![
        Just(SemanticBase::WinHome),
        Just(SemanticBase::WinDocuments),
        Just(SemanticBase::WinAppData),
        Just(SemanticBase::WinLocalAppData),
        Just(SemanticBase::WinLocalAppDataLow),
        Just(SemanticBase::WinSavedGames),
        Just(SemanticBase::WinPublic),
        Just(SemanticBase::WinProgramData),
        Just(SemanticBase::WinDir),
        prop::char::range('a', 'z').prop_map(SemanticBase::WinDrive),
    ]
}

fn arb_semantic_path() -> impl Strategy<Value = SemanticPath> {
    (arb_semantic_base(), arb_tail()).prop_map(|(base, tail)| SemanticPath { base, tail })
}

proptest! {
    #[test]
    fn parse_serialize_round_trip(sp in arb_semantic_path()) {
        let serialized = sp.serialize();
        let parsed = SemanticPath::parse(&serialized).unwrap();
        prop_assert_eq!(sp, parsed);
    }

    #[test]
    fn storage_path_never_has_backslash(sp in arb_semantic_path()) {
        let storage = sp.storage_path();
        prop_assert!(!storage.contains('\\'), "storage path contains backslash: {}", storage);
    }

    #[test]
    fn storage_path_starts_with_prefix(sp in arb_semantic_path()) {
        let storage = sp.storage_path();
        prop_assert!(storage.starts_with("__ludusavi_semantic__/"), "storage path: {}", storage);
    }

    #[test]
    fn serde_json_round_trip(sp in arb_semantic_path()) {
        let json = serde_json::to_string(&sp).unwrap();
        let deserialized: SemanticPath = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(sp, deserialized);
    }

    #[test]
    fn changing_username_does_not_change_semantic_key(
        tail in "[a-zA-Z0-9_]{1,10}",
        user1 in "[a-z]{3,10}",
        user2 in "[a-z]{3,10}",
    ) {
        prop_assume!(user1 != user2);
        let kf1 = KnownFolders {
            documents: Some(format!("C:/Users/{}/Documents", user1)),
            ..Default::default()
        };
        let kf2 = KnownFolders {
            documents: Some(format!("C:/Users/{}/Documents", user2)),
            ..Default::default()
        };

        let path1 = StrictPath::new(format!("C:/Users/{}/Documents/Game/{}", user1, tail));
        let path2 = StrictPath::new(format!("C:/Users/{}/Documents/Game/{}", user2, tail));

        let sk1 = windows_physical_to_semantic(&path1, &kf1);
        let sk2 = windows_physical_to_semantic(&path2, &kf2);

        prop_assert!(sk1.is_some());
        prop_assert!(sk2.is_some());
        prop_assert_eq!(sk1.unwrap(), sk2.unwrap());
    }

    #[test]
    fn changing_wine_prefix_does_not_change_semantic_key(
        tail in "[a-zA-Z0-9_]{1,10}",
        prefix1 in "[a-z]{3,10}",
        prefix2 in "[a-z]{3,10}",
    ) {
        prop_assume!(prefix1 != prefix2);
        let p1 = StrictPath::new(format!("/home/{}/Prefixes/Game", prefix1));
        let p2 = StrictPath::new(format!("/home/{}/Prefixes/Game", prefix2));
        let f1 = StrictPath::new(format!("/home/{}/Prefixes/Game/drive_c/users/steamuser/Documents/Game/{}", prefix1, tail));
        let f2 = StrictPath::new(format!("/home/{}/Prefixes/Game/drive_c/users/steamuser/Documents/Game/{}", prefix2, tail));

        let sk1 = wine_physical_to_semantic(&f1, &p1, "steamuser");
        let sk2 = wine_physical_to_semantic(&f2, &p2, "steamuser");

        prop_assert!(sk1.is_some());
        prop_assert!(sk2.is_some());
        prop_assert_eq!(sk1.unwrap(), sk2.unwrap());
    }

    #[test]
    fn materialize_then_rederive_windows(sp in arb_semantic_path()) {
        // All semantic bases are Win* bases that materialize to Windows known folders.
        prop_assume!(matches!(sp.base,
            SemanticBase::WinHome |
            SemanticBase::WinDocuments |
            SemanticBase::WinAppData |
            SemanticBase::WinLocalAppData |
            SemanticBase::WinLocalAppDataLow |
            SemanticBase::WinSavedGames |
            SemanticBase::WinPublic |
            SemanticBase::WinProgramData |
            SemanticBase::WinDir |
            SemanticBase::WinDrive(_)
        ));

        let kf = KnownFolders {
            saved_games: Some("C:/Users/Test/Saved Games".to_string()),
            documents: Some("C:/Users/Test/Documents".to_string()),
            local_app_data: Some("C:/Users/Test/AppData/Local".to_string()),
            app_data: Some("C:/Users/Test/AppData/Roaming".to_string()),
            public: Some("C:/Users/Public".to_string()),
            program_data: Some("C:/ProgramData".to_string()),
            windows: Some("C:/Windows".to_string()),
            user_profile: Some("C:/Users/Test".to_string()),
        };

        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let physical = materialize_semantic(&sp, &target).unwrap();
        let rederived = windows_physical_to_semantic(&physical, &kf);

        prop_assert!(rederived.is_some(), "re-derivation failed for: {:?} -> {:?}", sp, physical);
        prop_assert_eq!(sp, rederived.unwrap());
    }
}
