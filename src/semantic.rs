pub mod conflict;
pub mod convert;
pub mod materialize;
pub mod prefix;
pub mod preview;
pub mod restore_prompt;
pub mod signals;

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Represents a portable semantic location category.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticBase {
    WinHome,
    WinDocuments,
    WinAppData,
    WinLocalAppData,
    WinLocalAppDataLow,
    WinSavedGames,
    WinPublic,
    WinProgramData,
    WinDir,
    WinDrive(char),
}

impl SemanticBase {
    /// Whether equality comparisons for this base should be case-sensitive.
    /// All Win* bases are case-insensitive; future Linux bases will be case-sensitive.
    pub fn case_sensitive(&self) -> bool {
        match self {
            Self::WinHome
            | Self::WinDocuments
            | Self::WinAppData
            | Self::WinLocalAppData
            | Self::WinLocalAppDataLow
            | Self::WinSavedGames
            | Self::WinPublic
            | Self::WinProgramData
            | Self::WinDir
            | Self::WinDrive(_) => false,
        }
    }

    /// Canonical display name for this base, used in serialization.
    fn display_name(&self) -> String {
        match self {
            Self::WinHome => "winHome".to_string(),
            Self::WinDocuments => "winDocuments".to_string(),
            Self::WinAppData => "winAppData".to_string(),
            Self::WinLocalAppData => "winLocalAppData".to_string(),
            Self::WinLocalAppDataLow => "winLocalAppDataLow".to_string(),
            Self::WinSavedGames => "winSavedGames".to_string(),
            Self::WinPublic => "winPublic".to_string(),
            Self::WinProgramData => "winProgramData".to_string(),
            Self::WinDir => "winDir".to_string(),
            Self::WinDrive(c) => format!("winDrive-{}", c.to_ascii_lowercase()),
        }
    }

    fn parse_name(s: &str) -> Option<Self> {
        match s {
            "winHome" => Some(Self::WinHome),
            "winDocuments" => Some(Self::WinDocuments),
            "winAppData" => Some(Self::WinAppData),
            "winLocalAppData" => Some(Self::WinLocalAppData),
            "winLocalAppDataLow" => Some(Self::WinLocalAppDataLow),
            "winSavedGames" => Some(Self::WinSavedGames),
            "winPublic" => Some(Self::WinPublic),
            "winProgramData" => Some(Self::WinProgramData),
            "winDir" => Some(Self::WinDir),
            other => {
                if let Some(rest) = other.strip_prefix("winDrive-") {
                    let chars: Vec<char> = rest.chars().collect();
                    if chars.len() == 1 && chars[0].is_ascii_alphabetic() {
                        return Some(Self::WinDrive(chars[0].to_ascii_lowercase()));
                    }
                }
                None
            }
        }
    }
}

impl Serialize for SemanticBase {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.display_name())
    }
}

impl<'de> Deserialize<'de> for SemanticBase {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SemanticBase::parse_name(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unrecognized semantic base: {}", s)))
    }
}

/// Error type for semantic path parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticPathError {
    /// The string does not start with a recognized `<base>` token.
    MissingBase,
    /// The tail is empty.
    EmptyTail,
    /// The tail contains a `.` or `..` component.
    InvalidTailComponent,
    /// The string is not a semantic key (e.g., it's a raw OS path).
    NotSemanticKey,
}

impl fmt::Display for SemanticPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBase => write!(f, "missing recognized semantic base"),
            Self::EmptyTail => write!(f, "empty tail path"),
            Self::InvalidTailComponent => write!(f, "tail contains '.' or '..' component"),
            Self::NotSemanticKey => write!(f, "not a semantic key"),
        }
    }
}

impl std::error::Error for SemanticPathError {}

/// A portable save-file identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticPath {
    pub base: SemanticBase,
    /// Forward-slash separated, no leading slash.
    pub tail: String,
}

