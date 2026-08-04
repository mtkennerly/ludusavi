//! Finding the Wine/Proton prefix a game's saves should be written to on *this* machine.
//!
//! A backup records the prefix its files came from, but that path is meaningless on
//! another device: both the username and the Steam compatdata app ID differ (the latter
//! even for the same game, since non-Steam shortcuts get a per-machine generated ID).
//! Restoring therefore needs a local answer, and the sources for one are ranked here.

use crate::{
    path::StrictPath,
    resource::{
        config::{Config, Root},
        manifest::{Manifest, Os, Store},
        sync_state::{self, SyncStateFile},
    },
    scan::{launchers::Launchers, semantic::Prefix, steam::SteamShortcuts},
};

/// Everything needed to locate a game's Wine prefix on this machine.
///
/// Built once per operation and borrowed across the per-game loop.
pub struct WineEnvironment<'a> {
    pub config: &'a Config,
    pub manifest: &'a Manifest,
    pub roots: &'a [Root],
    pub steam_shortcuts: &'a SteamShortcuts,
    /// Only needed for Heroic/Lutris games; the Steam compatdata path doesn't use it.
    pub launchers: Option<&'a Launchers>,
    /// From `--wine-prefix`. Highest priority when present.
    pub cli_prefix: Option<&'a StrictPath>,
    /// Per-device prefixes recorded in `settings.config`. Authoritative when it has an
    /// entry for this device, since it was written by the machine that owns the prefix.
    pub registry: Option<&'a SyncStateFile>,
}

impl<'a> WineEnvironment<'a> {
    /// Validated local prefixes for a game, most-specific first.
    ///
    /// Every candidate goes through [`Prefix::validated`], so entries pointing at a
    /// prefix that no longer exists are dropped and the next source is tried. That makes
    /// a stale registry entry self-healing rather than fatal.
    pub fn prefixes_for_game(&self, game_name: &str) -> Vec<Prefix> {
        let mut candidates: Vec<StrictPath> = Vec::new();

        if let Some(cli) = self.cli_prefix {
            candidates.push(cli.clone());
        }

        // Recorded by whichever machine actually owns this prefix, at backup time.
        if let Some(registry) = self.registry {
            let device = sync_state::current_device();
            if let Some(recorded) = registry.prefix_for(game_name, &device) {
                candidates.push(StrictPath::new(recorded));
            }
        }

        for custom in self.config.custom_games.iter().filter(|cg| cg.name == game_name) {
            for wp in custom.wine_prefix.iter().filter(|wp| !wp.trim().is_empty()) {
                candidates.push(StrictPath::new(wp));
            }
        }

        // At restore the game may exist only as a restorable backup, absent from the
        // manifest, so this is checked separately from the custom games above.
        if let Some(game) = self.manifest.0.get(game_name) {
            for wp in game.wine_prefix.iter().filter(|wp| !wp.trim().is_empty()) {
                candidates.push(StrictPath::new(wp));
            }
        }

        candidates.extend(self.steam_compatdata_candidates(game_name));

        if let Some(launchers) = self.launchers {
            for root in self.roots {
                for wp in launchers.get_game(root, game_name).filter_map(|x| x.prefix.as_ref()) {
                    candidates.push(wp.clone());
                    let pfx = wp.joined("pfx");
                    if pfx.exists() {
                        candidates.push(pfx);
                    }
                }
            }
        }

        let mut seen: Vec<String> = Vec::new();
        let mut valid = Vec::new();
        for candidate in candidates {
            let rendered = candidate.render();
            if seen.contains(&rendered) {
                continue;
            }
            seen.push(rendered.clone());

            match Prefix::validated(&candidate) {
                Some(prefix) => valid.push(prefix),
                None => log::debug!("Not a usable Wine prefix for {game_name}: {rendered}"),
            }
        }

        valid
    }

