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
    /// Override the configured backup directory.
    pub backup_dir: Option<StrictPath>,
    /// The order of the requests here will match the order of responses in the output.
    pub requests: Vec<Request>,
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
}

/// A response to an individual request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    Error(response::Error),
    FindTitle(response::FindTitle),
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
            let mut raw = String::new();
            let _ = stdin.read_to_string(&mut raw);
            let input = serde_json::from_str::<Input>(&raw).map_err(|e| e.to_string())?;
            log::debug!("API input from stdin: {:?}", &raw);
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
    eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
    std::process::exit(1);
}

pub fn abort_message(message: String) -> ! {
    let output = Output::Failure {
        error: response::Error { message },
    };
    eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
    std::process::exit(1);
}

pub fn process(input: Option<String>, config: &Config, manifest: &Manifest) -> Result<Output, String> {
    let input = parse_input(input)?;
    let mut responses = vec![];

    let restore_dir = input.backup_dir.unwrap_or_else(|| config.restore.path.clone());
    let layout = BackupLayout::new(restore_dir, config.backup.retention.clone());

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
          "backupDir": "/tmp",
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
        let deserialized = serde_json::from_str::<Input>(&serialized).unwrap();

        let expected = Input {
            backup_dir: Some(StrictPath::new("/tmp".to_string())),
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
