use std::collections::{BTreeMap, BTreeSet};

use crate::{
    lang::Language,
    prelude::{app_dir, CANONICAL_VERSION},
    resource::{
        config::{self, Config, Root},
        manifest::ManifestUpdate,
        ResourceFile, SaveableResourceFile,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Cache {
    pub version: Option<(u32, u32, u32)>,
    pub release: Release,
    pub migrations: Migrations,
    pub manifests: Manifests,
    pub roots: BTreeSet<Root>,
    pub backup: Backup,
    pub restore: Restore,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Release {
    pub checked: chrono::DateTime<chrono::Utc>,
    pub latest: Option<semver::Version>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Migrations {
    pub adopted_cache: bool,
    pub fixed_spanish_config: bool,
    pub set_default_manifest_url_to_null: bool,
}

pub type Manifests = BTreeMap<String, Manifest>;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub etag: Option<String>,
    pub checked: Option<chrono::DateTime<chrono::Utc>>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Backup {
    pub recent_games: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Restore {
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

        if !self.migrations.set_default_manifest_url_to_null {
            if config
                .manifest
                .url
                .as_ref()
                .is_some_and(|url| url == config::MANIFEST_URL)
            {
                config.manifest.url = None;
            }
            self.migrations.set_default_manifest_url_to_null = true;
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

    pub fn add_roots(&mut self, roots: &Vec<Root>) {
        for root in roots {
            if !self.has_root(root) {
                self.roots.insert(root.clone());
            }
        }
    }

    pub fn has_root(&self, candidate: &Root) -> bool {
        self.roots.iter().any(|root| {
            let primary = root.path().equivalent(candidate.path()) && root.store() == candidate.store();
            match (root, candidate) {
                (Root::Lutris(root), Root::Lutris(candidate)) => {
                    primary && (root.database.is_some() || candidate.database.is_none())
                }
                _ => primary,
            }
        })
    }

    pub fn should_check_app_update(&self) -> bool {
        let now = chrono::offset::Utc::now();
        now.signed_duration_since(self.release.checked).num_hours() >= 24
    }
}
