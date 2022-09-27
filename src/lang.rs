use byte_unit::Byte;
use fluent::{bundle::FluentBundle, FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Mutex;
use unic_langid::LanguageIdentifier;

use crate::{
    config::{BackupFormat, SortKey, Theme, ZipCompression},
    manifest::Store,
    prelude::{Error, OperationStatus, OperationStepDecision, StrictPath},
};

const PATH: &str = "path";
const PATH_ACTION: &str = "path-action";
const PROCESSED_GAMES: &str = "processed-games";
const PROCESSED_SIZE: &str = "processed-size";
const TOTAL_GAMES: &str = "total-games";
const TOTAL_SIZE: &str = "total-size";

// TODO: Some are blocked by https://github.com/mtkennerly/ludusavi/issues/9.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[allow(dead_code)]
    #[serde(rename = "ar-SA")]
    Arabic,
    #[allow(dead_code)]
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
    #[default]
    #[serde(rename = "en-US")]
    English,
    #[serde(rename = "eo")]
    Esperanto,
    #[serde(rename = "fil-PH")]
    Filipino,
    #[serde(rename = "de-DE")]
    German,
    #[serde(rename = "it-IT")]
    Italian,
    #[allow(dead_code)]
    #[serde(rename = "ko-KR")]
    Korean,
    #[serde(rename = "pt-BR")]
    PortugueseBrazilian,
    #[serde(rename = "pl-PL")]
    Polish,
    #[serde(rename = "es-ES")]
    Spanish,
}

impl Language {
    pub const ALL: &'static [Self] = &[
        Self::German,
        Self::English,
        Self::Esperanto,
        Self::Spanish,
        Self::Filipino,
        Self::Italian,
        Self::Polish,
        Self::PortugueseBrazilian,
    ];

    pub fn id(&self) -> LanguageIdentifier {
        let id = match self {
            Self::Arabic => "ar-SA",
            Self::ChineseSimplified => "zh-Hans",
            Self::English => "en-US",
            Self::Esperanto => "eo",
            Self::Filipino => "fil-PH",
            Self::German => "de-DE",
            Self::Italian => "it-IT",
            Self::Korean => "ko-KR",
            Self::Polish => "pl-PL",
            Self::PortugueseBrazilian => "pt-BR",
            Self::Spanish => "es-ES",
        };
        id.parse().unwrap()
    }
}

