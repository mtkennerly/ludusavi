pub mod convert;
pub mod prefix;

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticPath {
    pub base: SemanticBase,
    pub tail: String,
}
