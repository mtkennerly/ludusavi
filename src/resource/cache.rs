use std::collections::{BTreeSet, HashMap};

use crate::{
    lang::Language,
    prelude::{app_dir, CANONICAL_VERSION},
    resource::{
        config::{Config, RootsConfig},
        manifest::ManifestUpdate,
        ResourceFile, SaveableResourceFile,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    #[serde(default)]
    pub version: Option<(u32, u32, u32)>,
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
    #[serde(default)]
    pub fixed_spanish_config: bool,
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
            let _ = app_dir().joined(".flag_migrated_legacy_config").remove();
            self.migrations.adopted_cache = true;
            updated = true;
        }

        if !self.migrations.fixed_spanish_config && self.version.is_none() {
            if config.language == Language::Russian {
                config.language = Language::Spanish;
            }
            self.migrations.fixed_spanish_config = true;
            updated = true;
        }

        if self.roots.is_empty() && !config.roots.is_empty() {
            self.add_roots(&config.roots);
            updated = true;
        }

        if self.version != Some(*CANONICAL_VERSION) {
            self.version = Some(*CANONICAL_VERSION);
            updated = true;
        }

        if updated {
            self.save();
            config.save();
        }

        self
    }

    pub fn update_manifest(&mut self, update: ManifestUpdate) {
        let cached = self.manifests.entry(update.url).or_default();
        cached.etag = update.etag;
        cached.checked = Some(update.timestamp);
        if update.modified {
            cached.updated = Some(update.timestamp);
        }
    }

    pub fn add_roots(&mut self, roots: &Vec<RootsConfig>) {
        for root in roots {
            if !self.has_root(root) {
                self.roots.insert(root.clone());
            }
        }
    }

    pub fn has_root(&self, root: &RootsConfig) -> bool {
        self.roots
            .iter()
            .any(|x| x.path.equivalent(&root.path) && x.store == root.store)
    }
}
