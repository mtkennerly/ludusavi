use std::collections::{BTreeSet, HashMap};

use crate::{
    prelude::{app_dir, StrictPath},
    resource::{
        config::{Config, RootsConfig},
        manifest::ManifestUpdate,
        ResourceFile, SaveableResourceFile,
    },
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
    pub recent_games: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Restore {
    #[serde(default)]
    pub recent_games: BTreeSet<String>,
}

impl ResourceFile for Cache {
    const FILE_NAME: &'static str = "cache.yaml";
}

impl SaveableResourceFile for Cache {}

impl Cache {
    pub fn migrate_config(mut self, config: &mut Config) -> Self {
        let mut updated = false;

        if !self.migrations.adopted_cache {
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

    pub fn update_manifest(&mut self, update: ManifestUpdate) {
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
