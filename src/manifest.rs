use crate::{
    config::{Config, CustomGame},
    prelude::{app_dir, Error, StrictPath},
};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Os {
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "mac")]
    Mac,
    #[serde(other)]
    Other,
}

impl Default for Os {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Store {
    #[serde(rename = "steam")]
    Steam,
    #[serde(other)]
    Other,
}

impl Default for Store {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Tag {
    #[serde(rename = "save")]
    Save,
    #[serde(rename = "config")]
    Config,
    #[serde(other)]
    Other,
}

impl Default for Tag {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manifest(pub std::collections::HashMap<String, Game>);

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub files: Option<std::collections::HashMap<String, GameFileEntry>>,
    #[serde(rename = "installDir")]
    pub install_dir: Option<std::collections::HashMap<String, GameInstallDirEntry>>,
    pub registry: Option<std::collections::HashMap<String, GameRegistryEntry>>,
    pub steam: Option<SteamMetadata>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameFileEntry {
    pub tags: Option<Vec<Tag>>,
    pub when: Option<Vec<GameFileConstraint>>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameInstallDirEntry {}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryEntry {
    pub tags: Option<Vec<Tag>>,
    pub when: Option<Vec<GameRegistryConstraint>>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameFileConstraint {
    pub os: Option<Os>,
    pub store: Option<Store>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryConstraint {
    pub store: Option<Store>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SteamMetadata {
    pub id: Option<u32>,
}

impl From<CustomGame> for Game {
    fn from(item: CustomGame) -> Self {
        let file_tuples = item.files.iter().map(|x| (x.to_string(), GameFileEntry::default()));
        let files: std::collections::HashMap<_, _> = file_tuples.collect();

        let registry_tuples = item
            .registry
            .iter()
            .map(|x| (x.to_string(), GameRegistryEntry::default()));
        let registry: std::collections::HashMap<_, _> = registry_tuples.collect();

        Self {
            files: Some(files),
            install_dir: None,
            registry: Some(registry),
            steam: None,
        }
    }
}

impl Manifest {
    fn file() -> std::path::PathBuf {
        let mut path = app_dir();
        path.push("manifest.yaml");
        path
    }

    pub fn load(config: &mut Config, update: bool) -> Result<Self, Error> {
        if update || !StrictPath::from_std_path_buf(&Self::file()).exists() {
            Self::update(config)?;
        }
        let content = std::fs::read_to_string(Self::file()).unwrap();
        Self::load_from_string(&content)
    }

    pub fn load_from_string(content: &str) -> Result<Self, Error> {
        serde_yaml::from_str(&content).map_err(|e| Error::ManifestInvalid { why: format!("{}", e) })
    }

    pub fn update(config: &mut Config) -> Result<(), Error> {
        let mut req = reqwest::blocking::Client::new().get(&config.manifest.url);
        if let Some(etag) = &config.manifest.etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let mut res = req.send().map_err(|_e| Error::ManifestCannotBeUpdated)?;
        match res.status() {
            reqwest::StatusCode::OK => {
                std::fs::create_dir_all(app_dir()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                let mut file = std::fs::File::create(Self::file()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                res.copy_to(&mut file).map_err(|_| Error::ManifestCannotBeUpdated)?;

                if let Some(etag) = res.headers().get(reqwest::header::ETAG) {
                    match &config.manifest.etag {
                        Some(old_etag) if etag == old_etag => (),
                        _ => {
                            config.manifest.etag = Some(String::from_utf8_lossy(etag.as_bytes()).to_string());
                            config.save();
                        }
                    }
                }

                Ok(())
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(()),
            _ => Err(Error::ManifestCannotBeUpdated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    fn s(text: &str) -> String {
        text.to_string()
    }

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
                id: 123
            "#,
        )
        .unwrap();

        assert_eq!(
            Game {
                files: Some(hashmap! {
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
                install_dir: Some(hashmap! {
                    s("ExampleGame") => GameInstallDirEntry {}
                }),
                registry: Some(hashmap! {
                    s("bar") => GameRegistryEntry {
                        when: Some(vec![
                            GameRegistryConstraint {
                                store: Some(Store::Other),
                            }
                        ]),
                        tags: Some(vec![Tag::Config])
                    },
                }),
                steam: Some(SteamMetadata { id: Some(123) }),
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
}
