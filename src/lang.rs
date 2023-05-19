use std::sync::Mutex;

use byte_unit::Byte;
use fluent::{bundle::FluentBundle, FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use once_cell::sync::Lazy;
use regex::Regex;
use unic_langid::LanguageIdentifier;

use crate::{
    prelude::{CommandError, Error, StrictPath, VARIANT, VERSION},
    resource::{
        config::{BackupFormat, RedirectKind, RootsConfig, SortKey, Theme, ZipCompression},
        manifest::Store,
    },
    scan::{game_filter, OperationStatus, OperationStepDecision, ScanChange},
};

const PATH: &str = "path";
const LOCAL_PATH: &str = "local-path";
const CLOUD_PATH: &str = "cloud-path";
const PATH_ACTION: &str = "path-action";
const PROCESSED_GAMES: &str = "processed-games";
const PROCESSED_SIZE: &str = "processed-size";
const TOTAL: &str = "total";
const TOTAL_GAMES: &str = "total-games";
const TOTAL_SIZE: &str = "total-size";
const COMMAND: &str = "command";
const CODE: &str = "code";
const MESSAGE: &str = "message";
const APP: &str = "app";

pub const TRANSLATOR: Translator = Translator {};
pub const ADD_SYMBOL: &str = "+";
pub const CHANGE_SYMBOL: &str = "Δ";
pub const REMOVAL_SYMBOL: &str = "x";

fn title_case(text: &str) -> String {
    let lowercase = text.to_lowercase();
    let mut chars = lowercase.chars();
    match chars.next() {
        None => lowercase,
        Some(char) => format!("{}{}", char.to_uppercase(), chars.as_str()),
    }
}

// TODO: Some are blocked by https://github.com/mtkennerly/ludusavi/issues/9.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[allow(dead_code)]
    #[serde(rename = "ar-SA")]
    Arabic,
    #[allow(dead_code)]
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
    #[serde(rename = "nl-NL")]
    Dutch,
    #[default]
    #[serde(rename = "en-US")]
    English,
    #[serde(rename = "eo")]
    Esperanto,
    #[serde(rename = "fil-PH")]
    Filipino,
    #[serde(rename = "fr-FR")]
    French,
    #[serde(rename = "de-DE")]
    German,
    #[serde(rename = "it-IT")]
    Italian,
    #[allow(dead_code)]
    #[serde(rename = "ja-JP")]
    Japanese,
    #[allow(dead_code)]
    #[serde(rename = "ko-KR")]
    Korean,
    #[serde(rename = "pt-BR")]
    PortugueseBrazilian,
    #[serde(rename = "pl-PL")]
    Polish,
    #[allow(dead_code)]
    #[serde(rename = "ja-JP")]
    Russian,
    #[serde(rename = "ru-RU")]
    Spanish,
    #[allow(dead_code)]
    #[serde(rename = "uk-UA")]
    Ukrainian,
}

impl Language {
    pub const ALL: &'static [Self] = &[
        Self::German,
        Self::English,
        Self::Spanish,
        Self::Esperanto,
        Self::Filipino,
        Self::French,
        Self::Italian,
        Self::Dutch,
        Self::Polish,
        Self::PortugueseBrazilian,
        Self::Russian,
        Self::Ukrainian,
    ];

    pub fn id(&self) -> LanguageIdentifier {
        let id = match self {
            Self::Arabic => "ar-SA",
            Self::ChineseSimplified => "zh-Hans",
            Self::Dutch => "nl-NL",
            Self::English => "en-US",
            Self::Esperanto => "eo",
            Self::Filipino => "fil-PH",
            Self::French => "fr-FR",
            Self::German => "de-DE",
            Self::Italian => "it-IT",
            Self::Japanese => "ja-JP",
            Self::Korean => "ko-KR",
            Self::Polish => "pl-PL",
            Self::PortugueseBrazilian => "pt-BR",
            Self::Russian => "ru-RU",
            Self::Spanish => "es-ES",
            Self::Ukrainian => "uk-UA",
        };
        id.parse().unwrap()
    }
}

