use crate::path::StrictPath;

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Ea {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Epic {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Gog {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct GogGalaxy {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Heroic {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

impl Heroic {
    pub const FLATPAK_SUFFIX: &str = ".var/app/com.heroicgameslauncher.hgl/config/heroic";

    pub fn flatpak_home(&self) -> Option<StrictPath> {
        self.path
            .raw()
            .ends_with(Self::FLATPAK_SUFFIX)
            .then(|| self.path.popped().popped())
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Legendary {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Lutris {
    /// Where the root is located on your system.
    pub path: StrictPath,
    /// Full path to the Lutris `pga.db` file, if not contained within the main `path`.
    pub database: Option<StrictPath>,
}

impl Lutris {
    pub const FLATPAK_SUFFIX_DATA: &str = ".var/app/net.lutris.Lutris/data/lutris";
    pub const FLATPAK_SUFFIX_CONFIG: &str = ".var/app/net.lutris.Lutris/config/lutris";
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Microsoft {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Origin {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Prime {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Steam {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

impl Steam {
    pub const FLATPAK_SUFFIX: &str = ".var/app/com.valvesoftware.Steam/.steam/steam";
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Uplay {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherHome {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherWine {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherWindows {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherLinux {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherMac {
    /// Where the root is located on your system.
    pub path: StrictPath,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(default, rename_all = "camelCase")]
pub struct Other {
    /// Where the root is located on your system.
    pub path: StrictPath,
}