impl ToString for Language {
    fn to_string(&self) -> String {
        match self {
            Self::Arabic => "العربية (47%)",
            Self::ChineseSimplified => "中文（简体） (66%)",
            Self::English => "English",
            Self::Esperanto => "Esperanto (26%)",
            Self::Filipino => "Filipino (63%)",
            Self::German => "Deutsch (99%)",
            Self::Italian => "Italiano (100%)",
            Self::Korean => "한국어 (52%)",
            Self::Polish => "Polski (99%)",
            Self::PortugueseBrazilian => "Português brasileiro (96%)",
            Self::Spanish => "Español (89%)",
        }
        .to_string()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Translator {}

static BUNDLE: Lazy<Mutex<FluentBundle<FluentResource, IntlLangMemoizer>>> = Lazy::new(|| {
    let ftl = include_str!("../lang/en-US.ftl").to_owned();
    let res = FluentResource::try_new(ftl).expect("Failed to parse Fluent file content.");

    let mut bundle = FluentBundle::new_concurrent(vec![Language::English.id()]);
    bundle.set_use_isolating(false);

    bundle
        .add_resource(res)
        .expect("Failed to add Fluent resources to the bundle.");

    Mutex::new(bundle)
});

fn set_language(language: Language) {
    let mut bundle = BUNDLE.lock().unwrap();

    let ftl = match language {
        Language::Arabic => include_str!("../lang/ar-SA.ftl"),
        Language::ChineseSimplified => include_str!("../lang/zh-CN.ftl"),
        Language::English => include_str!("../lang/en-US.ftl"),
        Language::Esperanto => include_str!("../lang/eo-UY.ftl"),
        Language::Filipino => include_str!("../lang/fil-PH.ftl"),
        Language::German => include_str!("../lang/de-DE.ftl"),
        Language::Italian => include_str!("../lang/it-IT.ftl"),
        Language::Korean => include_str!("../lang/ko-KR.ftl"),
        Language::Polish => include_str!("../lang/pl-PL.ftl"),
        Language::PortugueseBrazilian => include_str!("../lang/pt-BR.ftl"),
        Language::Spanish => include_str!("../lang/es-ES.ftl"),
    }
    .to_owned();

    let res = FluentResource::try_new(ftl).expect("Failed to parse Fluent file content.");
    bundle.locales = vec![language.id()];

    bundle.add_resource_overriding(res);
}

static RE_EXTRA_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r#"([^\r\n ]) {2,}"#).unwrap());
static RE_EXTRA_LINES: Lazy<Regex> = Lazy::new(|| Regex::new(r#"([^\r\n ])[\r\n]([^\r\n ])"#).unwrap());
static RE_EXTRA_PARAGRAPHS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"([^\r\n ])[\r\n]{2,}([^\r\n ])"#).unwrap());

fn translate(id: &str) -> String {
    translate_args(id, &FluentArgs::new())
}

fn translate_args(id: &str, args: &FluentArgs) -> String {
    let bundle = match BUNDLE.lock() {
        Ok(x) => x,
        Err(_) => return "fluent-cannot-lock".to_string(),
    };

    let parts: Vec<&str> = id.splitn(2, '.').collect();
    let (name, attr) = if parts.len() < 2 {
        (id, None)
    } else {
        (parts[0], Some(parts[1]))
    };

    let message = match bundle.get_message(name) {
        Some(x) => x,
        None => return format!("fluent-no-message={}", name),
    };

    let pattern = match attr {
        None => match message.value() {
            Some(x) => x,
            None => return format!("fluent-no-message-value={}", id),
        },
        Some(attr) => match message.get_attribute(attr) {
            Some(x) => x.value(),
            None => return format!("fluent-no-attr={}", id),
        },
    };
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, Some(args), &mut errors);

    RE_EXTRA_PARAGRAPHS
        .replace_all(
            &RE_EXTRA_LINES.replace_all(&RE_EXTRA_SPACES.replace_all(&value, "${1} "), "${1} ${2}"),
            "${1}\n\n${2}",
        )
        .to_string()
}

impl Translator {
    pub fn set_language(&self, language: Language) {
        set_language(Language::English);
        if language != Language::English {
            set_language(language);
        }
    }

    pub fn window_title(&self) -> String {
        let name = translate("ludusavi");
        let version = option_env!("LUDUSAVI_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        match option_env!("LUDUSAVI_VARIANT") {
            Some(variant) => format!("{} v{} ({})", name, version, variant),
            None => format!("{} v{}", name, version),
        }
    }

    pub fn pcgamingwiki(&self) -> String {
        "PCGamingWiki".to_string()
    }

    pub fn handle_error(&self, error: &Error) -> String {
        match error {
            Error::ConfigInvalid { why } => self.config_is_invalid(why),
            Error::ManifestInvalid { why } => self.manifest_is_invalid(why),
            Error::ManifestCannotBeUpdated => self.manifest_cannot_be_updated(),
            Error::CliBackupTargetExists { path } => self.cli_backup_target_exists(path),
            Error::CliUnrecognizedGames { games } => self.cli_unrecognized_games(games),
            Error::CliUnableToRequestConfirmation => self.cli_unable_to_request_confirmation(),
            Error::CliBackupIdWithMultipleGames => self.cli_backup_id_with_multiple_games(),
            Error::CliInvalidBackupId => self.cli_invalid_backup_id(),
            Error::SomeEntriesFailed => self.some_entries_failed(),
            Error::CannotPrepareBackupTarget { path } => self.cannot_prepare_backup_target(path),
            Error::RestorationSourceInvalid { path } => self.restoration_source_is_invalid(path),
            Error::RegistryIssue => self.registry_issue(),
            Error::UnableToBrowseFileSystem => self.unable_to_browse_file_system(),
            Error::UnableToOpenDir(path) => self.unable_to_open_dir(path),
            Error::UnableToOpenUrl(url) => self.unable_to_open_url(url),
        }
    }

    pub fn cli_backup_target_exists(&self, path: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, path.render());
        translate_args("cli-backup-target-already-exists", &args)
    }

    pub fn cli_unrecognized_games(&self, games: &[String]) -> String {
        let prefix = translate("cli-unrecognized-games");
        let lines: Vec<_> = games.iter().map(|x| format!("  - {}", x)).collect();
        format!("{}\n{}", prefix, lines.join("\n"))
    }

    pub fn cli_confirm_restoration(&self, path: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, path.render());
        translate_args("cli-confirm-restoration", &args)
    }

    pub fn cli_unable_to_request_confirmation(&self) -> String {
        #[cfg(target_os = "windows")]
        let extra_note = translate("cli-unable-to-request-confirmation.winpty-workaround");

        #[cfg(not(target_os = "windows"))]
        let extra_note = "";

        format!("{} {}", translate("cli-unable-to-request-confirmation"), extra_note)
    }

    pub fn cli_backup_id_with_multiple_games(&self) -> String {
        translate("cli-backup-id-with-multiple-games")
    }

    pub fn cli_invalid_backup_id(&self) -> String {
        translate("cli-invalid-backup-id")
    }

    pub fn some_entries_failed(&self) -> String {
        translate("some-entries-failed")
    }

    fn label(&self, text: &str) -> String {
        format!("[{}]", text)
    }

    pub fn label_failed(&self) -> String {
        self.label(&self.badge_failed())
    }

    pub fn label_duplicates(&self) -> String {
        self.label(&self.badge_duplicates())
    }

    pub fn label_duplicated(&self) -> String {
        self.label(&self.badge_duplicated())
    }

    pub fn label_ignored(&self) -> String {
        self.label(&self.badge_ignored())
    }

    fn field(&self, text: &str) -> String {
        format!("{}:", text)
    }

    pub fn field_language(&self) -> String {
        self.field(&translate("language"))
    }

    pub fn field_theme(&self) -> String {
        self.field(&translate("theme"))
    }

    pub fn badge_failed(&self) -> String {
        translate("badge-failed")
    }

    pub fn badge_duplicates(&self) -> String {
        translate("badge-duplicates")
    }

    pub fn badge_duplicated(&self) -> String {
        translate("badge-duplicated")
    }

    pub fn badge_ignored(&self) -> String {
        translate("badge-ignored")
    }

    pub fn badge_redirected_from(&self, original: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, original.render());
        translate_args("badge-redirected-from", &args)
    }

