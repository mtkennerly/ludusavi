use std::collections::{BTreeSet, HashMap};

use crate::{
    config::{Config, RootsConfig},
    prelude::{app_dir, StrictPath},
};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    #[serde(default)]
    pub migrations: Migrations,
    #[serde(default)]
    pub manifests: Manifests,
    #[serde(default)]
    pub roots: BTreeSet<RootsConfig>,
    #[serde(default)]
    pub backup: Backup,
    #[serde(default)]
    pub restore: Restore,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Migrations {
    #[serde(default)]
    pub adopted_cache: bool,
}

pub type Manifests = HashMap<String, Manifest>;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub checked: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Backup {
    #[serde(default)]
    pub recent_games: std::collections::BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Restore {
    #[serde(default)]
    pub recent_games: std::collections::BTreeSet<String>,
}

impl Cache {
    fn file() -> std::path::PathBuf {
        let mut path = app_dir();
        path.push("cache.yaml");
        path
    }

    pub fn save(&self) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old_content) = Self::load_raw() {
            if old_content == new_content {
                return;
            }
        }

        if std::fs::create_dir_all(app_dir()).is_ok() {
            std::fs::write(Self::file(), new_content.as_bytes()).unwrap();
        }
    }

    pub fn load() -> Self {
        if !std::path::Path::new(&Self::file()).exists() {
            return Self::default();
        }
        let content = Self::load_raw().unwrap();
        Self::load_from_string(&content)
    }

    fn load_raw() -> Result<String, Box<dyn std::error::Error>> {
        Ok(std::fs::read_to_string(Self::file())?)
    }

    pub fn load_from_string(content: &str) -> Self {
        match serde_yaml::from_str(content) {
            Ok(x) => x,
            Err(_) => Self::default(),
        }
    }

    #[allow(deprecated)]
    pub fn migrated(mut self, config: &mut Config) -> Self {
        let mut updated = false;

        if let Some(etag) = config.manifest.etag.take() {
            let mut manifest = self
                .manifests
                .entry(config.manifest.url.clone())
                .or_insert_with(Default::default);
            manifest.etag = Some(etag);
            if let Some(modified) = crate::manifest::Manifest::modified() {
                manifest.checked = Some(modified);
                manifest.updated = Some(modified);
            }
            updated = true;
        }

        if !self.migrations.adopted_cache {
            self.backup.recent_games.extend(config.backup.recent_games.drain(..));
            self.restore.recent_games.extend(config.restore.recent_games.drain(..));
            let _ = StrictPath::from(app_dir())
                .joined(".flag_migrated_legacy_config")
                .remove();
            self.migrations.adopted_cache = true;
            updated = true;
        }

        if self.roots.is_empty() && !config.roots.is_empty() {
            self.add_roots(&config.roots);
            updated = true;
        }

        if updated {
            self.save();
            config.save();
        }

        self
    }

    pub fn update_manifest(&mut self, update: crate::manifest::ManifestUpdate) {
        let mut cached = self.manifests.entry(update.url).or_insert_with(Default::default);
        cached.etag = update.etag;
        cached.checked = Some(update.timestamp);
        if update.modified {
            cached.updated = Some(update.timestamp);
        }
    }

    pub fn add_roots(&mut self, roots: &Vec<RootsConfig>) {
        for root in roots {
            if !self.has_root(root) {
                self.roots.insert(RootsConfig {
                    path: root.path.interpreted(),
                    store: root.store,
                });
            }
        }
    }

    pub fn has_root(&self, root: &RootsConfig) -> bool {
        self.roots
            .iter()
            .any(|x| x.path.interpret() == root.path.interpret() && x.store == root.store)
    }
}
