use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    lang::TRANSLATOR,
    prelude::{app_dir, Error, StrictPath},
    resource::{
        cache::{self, Cache},
        config::{Config, CustomGame, ManifestConfig},
        ResourceFile, SaveableResourceFile,
    },
    scan::layout::escape_folder_name,
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
    #[serde(rename = "legendary")]
    Legendary,
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
    #[serde(rename = "otherWindows")]
    OtherWindows,
    #[serde(rename = "otherLinux")]
    OtherLinux,
    #[serde(rename = "otherMac")]
    OtherMac,
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
        Store::Legendary,
        Store::Lutris,
        Store::Microsoft,
        Store::Origin,
        Store::Prime,
        Store::Steam,
        Store::Uplay,
        Store::OtherHome,
        Store::OtherWine,
        Store::OtherWindows,
        Store::OtherLinux,
        Store::OtherMac,
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
    pub alias: Option<String>,
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
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub gog_extra: BTreeSet<u64>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub steam_extra: BTreeSet<u32>,
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
    fn file_name_for(url: &str, primary: bool) -> String {
        if primary {
            Self::FILE_NAME.to_string()
        } else {
            let encoded = escape_folder_name(url.trim_end_matches(".yaml"));
            format!("manifest-{}.yaml", encoded)
        }
    }

    pub fn path_for(url: &str, primary: bool) -> StrictPath {
        if primary {
            Self::path()
        } else {
            app_dir().joined(&Self::file_name_for(url, primary))
        }
    }

    pub fn load() -> Result<Self, Error> {
        ResourceFile::load().map_err(|e| Error::ManifestInvalid {
            why: format!("{}", e),
            identifier: None,
        })
    }

    pub fn should_update(url: &str, cache: &cache::Manifests, force: bool, primary: bool) -> bool {
        if force {
            return true;
        }
        if !Self::path_for(url, primary).exists() {
            return true;
        }
        match cache.get(url) {
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
    ) -> Vec<Result<Option<ManifestUpdate>, Error>> {
        let mut out = vec![];

        out.push(Self::update_one(&config.url, &cache, force, true));

        for secondary in config.secondary_manifest_urls() {
            out.push(Self::update_one(secondary, &cache, force, false));
        }

        out
    }

    fn update_one(
        url: &str,
        cache: &cache::Manifests,
        force: bool,
        primary: bool,
    ) -> Result<Option<ManifestUpdate>, Error> {
        let identifier = (!primary).then(|| url.to_string());
        let cannot_update = || Error::ManifestCannotBeUpdated {
            identifier: identifier.clone(),
        };

        if !Self::should_update(url, cache, force, primary) {
            return Ok(None);
        }

        let path = Self::path_for(url, primary);

        let mut req = reqwest::blocking::Client::new().get(url);
        let old_etag = cache.get(url).and_then(|x| x.etag.clone());
        if let Some(etag) = old_etag.as_ref() {
            if path.exists() {
                req = req.header(reqwest::header::IF_NONE_MATCH, etag);
            }
        }
        let mut res = req.send().map_err(|_e| cannot_update())?;
        match res.status() {
            reqwest::StatusCode::OK => {
                app_dir().create_dirs().map_err(|_| cannot_update())?;

                // Ensure that the manifest data is valid before we save it.
                let mut manifest_bytes = vec![];
                res.copy_to(&mut manifest_bytes).map_err(|_| cannot_update())?;
                let manifest_string = String::from_utf8(manifest_bytes).map_err(|_| cannot_update())?;
                if let Err(e) = Self::load_from_string(&manifest_string) {
                    return Err(Error::ManifestInvalid {
                        why: e.to_string(),
                        identifier: identifier.clone(),
                    });
                }

                path.write_with_content(&manifest_string).map_err(|_| cannot_update())?;

                let new_etag = res
                    .headers()
                    .get(reqwest::header::ETAG)
                    .map(|etag| String::from_utf8_lossy(etag.as_bytes()).to_string());

                Ok(Some(ManifestUpdate {
                    url: url.to_string(),
                    etag: new_etag,
                    timestamp: chrono::offset::Utc::now(),
                    modified: true,
                }))
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(Some(ManifestUpdate {
                url: url.to_string(),
                etag: old_etag,
                timestamp: chrono::offset::Utc::now(),
                modified: false,
            })),
            _ => Err(cannot_update()),
        }
    }

    pub fn update_mut(config: &Config, cache: &mut Cache, force: bool) -> Result<(), Error> {
        let mut error = None;

        let updates = Self::update(config.manifest.clone(), cache.manifests.clone(), force);
        for update in updates {
            match update {
                Ok(Some(update)) => {
                    cache.update_manifest(update);
                    cache.save();
                }
                Ok(None) => {}
                Err(e) => {
                    if error.is_none() {
                        error = Some(e);
                    }
                }
            }
        }

        if let Some(error) = error {
            return Err(error);
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

    pub fn incorporate_extensions(&mut self, config: &Config) {
        for (path, secondary) in config.manifest.load_secondary_manifests() {
            self.incorporate_secondary_manifest(path, secondary);
        }

        for root in &config.roots {
            for (path, secondary) in root.find_secondary_manifests() {
                self.incorporate_secondary_manifest(path, secondary);
            }
        }

        for custom_game in &config.custom_games {
            if custom_game.ignore {
                continue;
            }
            self.add_custom_game(custom_game.clone());
        }
    }

    pub fn with_extensions(mut self, config: &Config) -> Self {
        self.incorporate_extensions(config);
        self
    }

    fn add_custom_game(&mut self, custom: CustomGame) {
        let name = custom.name.clone();
        let existing = self.0.get(&name);

        let game = Game {
            alias: custom.alias,
            files: (!custom.files.is_empty()).then(|| {
                custom
                    .files
                    .into_iter()
                    .map(|x| (x, GameFileEntry::default()))
                    .collect()
            }),
            install_dir: existing.and_then(|x| x.install_dir.clone()),
            registry: (!custom.registry.is_empty()).then(|| {
                custom
                    .registry
                    .into_iter()
                    .map(|x| (x, GameRegistryEntry::default()))
                    .collect()
            }),
            steam: existing.and_then(|x| x.steam.clone()),
            gog: existing.and_then(|x| x.gog.clone()),
            id: existing.and_then(|x| x.id.clone()),
        };

        self.0.insert(name, game);
    }

    fn incorporate_secondary_manifest(&mut self, path: StrictPath, secondary: Manifest) {
        log::debug!("incorporating secondary manifest: {}", path.render());
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
                        standard_id.gog_extra.extend(secondary_id.gog_extra);
                        standard_id.steam_extra.extend(secondary_id.steam_extra);
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

    pub fn processable_titles(&self) -> impl Iterator<Item = &String> {
        self.processable_games().map(|(k, _)| k)
    }

    pub fn processable_games(&self) -> impl Iterator<Item = (&String, &Game)> {
        self.0.iter().filter(|(_, v)| {
            let Game {
                alias,
                files,
                install_dir: _,
                registry,
                steam,
                gog,
                id,
            } = &v;
            alias.is_none()
                && (files.is_some() || registry.is_some() || steam.is_some() || gog.is_some() || id.is_some())
        })
    }

    pub fn primary_titles(&self) -> BTreeSet<String> {
        self.0
            .iter()
            .filter_map(|(k, v)| v.alias.is_none().then_some(k))
            .cloned()
            .collect()
    }

    pub fn aliases(&self) -> HashMap<String, String> {
        self.0
            .keys()
            .filter_map(|k| {
                let mut i = 0;
                let mut lookup = k;
                loop {
                    if i >= 100 {
                        break None;
                    }

                    let game = self.0.get(lookup)?;
                    match game.alias.as_ref() {
                        Some(alias) => {
                            lookup = alias;
                        }
                        None => {
                            if i == 0 {
                                break None;
                            } else {
                                break Some((k.to_string(), lookup.to_string()));
                            }
                        }
                    }

                    i += 1;
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_map};

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
                alias: None,
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
              alias: other
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
                alias: Some("other".to_string()),
                files: Some(btree_map! {
                    s("foo"): GameFileEntry {
                        when: Some(vec![
                            GameFileConstraint {
                                os: Some(Os::Windows),
                                store: Some(Store::Steam),
                            }
                        ]),
                        tags: Some(vec![Tag::Save]),
                    }
                }),
                install_dir: Some(btree_map! {
                    s("ExampleGame"): GameInstallDirEntry {}
                }),
                registry: Some(btree_map! {
                    s("bar"): GameRegistryEntry {
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
                    gog_extra: vec![10, 11].into_iter().collect(),
                    steam_extra: vec![1, 2].into_iter().collect(),
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

    #[test]
    fn can_get_aliases() {
        let manifest = Manifest::load_from_string(
            r#"
            foo: {}
            bar:
              alias: foo
            baz:
              alias: bar
            "#,
        )
        .unwrap();

        assert_eq!(
            hash_map! {
                "bar".to_string(): "foo".to_string(),
                "baz".to_string(): "foo".to_string(),
            },
            manifest.aliases(),
        );
    }
}