impl ToString for Language {
    fn to_string(&self) -> String {
        match self {
            Self::Arabic => "العربية (64%)",
            Self::ChineseSimplified => "中文（简体） (64%)",
            Self::Dutch => "Nederlands (25%)",
            Self::English => "English",
            Self::Esperanto => "Esperanto (22%)",
            Self::Filipino => "Filipino (41%)",
            Self::French => "Français (99%)",
            Self::German => "Deutsch (100%)",
            Self::Italian => "Italiano (64%)",
            Self::Japanese => "日本語 (55%)",
            Self::Korean => "한국어 (33%)",
            Self::Polish => "Polski (65%)",
            Self::PortugueseBrazilian => "Português brasileiro (95%)",
            Self::Russian => "Русский язык (20%)",
            Self::Spanish => "Español (54%)",
            Self::Ukrainian => "Украї́нська мо́ва (7%)",
        }
        .to_string()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Translator {}

static LANGUAGE: Mutex<Language> = Mutex::new(Language::English);

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
        Language::Dutch => include_str!("../lang/nl-NL.ftl"),
        Language::English => include_str!("../lang/en-US.ftl"),
        Language::Esperanto => include_str!("../lang/eo-UY.ftl"),
        Language::Filipino => include_str!("../lang/fil-PH.ftl"),
        Language::French => include_str!("../lang/fr-FR.ftl"),
        Language::German => include_str!("../lang/de-DE.ftl"),
        Language::Italian => include_str!("../lang/it-IT.ftl"),
        Language::Japanese => include_str!("../lang/ja-JP.ftl"),
        Language::Korean => include_str!("../lang/ko-KR.ftl"),
        Language::Polish => include_str!("../lang/pl-PL.ftl"),
        Language::PortugueseBrazilian => include_str!("../lang/pt-BR.ftl"),
        Language::Russian => include_str!("../lang/ru-RU.ftl"),
        Language::Spanish => include_str!("../lang/es-ES.ftl"),
        Language::Ukrainian => include_str!("../lang/uk-UA.ftl"),
    }
    .to_owned();

    let res = FluentResource::try_new(ftl).expect("Failed to parse Fluent file content.");
    bundle.locales = vec![language.id()];

    bundle.add_resource_overriding(res);

