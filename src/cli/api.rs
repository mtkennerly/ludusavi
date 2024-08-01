use std::io::Read;

use crate::{
    lang::TRANSLATOR,
    path::StrictPath,
    prelude::Error,
    resource::{config::Config, manifest::Manifest},
    scan::{layout::BackupLayout, TitleFinder, TitleQuery},
};

/// The full input to the `api` command.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// Override configuration.
    #[serde(default)]
    pub config: ConfigOverride,
    /// The order of the requests here will match the order of responses in the output.
    pub requests: Vec<Request>,
}

/// Overridden configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOverride {
    /// Directory where Ludusavi stores backups.
    pub backup_path: Option<StrictPath>,
}

/// The full output of the `api` command.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Output {
    Success {
        /// Responses to each request, in the same order as the request input.
        responses: Vec<Response>,
    },
    Failure {
        /// A top-level error not tied to any particular request.
        error: response::Error,
    },
}

/// An individual request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    FindTitle(request::FindTitle),
    CheckAppUpdate(request::CheckAppUpdate),
}

/// A response to an individual request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    Error(response::Error),
    FindTitle(response::FindTitle),
    CheckAppUpdate(response::CheckAppUpdate),
}

pub mod request {
    /// Find game titles
    ///
    /// Precedence: Steam ID -> GOG ID -> Lutris ID -> exact names -> normalized names.
    /// Once a match is found for one of these options,
    /// Ludusavi will stop looking and return that match.
    ///
    /// Depending on the options chosen, there may be multiple matches, but the default is a single match.
    ///
    /// Aliases will be resolved to the target title.
    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct FindTitle {
        /// Ensure the game is recognized in a backup context.
        pub backup: bool,
        /// Ensure the game is recognized in a restore context.
        pub restore: bool,
        /// Look up game by a Steam ID.
        pub steam_id: Option<u32>,
        /// Look up game by a GOG ID.
        pub gog_id: Option<u64>,
        /// Look up game by a Lutris slug.
        pub lutris_id: Option<String>,
        /// Look up game by an approximation of the title.
        /// Ignores capitalization, "edition" suffixes, year suffixes, and some special symbols.
        /// This may find multiple games for a single input.
        pub normalized: bool,
        /// Select games that are disabled.
        pub disabled: bool,
        /// Select games that have some saves disabled.
        pub partial: bool,
        /// Look up game by an exact title.
        /// With multiple values, they will be checked in the order given.
        pub names: Vec<String>,
    }

    /// Check whether an application update is available.
    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct CheckAppUpdate {}
}

pub mod response {
    use std::collections::BTreeSet;

    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct Error {
        /// Human-readable error message.
        pub message: String,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct FindTitle {
        /// Any matching titles found.
        pub titles: BTreeSet<String>,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct CheckAppUpdate {
        /// An available update.
        pub update: Option<AppUpdate>,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    #[serde(default, rename_all = "camelCase")]
    pub struct AppUpdate {
        /// New version number.
        pub version: String,
        /// Release URL to open in browser.
        pub url: String,
    }
}

fn parse_input(input: Option<String>) -> Result<Input, String> {
    if let Some(input) = input {
        let input = serde_json::from_str::<Input>(&input).map_err(|e| e.to_string())?;
        Ok(input)
    } else {
        use std::io::IsTerminal;

        let mut stdin = std::io::stdin();
        if stdin.is_terminal() {
            Ok(Input::default())
        } else {
            let mut bytes = vec![];
            let _ = stdin.read_to_end(&mut bytes);
            let raw = String::from_utf8_lossy(&bytes);
            let input = serde_json::from_str::<Input>(&raw).map_err(|e| e.to_string())?;
            Ok(input)
        }
    }
}

pub fn abort_error(error: Error) -> ! {
    let output = Output::Failure {
        error: response::Error {
            message: TRANSLATOR.handle_error(&error),
        },
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    std::process::exit(1);
}

pub fn abort_message(message: String) -> ! {
    let output = Output::Failure {
        error: response::Error { message },
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    std::process::exit(1);
}

pub fn process(input: Option<String>, config: &Config, manifest: &Manifest) -> Result<Output, String> {
    let input = parse_input(input)?;
    log::debug!("API input: {input:?}");
    let mut responses = vec![];

    let backup_path = input.config.backup_path.unwrap_or_else(|| config.restore.path.clone());
    let layout = BackupLayout::new(backup_path, config.backup.retention.clone());

    let title_finder = TitleFinder::new(config, manifest, layout.restorable_game_set());

    for request in input.requests {
        match request {
            Request::FindTitle(request::FindTitle {
                backup,
                restore,
                steam_id,
                gog_id,
                lutris_id,
                normalized,
                disabled,
                partial,
                names,
            }) => {
                let titles = title_finder.find(TitleQuery {
                    names,
                    steam_id,
                    gog_id,
                    lutris_id,
                    normalized,
                    backup,
                    restore,
                    disabled,
                    partial,
                });

                responses.push(Response::FindTitle(response::FindTitle { titles }));
            }
            Request::CheckAppUpdate(request::CheckAppUpdate {}) => match crate::metadata::Release::fetch_sync() {
                Ok(release) => {
                    let update = release.is_update().then(|| response::AppUpdate {
                        version: release.version.to_string(),
                        url: release.url,
                    });

                    responses.push(Response::CheckAppUpdate(response::CheckAppUpdate { update }));
                }
                Err(e) => {
                    responses.push(Response::Error(response::Error { message: e.to_string() }));
                }
            },
        }
    }

    Ok(Output::Success { responses })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn deserialize_input() {
        let serialized = r#"
        {
          "config": {
            "backupPath": "/tmp"
          },
          "requests": [
            {
              "findTitle": {
                "steamId": 10
              }
            }
          ]
        }
                "#
        .trim();
        let deserialized = serde_json::from_str::<Input>(serialized).unwrap();

        let expected = Input {
            config: ConfigOverride {
                backup_path: Some(StrictPath::new("/tmp".to_string())),
            },
            requests: vec![Request::FindTitle(request::FindTitle {
                steam_id: Some(10),
                ..Default::default()
            })],
        };
        assert_eq!(expected, deserialized);
    }

    #[test]
    pub fn serialize_output() {
        let output = Output::Success {
            responses: vec![Response::FindTitle(response::FindTitle {
                titles: BTreeSet::from(["foo".to_string()]),
            })],
        };
        let serialized = serde_json::to_string_pretty(&output).unwrap();

        let expected = r#"
{
  "responses": [
    {
      "findTitle": {
        "titles": [
          "foo"
        ]
      }
    }
  ]
}
        "#
        .trim();
        assert_eq!(expected, serialized);
    }
}