    /// Steam Proton prefixes, at `<steam root>/steamapps/compatdata/<app id>/pfx`.
    ///
    /// The shortcut ID is tried before the manifest's Steam ID: a shortcut exists only
    /// because the user explicitly registered a specific install, which is the stronger
    /// signal about where they actually play the game.
    fn steam_compatdata_candidates(&self, game_name: &str) -> Vec<StrictPath> {
        if Os::HOST != Os::Linux {
            return vec![];
        }

        let shortcut_id = self.steam_shortcuts.get(game_name).map(|x| x.id);
        let manifest_game = self.manifest.0.get(game_name);

        let mut ids: Vec<u32> = Vec::new();
        ids.extend(shortcut_id);
        if let Some(game) = manifest_game {
            for id in game.all_ids().steam(None) {
                ids.push(id);
            }
        }

        let mut candidates = Vec::new();
        for root in self.roots.iter().filter(|root| root.store() == Store::Steam) {
            for id in &ids {
                candidates.push(root.path().joined(format!("steamapps/compatdata/{id}/pfx")));
            }
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{
        config::CustomGame,
        manifest::{Game, SteamMetadata},
        sync_state::GameSyncEntry,
    };

    fn make_valid_prefix(prefix: &str) {
        let _ = std::fs::create_dir_all(format!("{prefix}/drive_c/users/steamuser"));
        let _ = std::fs::create_dir_all(format!("{prefix}/drive_c/users/Public"));
        let _ = std::fs::create_dir_all(format!("{prefix}/dosdevices"));
        let _ = std::fs::File::create(format!("{prefix}/system.reg"));
    }

    fn compatdata(steam_root: &str, id: u32) -> String {
        format!("{steam_root}/steamapps/compatdata/{id}/pfx")
    }

    fn manifest_with_steam_id(game_name: &str, id: u32) -> Manifest {
        let mut manifest = Manifest::default();
        manifest.0.insert(
            game_name.to_string(),
            Game {
                steam: SteamMetadata { id: Some(id) },
                ..Default::default()
            },
        );
        manifest
    }

    fn registry_with_prefix(game_name: &str, prefix: &str) -> SyncStateFile {
        let mut state = SyncStateFile::default();
        state.merge_game(
            game_name,
            GameSyncEntry {
                last_push: chrono::Utc::now(),
                device: sync_state::current_device(),
                mapping_path: format!("{game_name}/mapping.yaml"),
                prefixes: Default::default(),
            },
        );
        state.set_prefix(game_name, &sync_state::current_device(), prefix);
        state
    }

    struct Fixture {
        config: Config,
        manifest: Manifest,
        roots: Vec<Root>,
        shortcuts: SteamShortcuts,
    }

    impl Fixture {
        fn new(steam_root: &str, game_name: &str, steam_id: u32) -> Self {
            Self {
                config: Config::default(),
                manifest: manifest_with_steam_id(game_name, steam_id),
                roots: vec![Root::new(StrictPath::new(steam_root), Store::Steam)],
                shortcuts: SteamShortcuts::default(),
            }
        }

        fn env<'a>(&'a self, registry: Option<&'a SyncStateFile>) -> WineEnvironment<'a> {
            WineEnvironment {
                config: &self.config,
                manifest: &self.manifest,
                roots: &self.roots,
                steam_shortcuts: &self.shortcuts,
                launchers: None,
                cli_prefix: None,
                registry,
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_compatdata_prefix_for_manifest_steam_id() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let prefix = compatdata(steam_root, 1086940);
        make_valid_prefix(&prefix);

        let fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        let found = fixture.env(None).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(1, found.len());
        assert_eq!(StrictPath::new(prefix).render(), found[0].path.render());
        assert_eq!("steamuser", found[0].wine_user);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn registry_entry_outranks_discovery() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let discovered = compatdata(steam_root, 1086940);
        let recorded = format!("{steam_root}/recorded-prefix");
        make_valid_prefix(&discovered);
        make_valid_prefix(&recorded);

        let fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        let registry = registry_with_prefix("Baldur's Gate 3", &recorded);
        let found = fixture.env(Some(&registry)).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(
            StrictPath::new(recorded).render(),
            found[0].path.render(),
            "the prefix recorded by this device should win over a guess"
        );
        assert_eq!(2, found.len(), "discovery should still be offered as a fallback");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn falls_back_to_discovery_when_registry_has_no_entry_for_this_device() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let prefix = compatdata(steam_root, 1086940);
        make_valid_prefix(&prefix);

        // Registry knows the game, but only for some other machine.
        let mut registry = registry_with_prefix("Baldur's Gate 3", "/home/deck/other");
        registry.set_prefix("Baldur's Gate 3", "some-other-device", "/home/deck/other");
        registry
            .games
            .get_mut("Baldur's Gate 3")
            .unwrap()
            .prefixes
            .remove(&sync_state::current_device());

        let fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        let found = fixture.env(Some(&registry)).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(1, found.len());
        assert_eq!(StrictPath::new(prefix).render(), found[0].path.render());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn skips_registry_entry_whose_prefix_no_longer_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let prefix = compatdata(steam_root, 1086940);
        make_valid_prefix(&prefix);

        let registry = registry_with_prefix("Baldur's Gate 3", &format!("{steam_root}/deleted-prefix"));

        let fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        let found = fixture.env(Some(&registry)).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(1, found.len(), "stale registry entries should self-heal");
        assert_eq!(StrictPath::new(prefix).render(), found[0].path.render());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn shortcut_id_outranks_manifest_id() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let by_manifest = compatdata(steam_root, 1086940);
        let by_shortcut = compatdata(steam_root, 2811670038);
        make_valid_prefix(&by_manifest);
        make_valid_prefix(&by_shortcut);

        let mut fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        fixture.shortcuts.set_for_test("Baldur's Gate 3", 2811670038);
        let found = fixture.env(None).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(
            StrictPath::new(by_shortcut).render(),
            found[0].path.render(),
            "an explicitly registered shortcut is the stronger signal"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn custom_game_wine_prefix_outranks_compatdata() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        let discovered = compatdata(steam_root, 1086940);
        let custom = format!("{steam_root}/custom-prefix");
        make_valid_prefix(&discovered);
        make_valid_prefix(&custom);

        let mut fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        fixture.config.custom_games.push(CustomGame {
            name: "Baldur's Gate 3".to_string(),
            wine_prefix: vec![custom.clone()],
            ..Default::default()
        });
        let found = fixture.env(None).prefixes_for_game("Baldur's Gate 3");

        assert_eq!(StrictPath::new(custom).render(), found[0].path.render());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn ignores_compatdata_that_is_not_a_valid_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();
        // Directory exists but has none of the Wine markers.
        let _ = std::fs::create_dir_all(compatdata(steam_root, 1086940));

        let fixture = Fixture::new(steam_root, "Baldur's Gate 3", 1086940);
        assert!(fixture.env(None).prefixes_for_game("Baldur's Gate 3").is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn returns_empty_for_game_with_no_prefix_anywhere() {
        let tmp = tempfile::tempdir().unwrap();
        let steam_root = tmp.path().to_str().unwrap();

        let fixture = Fixture::new(steam_root, "Some Native Game", 12345);
        assert!(fixture.env(None).prefixes_for_game("Some Native Game").is_empty());
    }
}
