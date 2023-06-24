use std::collections::{BTreeMap, HashMap};

use crate::{
    lang::TRANSLATOR,
    prelude::{app_dir, Error, StrictPath},
    resource::{
        cache::{self, Cache},
        config::{Config, CustomGame, ManifestConfig, RootsConfig},
        ResourceFile, SaveableResourceFile,
    },
};

pub mod placeholder {
    pub const ROOT: &str = "<root>";
    pub const GAME: &str = "<game>";
    pub const BASE: &str = "<base>";
    pub const HOME: &str = "<home>";
    pub const STORE_USER_ID: &str = "<storeUserId>";
    pub const OS_USER_NAME: &str = "<osUserName>";
    pub const WIN_APP_DATA: &str = "<winAppData>";
    pub const WIN_LOCAL_APP_DATA: &str = "<winLocalAppData>";
    pub const WIN_DOCUMENTS: &str = "<winDocuments>";
    pub const WIN_PUBLIC: &str = "<winPublic>";
    pub const WIN_PROGRAM_DATA: &str = "<winProgramData>";
    pub const WIN_DIR: &str = "<winDir>";
    pub const XDG_DATA: &str = "<xdgData>";
    pub const XDG_CONFIG: &str = "<xdgConfig>";
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Os {
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "mac")]
    Mac,
    #[default]
    #[serde(other, rename = "other")]
    Other,
}

impl Os {
    const WINDOWS: bool = cfg!(target_os = "windows");
    const MAC: bool = cfg!(target_os = "macos");
    const LINUX: bool = cfg!(target_os = "linux");
    pub const HOST: Os = if Self::WINDOWS {
        Self::Windows
    } else if Self::MAC {
        Self::Mac
    } else if Self::LINUX {
        Self::Linux
    } else {
        Self::Other
    };

    pub fn is_case_sensitive(&self) -> bool {
        match self {
            Self::Windows | Self::Mac => false,
            Self::Linux | Self::Other => true,
        }
    }
}

impl From<&str> for Os {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            "mac" | "macos" => Self::Mac,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
pub enum Store {
    #[serde(rename = "ea")]
    Ea,
    #[serde(rename = "epic")]
    Epic,
    #[serde(rename = "gog")]
    Gog,
    #[serde(rename = "gogGalaxy")]
    GogGalaxy,
    #[serde(rename = "heroic")]
    Heroic,
    #[serde(rename = "lutris")]
    Lutris,
    #[serde(rename = "microsoft")]
    Microsoft,
    #[serde(rename = "origin")]
    Origin,
    #[serde(rename = "prime")]
    Prime,
    #[serde(rename = "steam")]
    Steam,
    #[serde(rename = "uplay")]
    Uplay,
    #[serde(rename = "otherHome")]
    OtherHome,
    #[serde(rename = "otherWine")]
    OtherWine,
    #[default]
    #[serde(other, rename = "other")]
    Other,
}

impl Store {
    pub const ALL: &'static [Self] = &[
        Store::Ea,
        Store::Epic,
        Store::Gog,
        Store::GogGalaxy,
        Store::Heroic,
        Store::Lutris,
        Store::Microsoft,
        Store::Origin,
        Store::Prime,
        Store::Steam,
        Store::Uplay,
        Store::OtherHome,
        Store::OtherWine,
        Store::Other,
    ];
}