impl SemanticPath {
    /// Parse a semantic path from the `<baseName>/tail/path` format.
    pub fn parse(s: &str) -> Result<Self, SemanticPathError> {
        if !s.starts_with('<') {
            return Err(SemanticPathError::NotSemanticKey);
        }

        let end = s.find('>').ok_or(SemanticPathError::MissingBase)?;
        let base_name = &s[1..end];
        let base = SemanticBase::parse_name(base_name).ok_or(SemanticPathError::MissingBase)?;

        let rest = &s[end + 1..];
        let tail = if rest.is_empty() {
            return Err(SemanticPathError::EmptyTail);
        } else {
            rest.strip_prefix('/')
                .ok_or(SemanticPathError::MissingBase)?
                .to_string()
        };

        if tail.is_empty() {
            return Err(SemanticPathError::EmptyTail);
        }

        for component in tail.split('/') {
            if component == "." || component == ".." {
                return Err(SemanticPathError::InvalidTailComponent);
            }
        }

        Ok(Self { base, tail })
    }

    /// Canonical string form: `<baseName>/tail/path`.
    pub fn serialize(&self) -> String {
        format!("<{}>/{}", self.base.display_name(), self.tail)
    }

    /// Returns the safe backup storage path: `__ludusavi_semantic__/<baseName>/tail`.
    pub fn storage_path(&self) -> String {
        let base_name = self.base.display_name();
        let safe_tail = self.tail.replace('\\', "/");
        format!("__ludusavi_semantic__/{}/{}", base_name, safe_tail)
    }

    /// Semantic equality that respects case policy of the base.
    pub fn eq_semantic(&self, other: &Self) -> bool {
        if self.base != other.base {
            return false;
        }
        if self.base.case_sensitive() {
            self.tail == other.tail
        } else {
            self.tail.eq_ignore_ascii_case(&other.tail)
        }
    }
}

impl Serialize for SemanticPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.serialize())
    }
}