    let mut last_language = LANGUAGE.lock().unwrap();
    *last_language = language;
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
        match VARIANT {
            Some(variant) => format!("{} v{} ({})", name, *VERSION, variant),
            None => format!("{} v{}", name, *VERSION),
        }
    }

    pub fn pcgamingwiki(&self) -> String {
        "PCGamingWiki".to_string()
    }

    pub fn comment_button(&self) -> String {
        translate("button-comment")
    }

    pub fn lock_button(&self) -> String {
        translate("button-lock")
    }

    pub fn unlock_button(&self) -> String {
        translate("button-unlock")
    }

    pub fn handle_error(&self, error: &Error) -> String {
        match error {
            Error::ConfigInvalid { why } => self.config_is_invalid(why),
            Error::ManifestInvalid { why } => self.manifest_is_invalid(why),
            Error::ManifestCannotBeUpdated => self.manifest_cannot_be_updated(),
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
            Error::RcloneUnavailable => self.rclone_unavailable(),
            Error::CloudNotConfigured => self.cloud_not_configured(),
            Error::CloudPathInvalid => self.cloud_path_invalid(),
            Error::UnableToConfigureCloud(error) => {
                format!(
                    "{}\n\n{}",
                    self.prefix_error(&self.unable_to_configure_cloud()),
                    self.handle_command_error(error)
                )
            }
            Error::UnableToSynchronizeCloud(error) => {
                format!(
                    "{}\n\n{}",
                    self.prefix_error(&self.unable_to_synchronize_with_cloud()),
                    self.handle_command_error(error)
                )
            }
            Error::CloudConflict => TRANSLATOR.prefix_error(&TRANSLATOR.cloud_synchronize_conflict()),
        }
    }

    fn handle_command_error(&self, error: &CommandError) -> String {
        let mut args = FluentArgs::new();
        args.set(COMMAND, error.command());
        match error {
            CommandError::Launched { raw, .. } => {
                format!("{}\n\n{}", translate_args("command-unlaunched", &args), raw)
            }
            CommandError::Terminated { .. } => translate_args("command-terminated", &args),
            CommandError::Exited {
                code, stdout, stderr, ..
            } => {
                args.set(CODE, code);
                let mut out = translate_args("command-failed", &args);

                if let Some(stdout) = stdout {
                    out.push_str("\n\n");
                    out.push_str(stdout);
                }

                if let Some(stderr) = stderr {
                    out.push_str("\n\n");
                    out.push_str(stderr);
                }

                out
            }
        }
    }

    pub fn cli_unrecognized_games(&self, games: &[String]) -> String {
        let prefix = translate("cli-unrecognized-games");
        let lines: Vec<_> = games.iter().map(|x| format!("  - {}", x)).collect();
        format!("{}\n{}", prefix, lines.join("\n"))
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

    pub fn cloud_not_configured(&self) -> String {
        translate("cloud-not-configured")
    }

    pub fn cloud_path_invalid(&self) -> String {
        translate("cloud-path-invalid")
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
        let language = LANGUAGE.lock().unwrap();
        match *language {
            Language::French => format!("{} :", text),
            _ => format!("{}:", text),
        }
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

    pub fn badge_redirecting_to(&self, path: &StrictPath) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, path.render());
        translate_args("badge-redirecting-to", &args)
    }

    pub fn cli_game_header(
        &self,
        name: &str,
        bytes: u64,
        decision: &OperationStepDecision,
        duplicated: bool,
        change: ScanChange,
    ) -> String {
        let mut labels = vec![];
        match change {
            ScanChange::New => {
                labels.push(format!("[{}]", crate::lang::ADD_SYMBOL));
            }
            ScanChange::Different => {
                labels.push(format!("[{}]", crate::lang::CHANGE_SYMBOL));
            }
            ScanChange::Removed | ScanChange::Same | ScanChange::Unknown => (),
        }
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

    pub fn cli_game_line_item(
        &self,
        item: &str,
        successful: bool,
        ignored: bool,
        duplicated: bool,
        change: ScanChange,
        nested: bool,
    ) -> String {
        let mut parts = vec![];
        match change {
            ScanChange::Same | ScanChange::Unknown => (),
            ScanChange::New => parts.push(format!("[{}]", ADD_SYMBOL)),
            ScanChange::Different => parts.push(format!("[{}]", CHANGE_SYMBOL)),
            ScanChange::Removed => parts.push(format!("[{}]", REMOVAL_SYMBOL)),
        }
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

        if nested {
            format!("    - {}", parts.join(" "))
        } else {
            format!("  - {}", parts.join(" "))
        }
    }

    pub fn cli_game_line_item_redirected(&self, item: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, item);
        format!("    - {}", translate_args("cli-game-line-item-redirected", &args),)
    }

    pub fn cli_game_line_item_redirecting(&self, item: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH, item);
        format!("    - {}", translate_args("cli-game-line-item-redirecting", &args),)
    }

    pub fn cli_summary(&self, status: &OperationStatus, location: &StrictPath) -> String {
        let new_games = if status.changed_games.new > 0 {
            format!(" [{}{}]", crate::lang::ADD_SYMBOL, status.changed_games.new)
        } else {
            "".to_string()
        };
        let changed_games = if status.changed_games.different > 0 {
            format!(" [{}{}]", crate::lang::CHANGE_SYMBOL, status.changed_games.different)
        } else {
            "".to_string()
        };

        format!(
            "{}:\n  {}: {}{}{}\n  {}: {}\n  {}: {}",
            translate("overall"),
            translate("total-games"),
            if status.processed_all_games() {
                status.processed_games.to_string()
            } else {
                format!("{} / {}", status.processed_games, status.total_games)
            },
            new_games,
            changed_games,
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

    pub fn backup_button_no_confirmation(&self) -> String {
        format!("{} ({})", self.backup_button(), self.suffix_no_confirmation())
    }

    pub fn preview_button(&self) -> String {
        translate("button-preview")
    }

    pub fn preview_button_in_custom_mode(&self) -> String {
        format!("{} ({})", self.preview_button(), self.backup_button().to_lowercase())
    }

    pub fn restore_button(&self) -> String {
        translate("button-restore")
    }

    pub fn restore_button_no_confirmation(&self) -> String {
        format!("{} ({})", self.restore_button(), self.suffix_no_confirmation())
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

    pub fn customize_button(&self) -> String {
        translate("button-customize")
    }

    pub fn no_missing_roots(&self) -> String {
        translate("no-missing-roots")
    }

    pub fn loading(&self) -> String {
        translate("loading")
    }

    pub fn updating_manifest(&self) -> String {
        translate("updating-manifest")
    }

    pub fn confirm_add_missing_roots(&self, roots: &[RootsConfig]) -> String {
        use std::fmt::Write;
        let mut msg = translate("confirm-add-missing-roots") + "\n";

        for root in roots {
            let _ = &write!(msg, "\n[{}] {}", self.store(&root.store), root.path.render());
        }

        msg
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

    pub fn exit_button(&self) -> String {
        translate("button-exit")
    }

    pub fn get_rclone_button(&self) -> String {
        let mut args = FluentArgs::new();
        args.set(APP, "Rclone");
        translate_args("button-get-app", &args)
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

    pub fn unable_to_configure_cloud(&self) -> String {
        translate("unable-to-configure-cloud")
    }

    pub fn unable_to_synchronize_with_cloud(&self) -> String {
        translate("unable-to-synchronize-with-cloud")
    }

    pub fn cloud_synchronize_conflict(&self) -> String {
        translate("cloud-synchronize-conflict")
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

    pub fn restore_source_label(&self) -> String {
        translate("field-restore-source")
    }

    pub fn custom_files_label(&self) -> String {
        translate("field-custom-files")
    }

    pub fn custom_registry_label(&self) -> String {
        translate("field-custom-registry")
    }

    pub fn sort_label(&self) -> String {
        translate("field-sort")
    }

    pub fn store(&self, store: &Store) -> String {
        translate(match store {
            Store::Ea => "store-ea",
            Store::Epic => "store-epic",
            Store::Gog => "store-gog",
            Store::GogGalaxy => "store-gog-galaxy",
            Store::Heroic => "store-heroic",
            Store::Lutris => "store-lutris",
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
            SortKey::Status => "status",
        })
    }

    pub fn filter_uniqueness(&self, filter: game_filter::Uniqueness) -> String {
        match filter {
            game_filter::Uniqueness::Unique => translate("label-unique"),
            game_filter::Uniqueness::Duplicate => title_case(&self.badge_duplicated()),
        }
    }

    pub fn filter_completeness(&self, filter: game_filter::Completeness) -> String {
        translate(match filter {
            game_filter::Completeness::Complete => "label-complete",
            game_filter::Completeness::Partial => "label-partial",
        })
    }

    pub fn filter_enablement(&self, filter: game_filter::Enablement) -> String {
        translate(match filter {
            game_filter::Enablement::Enabled => "label-enabled",
            game_filter::Enablement::Disabled => "label-disabled",
        })
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

    pub fn redirect_kind(&self, redirect: &RedirectKind) -> String {
        match redirect {
            RedirectKind::Backup => self.backup_button(),
            RedirectKind::Restore => self.restore_button(),
            RedirectKind::Bidirectional => translate("redirect-bidirectional"),
        }
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

    pub fn show_deselected_games(&self) -> String {
        translate("show-deselected-games")
    }

    pub fn show_unchanged_games(&self) -> String {
        translate("show-unchanged-games")
    }

    pub fn show_unscanned_games(&self) -> String {
        translate("show-unscanned-games")
    }

    pub fn override_max_threads(&self) -> String {
        format!(
            "{} ({})",
            translate("override-max-threads"),
            self.suffix_restart_required()
        )
    }

    pub fn explanation_for_exclude_store_screenshots(&self) -> String {
        translate("explanation-for-exclude-store-screenshots")
    }

    pub fn roots_label(&self) -> String {
        translate("field-roots")
    }

    pub fn ignored_items_label(&self) -> String {
        translate("field-backup-excluded-items")
    }

    pub fn redirects_label(&self) -> String {
        translate("field-redirects")
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

    pub fn backup_compression_level_field(&self) -> String {
        translate("field-backup-compression-level")
    }

    pub fn manifest_label(&self) -> String {
        self.field(&translate("label-manifest"))
    }

    pub fn checked_label(&self) -> String {
        self.field(&translate("label-checked"))
    }

    pub fn updated_label(&self) -> String {
        self.field(&translate("label-updated"))
    }

    pub fn comment_label(&self) -> String {
        translate("label-comment")
    }

    pub fn scan_label(&self) -> String {
        translate("label-scan")
    }

    pub fn scan_field(&self) -> String {
        self.field(&self.scan_label())
    }

    pub fn filter_label(&self) -> String {
        self.field(&translate("label-filter"))
    }

    pub fn threads_label(&self) -> String {
        self.field(&translate("label-threads"))
    }

    pub fn cloud_label(&self) -> String {
        translate("label-cloud")
    }

    pub fn cloud_field(&self) -> String {
        self.field(&self.cloud_label())
    }

    pub fn rclone_label(&self) -> String {
        self.field("Rclone")
    }

    pub fn remote_label(&self) -> String {
        self.field(&translate("label-remote"))
    }

    pub fn remote_name_label(&self) -> String {
        self.field(&translate("label-remote-name"))
    }

    pub fn folder_label(&self) -> String {
        self.field(&translate("label-folder"))
    }

    pub fn executable_label(&self) -> String {
        translate("label-executable")
    }

    pub fn arguments_label(&self) -> String {
        translate("label-arguments")
    }

    pub fn url_label(&self) -> String {
        self.field(&translate("label-url"))
    }

    pub fn host_label(&self) -> String {
        self.field(&translate("label-host"))
    }

    pub fn port_label(&self) -> String {
        self.field(&translate("label-port"))
    }

    pub fn username_label(&self) -> String {
        self.field(&translate("label-username"))
    }

    pub fn password_label(&self) -> String {
        self.field(&translate("label-password"))
    }

    pub fn provider_label(&self) -> String {
        self.field(&translate("label-provider"))
    }

    pub fn none_label(&self) -> String {
        translate("label-none")
    }

    pub fn custom_label(&self) -> String {
        translate("label-custom")
    }

    pub fn change_count_label(&self, total: usize) -> String {
        let mut args = FluentArgs::new();
        args.set(TOTAL, total);
        translate_args("label-change-count", &args)
    }

    pub fn synchronize_automatically(&self) -> String {
        translate("synchronize-automatically")
    }

    pub fn total_games(&self) -> String {
        translate("total-games")
    }

    pub fn new_tooltip(&self) -> String {
        translate("label-new")
    }

    pub fn updated_tooltip(&self) -> String {
        translate("label-updated")
    }

    pub fn removed_tooltip(&self) -> String {
        translate("label-removed")
    }

    fn consider_doing_a_preview(&self) -> String {
        translate("consider-doing-a-preview")
    }

    pub fn confirm_backup(&self, target: &StrictPath, target_exists: bool, suggest: bool) -> String {
        let mut args = FluentArgs::new();
        args.set(PATH_ACTION, if !target_exists { "create" } else { "merge" });
        let primary = translate_args("confirm-backup", &args);

        if suggest {
            format!(
                "{}\n\n{}\n\n{}",
                primary,
                target.render(),
                self.consider_doing_a_preview(),
            )
        } else {
            format!("{}\n\n{}", primary, target.render(),)
        }
    }

    pub fn confirm_restore(&self, source: &StrictPath, suggest: bool) -> String {
        let primary = translate("confirm-restore");

        if suggest {
            format!(
                "{}\n\n{}\n\n{}",
                primary,
                source.render(),
                self.consider_doing_a_preview(),
            )
        } else {
            format!("{}\n\n{}", primary, source.render(),)
        }
    }

    pub fn confirm_cloud_upload(&self, local: &str, cloud: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(LOCAL_PATH, local);
        args.set(CLOUD_PATH, cloud);
        translate_args("confirm-cloud-upload", &args)
    }

    pub fn confirm_cloud_download(&self, local: &str, cloud: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(LOCAL_PATH, local);
        args.set(CLOUD_PATH, cloud);
        translate_args("confirm-cloud-download", &args)
    }

    pub fn no_cloud_changes(&self) -> String {
        translate("no-cloud-changes")
    }

    pub fn notify_single_game_status(&self, found: bool) -> String {
        if found {
            translate("saves-found")
        } else {
            translate("no-saves-found")
        }
    }

    pub fn suffix_no_confirmation(&self) -> String {
        translate("suffix-no-confirmation")
    }

    pub fn suffix_restart_required(&self) -> String {
        translate("suffix-restart-required")
    }

    pub fn prefix_error(&self, message: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(MESSAGE, message);
        translate_args("prefix-error", &args)
    }

    pub fn prefix_warning(&self, message: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(MESSAGE, message);
        translate_args("prefix-warning", &args)
    }

    pub fn rclone_unavailable(&self) -> String {
        let mut args = FluentArgs::new();
        args.set(APP, "Rclone");
        translate_args("cloud-app-unavailable", &args)
    }

    pub fn cloud_progress(&self, processed_bytes: u64, total_bytes: u64) -> String {
        format!(
            "{} / {}",
            self.adjusted_size(processed_bytes),
            self.adjusted_size(total_bytes)
        )
    }
}