impl ToString for Store {
    fn to_string(&self) -> String {
        TRANSLATOR.store(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tag {
    #[serde(rename = "save")]
    Save,
    #[serde(rename = "config")]
    Config,
    #[default]
    #[serde(other, rename = "other")]
    Other,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest(#[serde(serialize_with = "crate::serialization::ordered_map")] pub HashMap<String, Game>);

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Game {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<BTreeMap<String, GameFileEntry>>,
    #[serde(rename = "installDir", skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<BTreeMap<String, GameInstallDirEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<BTreeMap<String, GameRegistryEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam: Option<SteamMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gog: Option<GogMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdMetadata>,
}

impl Game {
    /// This is intended for secondary manifests.
    fn normalize_relative_paths(&mut self) {
        use placeholder::BASE;
        if let Some(files) = self.files.as_mut() {
            *files = files
                .iter_mut()
                .map(|(k, v)| {
                    let v = v.clone();
                    if let Some(k) = k.strip_prefix("./") {
                        (format!("{BASE}/{k}"), v)
                    } else if let Some(k) = k.strip_prefix(".\\") {
                        (format!("{BASE}/{k}"), v)
                    } else if let Some(k) = k.strip_prefix("../") {
                        (format!("{BASE}/../{k}"), v)
                    } else if let Some(k) = k.strip_prefix("..\\") {
                        (format!("{BASE}/../{k}"), v)
                    } else {
                        (k.clone(), v)
                    }
                })
                .collect();
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameFileEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Vec<GameFileConstraint>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameInstallDirEntry {}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Vec<GameRegistryConstraint>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameFileConstraint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<Store>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryConstraint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<Store>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SteamMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GogMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatpak: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gog_extra: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steam_extra: Vec<u32>,
}

impl From<CustomGame> for Game {
    fn from(item: CustomGame) -> Self {
        let file_tuples = item.files.iter().map(|x| (x.to_string(), GameFileEntry::default()));
        let files: BTreeMap<_, _> = file_tuples.collect();

        let registry_tuples = item
            .registry
            .iter()
            .map(|x| (x.to_string(), GameRegistryEntry::default()));
        let registry: BTreeMap<_, _> = registry_tuples.collect();

        Self {
            files: Some(files),
            install_dir: None,
            registry: Some(registry),
            steam: None,
            gog: None,
            id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestUpdate {
    pub url: String,
    pub etag: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub modified: bool,
}

impl ResourceFile for Manifest {
    const FILE_NAME: &'static str = "manifest.yaml";
}

impl Manifest {
    pub fn load() -> Result<Self, Error> {
        ResourceFile::load().map_err(|e| Error::ManifestInvalid { why: format!("{}", e) })
    }

    pub fn should_update(config: &ManifestConfig, cache: &cache::Manifests, force: bool) -> bool {
        if force {
            return true;
        }
        if !Self::path().exists() {
            return true;
        }
        match cache.get(&config.url) {
            None => true,
            Some(cached) => {
                let now = chrono::offset::Utc::now();
                now.signed_duration_since(cached.checked.unwrap_or_default())
                    .num_hours()
                    >= 24
            }
        }
    }

    pub fn update(
        config: ManifestConfig,
        cache: cache::Manifests,
        force: bool,
    ) -> Result<Option<ManifestUpdate>, Error> {
        if !Self::should_update(&config, &cache, force) {
            return Ok(None);
        }

        let mut req = reqwest::blocking::Client::new().get(&config.url);
        let old_etag = cache.get(&config.url).and_then(|x| x.etag.clone());
        if let Some(etag) = old_etag.as_ref() {
            if StrictPath::from_std_path_buf(&Self::path()).exists() {
                req = req.header(reqwest::header::IF_NONE_MATCH, etag);
            }
        }
        let mut res = req.send().map_err(|_e| Error::ManifestCannotBeUpdated)?;
        match res.status() {
            reqwest::StatusCode::OK => {
                std::fs::create_dir_all(app_dir()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                let mut file = std::fs::File::create(Self::path()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                res.copy_to(&mut file).map_err(|_| Error::ManifestCannotBeUpdated)?;

                let new_etag = res
                    .headers()
                    .get(reqwest::header::ETAG)
                    .map(|etag| String::from_utf8_lossy(etag.as_bytes()).to_string());

                Ok(Some(ManifestUpdate {
                    url: config.url,
                    etag: new_etag,
                    timestamp: chrono::offset::Utc::now(),
                    modified: true,
                }))
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(Some(ManifestUpdate {
                url: config.url,
                etag: old_etag,
                timestamp: chrono::offset::Utc::now(),
                modified: false,
            })),
            _ => Err(Error::ManifestCannotBeUpdated),
        }
    }

    pub fn update_mut(config: &Config, cache: &mut Cache, force: bool) -> Result<(), Error> {
        let updated = Self::update(config.manifest.clone(), cache.manifests.clone(), force)?;
        if let Some(updated) = updated {
            cache.update_manifest(updated);
            cache.save();
        }
        Ok(())
    }

    pub fn map_steam_ids_to_names(&self) -> HashMap<u32, String> {
        self.0
            .iter()
            .filter_map(|(k, v)| match &v.steam {
                None => None,
                Some(steam) => steam.id.map(|id| (id, k.to_owned())),
            })
            .collect()
    }

    pub fn map_gog_ids_to_names(&self) -> HashMap<u64, String> {
        self.0
            .iter()
            .filter_map(|(k, v)| match &v.gog {
                None => None,
                Some(gog) => gog.id.map(|id| (id, k.to_owned())),
            })
            .collect()
    }

    pub fn incorporate_extensions(&mut self, roots: &[RootsConfig], custom_games: &[CustomGame]) {
        for root in roots {
            for (path, secondary) in root.find_secondary_manifests() {
                self.incorporate_secondary_manifest(path, secondary);
            }
        }

        for custom_game in custom_games {
            if custom_game.ignore {
                continue;
            }
            self.add_custom_game(custom_game.clone());
        }
    }

    fn add_custom_game(&mut self, custom: CustomGame) {
        let name = custom.name.clone();
        let mut game: Game = custom.into();
        if let Some(existing) = self.0.get(&name) {
            game.steam = existing.steam.clone();
            game.install_dir = existing.install_dir.clone();
        }
        self.0.insert(name, game);
    }

    fn incorporate_secondary_manifest(&mut self, path: StrictPath, secondary: Manifest) {
        for (name, mut game) in secondary.0 {
            game.normalize_relative_paths();

            if let Some(standard) = self.0.get_mut(&name) {
                log::debug!("overriding game from secondary manifest: {name}");

                if let Some(secondary) = game.files {
                    if let Some(standard) = &mut standard.files {
                        standard.extend(secondary);
                    } else {
                        standard.files = Some(secondary);
                    }
                }

                if let Some(secondary) = game.registry {
                    if let Some(standard) = &mut standard.registry {
                        standard.extend(secondary);
                    } else {
                        standard.registry = Some(secondary);
                    }
                }

                if let Some(mut secondary) = game.install_dir {
                    if let Some(folder) = path.parent().and_then(|x| x.leaf()) {
                        secondary.insert(folder, GameInstallDirEntry {});
                    }

                    if let Some(standard) = &mut standard.install_dir {
                        standard.extend(secondary);
                    } else {
                        standard.install_dir = Some(secondary);
                    }
                }

                if let (None, Some(secondary)) = (standard.steam.as_ref(), game.steam) {
                    standard.steam = Some(secondary);
                }

                if let (None, Some(secondary)) = (standard.gog.as_ref(), game.gog) {
                    standard.gog = Some(secondary);
                }

                if let Some(secondary_id) = game.id {
                    if let Some(standard_id) = &mut standard.id {
                        if standard_id.flatpak.is_none() {
                            standard_id.flatpak = secondary_id.flatpak;
                        }
                    } else {
                        standard.id = Some(secondary_id);
                    }
                }
            } else {
                log::debug!("adding game from secondary manifest: {name}");

                if let Some(folder) = path.parent().and_then(|x| x.leaf()) {
                    if let Some(secondary) = &mut game.install_dir {
                        secondary.insert(folder, GameInstallDirEntry {});
                    } else {
                        game.install_dir = Some(BTreeMap::from([(folder, GameInstallDirEntry {})]));
                    }
                }

                self.0.insert(name, game);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::testing::s;

    #[test]
    fn can_parse_game_with_no_fields() {
        let manifest = Manifest::load_from_string(
            r#"
            game: {}
            "#,
        )
        .unwrap();

        assert_eq!(
            Game {
                files: None,
                install_dir: None,
                registry: None,
                steam: None,
                gog: None,
                id: None,
            },
            manifest.0["game"],
        );
    }

    #[test]
    fn can_parse_game_with_all_fields() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              files:
                foo:
                  when:
                    - os: windows
                      store: steam
                  tags:
                    - save
              installDir:
                ExampleGame: {}
              registry:
                bar:
                  when:
                    - store: epic
                  tags:
                    - config
              steam:
                id: 101
              gog:
                id: 102
              id:
                flatpak: com.example.Game
                gogExtra: [10, 11]
                steamExtra: [1, 2]
            "#,
        )
        .unwrap();

        assert_eq!(
            Game {
                files: Some(btreemap! {
                    s("foo") => GameFileEntry {
                        when: Some(vec![
                            GameFileConstraint {
                                os: Some(Os::Windows),
                                store: Some(Store::Steam),
                            }
                        ]),
                        tags: Some(vec![Tag::Save]),
                    }
                }),
                install_dir: Some(btreemap! {
                    s("ExampleGame") => GameInstallDirEntry {}
                }),
                registry: Some(btreemap! {
                    s("bar") => GameRegistryEntry {
                        when: Some(vec![
                            GameRegistryConstraint {
                                store: Some(Store::Epic),
                            }
                        ]),
                        tags: Some(vec![Tag::Config])
                    },
                }),
                steam: Some(SteamMetadata { id: Some(101) }),
                gog: Some(GogMetadata { id: Some(102) }),
                id: Some(IdMetadata {
                    flatpak: Some("com.example.Game".to_string()),
                    gog_extra: vec![10, 11],
                    steam_extra: vec![1, 2],
                }),
            },
            manifest.0["game"],
        );
    }

    #[test]
    fn can_parse_game_with_minimal_files() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              files: {}
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].files.as_ref().unwrap().is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_files_when() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              files:
                foo:
                  when: []
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].files.as_ref().unwrap()["foo"]
            .when
            .as_ref()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_files_when_item() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              files:
                foo:
                  when:
                    - {}
            "#,
        )
        .unwrap();

        assert_eq!(
            GameFileConstraint { os: None, store: None },
            manifest.0["game"].files.as_ref().unwrap()["foo"].when.as_ref().unwrap()[0],
        );
    }

    #[test]
    fn can_parse_game_with_minimal_files_tags() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              files:
                foo:
                  tags: []
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].files.as_ref().unwrap()["foo"]
            .tags
            .as_ref()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_install_dir() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              installDir: {}
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].install_dir.as_ref().unwrap().is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_registry() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              registry: {}
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].registry.as_ref().unwrap().is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_registry_when() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              registry:
                foo:
                  when: []
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].registry.as_ref().unwrap()["foo"]
            .when
            .as_ref()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_registry_when_item() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              registry:
                foo:
                  when:
                    - {}
            "#,
        )
        .unwrap();

        assert_eq!(
            GameRegistryConstraint { store: None },
            manifest.0["game"].registry.as_ref().unwrap()["foo"]
                .when
                .as_ref()
                .unwrap()[0],
        );
    }

    #[test]
    fn can_parse_game_with_minimal_registry_tags() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              registry:
                foo:
                  tags: []
            "#,
        )
        .unwrap();

        assert!(manifest.0["game"].registry.as_ref().unwrap()["foo"]
            .tags
            .as_ref()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn can_parse_game_with_minimal_steam() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              steam: {}
            "#,
        )
        .unwrap();

        assert_eq!(&SteamMetadata { id: None }, manifest.0["game"].steam.as_ref().unwrap());
    }

    #[test]
    fn can_parse_game_with_minimal_gog() {
        let manifest = Manifest::load_from_string(
            r#"
            game:
              gog: {}
            "#,
        )
        .unwrap();

        assert_eq!(&GogMetadata { id: None }, manifest.0["game"].gog.as_ref().unwrap());
    }
}