impl<'de> Deserialize<'de> for SemanticPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SemanticPath::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for SemanticPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.serialize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_win_documents() {
        let path = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        assert_eq!(path.base, SemanticBase::WinDocuments);
        assert_eq!(path.tail, "Game/save.dat");
        assert_eq!(path.serialize(), "<winDocuments>/Game/save.dat");
    }

    #[test]
    fn round_trip_parse_serialize() {
        let inputs = [
            "<winDocuments>/Game/save.dat",
            "<winAppData>/Game/config.ini",
            "<winLocalAppData>/Game/cache",
            "<winLocalAppDataLow>/Game/data",
            "<winSavedGames>/Game/profile",
            "<winPublic>/Game/shared",
            "<winProgramData>/Game/telemetry",
            "<winDir>/System32/config",
            "<winHome>/MyGames/save.dat",
            "<winDrive-d>/Games/save.dat",
        ];
        for input in inputs {
            let parsed = SemanticPath::parse(input).unwrap();
            let serialized = parsed.serialize();
            assert_eq!(serialized, input, "round-trip failed for: {}", input);
        }
    }

    #[test]
    fn parse_rejects_without_base_prefix() {
        assert_eq!(
            SemanticPath::parse("C:/Users/Alice/Documents/Game/save.dat"),
            Err(SemanticPathError::NotSemanticKey)
        );
    }

    #[test]
    fn parse_rejects_unrecognized_base() {
        assert_eq!(
            SemanticPath::parse("<unknown>/Game/save.dat"),
            Err(SemanticPathError::MissingBase)
        );
    }

    #[test]
    fn parse_rejects_empty_tail() {
        assert_eq!(SemanticPath::parse("<winDocuments>"), Err(SemanticPathError::EmptyTail));
        assert_eq!(
            SemanticPath::parse("<winDocuments>/"),
            Err(SemanticPathError::EmptyTail)
        );
    }

    #[test]
    fn parse_rejects_dot_components() {
        assert_eq!(
            SemanticPath::parse("<winDocuments>/../etc/passwd"),
            Err(SemanticPathError::InvalidTailComponent)
        );
        assert_eq!(
            SemanticPath::parse("<winDocuments>/./save.dat"),
            Err(SemanticPathError::InvalidTailComponent)
        );
        assert_eq!(
            SemanticPath::parse("<winDocuments>/Game/../save.dat"),
            Err(SemanticPathError::InvalidTailComponent)
        );
    }

    #[test]
    fn storage_path_never_uses_backslash() {
        let path = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let storage = path.storage_path();
        assert!(!storage.contains('\\'), "storage path contains backslash: {}", storage);
        assert_eq!(storage, "__ludusavi_semantic__/winDocuments/Game/save.dat");
    }

    #[test]
    fn storage_path_for_all_bases() {
        let cases = [
            ("<winHome>/x", "__ludusavi_semantic__/winHome/x"),
            ("<winDocuments>/x", "__ludusavi_semantic__/winDocuments/x"),
            ("<winAppData>/x", "__ludusavi_semantic__/winAppData/x"),
            ("<winLocalAppData>/x", "__ludusavi_semantic__/winLocalAppData/x"),
            ("<winLocalAppDataLow>/x", "__ludusavi_semantic__/winLocalAppDataLow/x"),
            ("<winSavedGames>/x", "__ludusavi_semantic__/winSavedGames/x"),
            ("<winPublic>/x", "__ludusavi_semantic__/winPublic/x"),
            ("<winProgramData>/x", "__ludusavi_semantic__/winProgramData/x"),
            ("<winDir>/x", "__ludusavi_semantic__/winDir/x"),
            ("<winDrive-d>/x", "__ludusavi_semantic__/winDrive-d/x"),
        ];
        for (input, expected) in cases {
            let path = SemanticPath::parse(input).unwrap();
            assert_eq!(path.storage_path(), expected, "storage_path failed for: {}", input);
        }
    }

    #[test]
    fn eq_semantic_case_insensitive_for_win_documents() {
        let a = SemanticPath::parse("<winDocuments>/Game/Save.dat").unwrap();
        let b = SemanticPath::parse("<winDocuments>/game/save.dat").unwrap();
        assert!(a.eq_semantic(&b));
    }

    #[test]
    fn eq_semantic_case_insensitive_for_win_appdata() {
        let a = SemanticPath::parse("<winAppData>/Game/Config.INI").unwrap();
        let b = SemanticPath::parse("<winAppData>/game/config.ini").unwrap();
        assert!(a.eq_semantic(&b));
    }

    #[test]
    fn eq_semantic_different_bases_not_equal() {
        let a = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let b = SemanticPath::parse("<winAppData>/Game/save.dat").unwrap();
        assert!(!a.eq_semantic(&b));
    }

    #[test]
    fn win_drive_serializes_with_lowercase_letter() {
        let base = SemanticBase::WinDrive('D');
        assert_eq!(base.display_name(), "winDrive-d");
    }

    #[test]
    fn win_drive_parses_case_insensitive() {
        let base = SemanticBase::parse_name("winDrive-D").unwrap();
        assert_eq!(base, SemanticBase::WinDrive('d'));
    }

    #[test]
    fn serde_round_trip_all_variants() {
        let variants = [
            SemanticBase::WinHome,
            SemanticBase::WinDocuments,
            SemanticBase::WinAppData,
            SemanticBase::WinLocalAppData,
            SemanticBase::WinLocalAppDataLow,
            SemanticBase::WinSavedGames,
            SemanticBase::WinPublic,
            SemanticBase::WinProgramData,
            SemanticBase::WinDir,
            SemanticBase::WinDrive('d'),
            SemanticBase::WinDrive('c'),
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: SemanticBase = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized, "serde round-trip failed for: {:?}", variant);
        }
    }

    #[test]
    fn semantic_path_serde_round_trip() {
        let path = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "Game/save.dat".to_string(),
        };
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: SemanticPath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, deserialized);
    }
}