    pub fn cli_game_header(
        &self,
        name: &str,
        bytes: u64,
        decision: &OperationStepDecision,
        duplicated: bool,
    ) -> String {
        let mut labels = vec![];
        if *decision == OperationStepDecision::Ignored {
            labels.push(self.label_ignored());
        }
        if duplicated {
            labels.push(self.label_duplicates());
        }

        if labels.is_empty() {
            format!("{} [{}]:", name, self.adjusted_size(bytes))
        } else {
            format!("{} [{}] {}:", name, self.adjusted_size(bytes), labels.join(" "))
        }
    }

    pub fn cli_game_line_item(&self, item: &str, successful: bool, ignored: bool, duplicated: bool) -> String {
        let mut parts = vec![];
        if !successful {
            parts.push(self.label_failed());
        }
        if ignored {
            parts.push(self.label_ignored());
        }
        if duplicated {
            parts.push(self.label_duplicated());
        }
        parts.push(item.to_string());

        format!("  - {}", parts.join(" "))
    }

    pub fn cli_game_line_item_redirected(&self, item: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, item);
        translate_args("cli-game-line-redirected-from", &args)
    }

    pub fn cli_summary(&self, status: &OperationStatus, location: &StrictPath) -> String {
        format!(
            "{}:\n  {}: {}\n  {}: {}\n  {}: {}",
            translate("overall"),
            translate("total-games"),
            if status.processed_all_games() {
                status.processed_games.to_string()
            } else {
                format!("{} / {}", status.processed_games, status.total_games)
            },
            translate("file-size"),
            if status.processed_all_bytes() {
                self.adjusted_size(status.processed_bytes)
            } else {
                format!(
                    "{} / {}",
                    self.adjusted_size(status.processed_bytes),
                    self.adjusted_size(status.total_bytes)
                )
            },
            translate("file-location"),
            location.render(),
        )
    }

    pub fn backup_button(&self) -> String {
        translate("button-backup")
    }

    pub fn preview_button(&self) -> String {
        translate("button-preview")
    }

    pub fn restore_button(&self) -> String {
        translate("button-restore")
    }

    pub fn nav_backup_button(&self) -> String {
        translate("button-nav-backup")
    }

    pub fn nav_restore_button(&self) -> String {
        translate("button-nav-restore")
    }

    pub fn nav_custom_games_button(&self) -> String {
        translate("button-nav-custom-games")
    }

    pub fn nav_other_button(&self) -> String {
        translate("button-nav-other")
    }

    pub fn add_root_button(&self) -> String {
        translate("button-add-root")
    }

    pub fn find_roots_button(&self) -> String {
        translate("button-find-roots")
    }

    pub fn customize_button(&self) -> String {
        translate("button-customize")
    }

    pub fn no_missing_roots(&self) -> String {
        translate("no-missing-roots")
    }

    pub fn preparing_backup_dir(&self) -> String {
        translate("preparing-backup-target")
    }

    pub fn updating_manifest(&self) -> String {
        translate("updating-manifest")
    }

    pub fn confirm_add_missing_roots(&self, roots: &[crate::config::RootsConfig]) -> String {
        use std::fmt::Write;
        let mut msg = translate("confirm-add-missing-roots") + "\n";

        for root in roots {
            let _ = &write!(msg, "\n[{}] {}", self.store(&root.store), root.path.render());
        }

        msg
    }

    pub fn add_redirect_button(&self) -> String {
        translate("button-add-redirect")
    }

    pub fn add_game_button(&self) -> String {
        translate("button-add-game")
    }

    pub fn continue_button(&self) -> String {
        translate("button-continue")
    }

    pub fn cancel_button(&self) -> String {
        translate("button-cancel")
    }

    pub fn cancelling_button(&self) -> String {
        translate("button-cancelling")
    }

    pub fn okay_button(&self) -> String {
        translate("button-okay")
    }

    pub fn select_all_button(&self) -> String {
        translate("button-select-all")
    }

    pub fn deselect_all_button(&self) -> String {
        translate("button-deselect-all")
    }

    pub fn enable_all_button(&self) -> String {
        translate("button-enable-all")
    }

    pub fn disable_all_button(&self) -> String {
        translate("button-disable-all")
    }

    pub fn no_roots_are_configured(&self) -> String {
        translate("no-roots-are-configured")
    }

    pub fn config_is_invalid(&self, why: &str) -> String {
        format!("{}\n{}", translate("config-is-invalid"), why)
    }

    pub fn manifest_is_invalid(&self, why: &str) -> String {
        format!("{}\n{}", translate("manifest-is-invalid"), why)
    }

    pub fn manifest_cannot_be_updated(&self) -> String {
        translate("manifest-cannot-be-updated")
    }

    pub fn cannot_prepare_backup_target(&self, target: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, target.render());
        translate_args("cannot-prepare-backup-target", &args)
    }

    pub fn restoration_source_is_invalid(&self, source: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, source.render());
        translate_args("restoration-source-is-invalid", &args)
    }

    pub fn registry_issue(&self) -> String {
        translate("registry-issue")
    }

    pub fn unable_to_browse_file_system(&self) -> String {
        translate("unable-to-browse-file-system")
    }

    pub fn unable_to_open_dir(&self, path: &StrictPath) -> String {
        format!("{}\n\n{}", translate("unable-to-open-directory"), path.render())
    }

    pub fn unable_to_open_url(&self, url: &str) -> String {
        format!("{}\n\n{}", translate("unable-to-open-url"), url)
    }

    pub fn adjusted_size(&self, bytes: u64) -> String {
        let byte = Byte::from_bytes(bytes.into());
        let adjusted_byte = byte.get_appropriate_unit(true);
        adjusted_byte.to_string()
    }

    pub fn processed_games(&self, status: &OperationStatus) -> String {
        let mut args = FluentArgs::new();
        args.set(TOTAL_GAMES, status.total_games);
        args.set(PROCESSED_GAMES, status.processed_games);

        if status.processed_all_games() {
            translate_args("processed-games", &args)
        } else {
            translate_args("processed-games-subset", &args)
        }
    }

    pub fn processed_bytes(&self, status: &OperationStatus) -> String {
        if status.processed_all_bytes() {
            self.adjusted_size(status.total_bytes)
        } else {
            let mut args = FluentArgs::new();
            args.set(TOTAL_SIZE, self.adjusted_size(status.total_bytes));
            args.set(PROCESSED_SIZE, self.adjusted_size(status.processed_bytes));
            translate_args("processed-size-subset", &args)
        }
    }

    pub fn processed_subset(&self, total: usize, processed: usize) -> String {
        let mut args = FluentArgs::new();
        args.set(TOTAL_SIZE, total as u64);
        args.set(PROCESSED_SIZE, processed as u64);
        translate_args("processed-size-subset", &args)
    }

    pub fn backup_target_label(&self) -> String {
        translate("field-backup-target")
    }

    pub fn backup_merge_label(&self) -> String {
        translate("toggle-backup-merge")
    }

    pub fn restore_source_label(&self) -> String {
        translate("field-restore-source")
    }

    pub fn custom_files_label(&self) -> String {
        translate("field-custom-files")
    }

    pub fn custom_registry_label(&self) -> String {
        translate("field-custom-registry")
    }

    pub fn search_label(&self) -> String {
        translate("field-search")
    }

    pub fn sort_label(&self) -> String {
        translate("field-sort")
    }

    pub fn store(&self, store: &Store) -> String {
        translate(match store {
            Store::Epic => "store-epic",
            Store::Gog => "store-gog",
            Store::GogGalaxy => "store-gog-galaxy",
            Store::Microsoft => "store-microsoft",
            Store::Origin => "store-origin",
            Store::Prime => "store-prime",
            Store::Steam => "store-steam",
            Store::Uplay => "store-uplay",
            Store::OtherHome => "store-other-home",
            Store::OtherWine => "store-other-wine",
            Store::Other => "store-other",
        })
    }

    pub fn sort_key(&self, key: &SortKey) -> String {
        translate(match key {
            SortKey::Name => "game-name",
            SortKey::Size => "file-size",
        })
    }

    pub fn sort_reversed(&self) -> String {
        translate("sort-reversed")
    }

    pub fn backup_format(&self, key: &BackupFormat) -> String {
        translate(match key {
            BackupFormat::Simple => "backup-format-simple",
            BackupFormat::Zip => "backup-format-zip",
        })
    }

    pub fn backup_compression(&self, key: &ZipCompression) -> String {
        translate(match key {
            ZipCompression::None => "compression-none",
            ZipCompression::Deflate => "compression-deflate",
            ZipCompression::Bzip2 => "compression-bzip2",
            ZipCompression::Zstd => "compression-zstd",
        })
    }

    pub fn theme_name(&self, theme: &Theme) -> String {
        translate(match theme {
            Theme::Light => "theme-light",
            Theme::Dark => "theme-dark",
        })
    }

    pub fn redirect_source_placeholder(&self) -> String {
        translate("field-redirect-source.placeholder")
    }

    pub fn redirect_target_placeholder(&self) -> String {
        translate("field-redirect-target.placeholder")
    }

    pub fn custom_game_name_placeholder(&self) -> String {
        translate("game-name")
    }

    pub fn search_game_name_placeholder(&self) -> String {
        translate("game-name")
    }

    pub fn explanation_for_exclude_store_screenshots(&self) -> String {
        translate("explanation-for-exclude-store-screenshots")
    }

    pub fn ignored_items_label(&self) -> String {
        translate("field-backup-excluded-items")
    }

    pub fn full_retention(&self) -> String {
        translate("field-retention-full")
    }

    pub fn differential_retention(&self) -> String {
        translate("field-retention-differential")
    }

    pub fn backup_format_field(&self) -> String {
        translate("field-backup-format")
    }

    pub fn backup_compression_field(&self) -> String {
        translate("field-backup-compression")
    }

    fn consider_doing_a_preview(&self) -> String {
        translate("consider-doing-a-preview")
    }

    pub fn modal_confirm_backup(&self, target: &StrictPath, target_exists: bool, merge: bool) -> String {
        let mut args = FluentArgs::new();
        args.set(
            PATH_ACTION,
            match (target_exists, merge) {
                (false, _) => "create",
                (true, false) => "recreate",
                (true, true) => "merge",
            },
        );
        format!(
            "{}\n\n{}\n\n{}",
            translate_args("confirm-backup", &args),
            target.render(),
            self.consider_doing_a_preview(),
        )
    }

    pub fn modal_confirm_restore(&self, source: &StrictPath) -> String {
        format!(
            "{}\n\n{}\n\n{}",
            translate("confirm-restore"),
            source.render(),
            self.consider_doing_a_preview(),
        )
    }
}
