use crate::{
    prelude::{run_command, CommandError, StrictPath},
    resource::config::App,
};

#[derive(Debug)]
pub struct RcloneProcess {
    program: String,
    args: Vec<String>,
    child: std::process::Child,
}

impl RcloneProcess {
    pub fn launch(program: String, args: Vec<String>) -> Result<Self, CommandError> {
        let mut command = std::process::Command::new(&program);
        command
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }

        let child = command.spawn().map_err(|e| CommandError::Launched {
            program: program.clone(),
            args: args.clone(),
            raw: e.to_string(),
        })?;

        Ok(Self { program, args, child })
    }

    pub fn progress(&mut self) -> Option<(f32, f32)> {
        use std::io::{BufRead, BufReader};

        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Log {
            stats: Stats,
        }

        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Stats {
            bytes: f32,
            total_bytes: f32,
        }

        if let Some(stderr) = self.child.stderr.as_mut() {
            for line in BufReader::new(stderr).lines().filter_map(|x| x.ok()) {
                if let Ok(parsed) = serde_json::from_str::<Log>(&line) {
                    return Some((parsed.stats.bytes, parsed.stats.total_bytes));
                }
            }
        }

        None
    }

    pub fn succeeded(&mut self) -> Option<Result<(), CommandError>> {
        match self.child.try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => Some(if code == 0 {
                    Ok(())
                } else {
                    use std::io::{BufRead, BufReader};

                    let stdout = self.child.stdout.as_mut().and_then(|x| {
                        let lines = BufReader::new(x).lines().filter_map(|x| x.ok()).collect::<Vec<_>>();
                        (!lines.is_empty()).then_some(lines.join("\n"))
                    });
                    let stderr = self.child.stderr.as_mut().and_then(|x| {
                        let lines = BufReader::new(x).lines().filter_map(|x| x.ok()).collect::<Vec<_>>();
                        (!lines.is_empty()).then_some(lines.join("\n"))
                    });

                    Err(CommandError::Exited {
                        program: self.program.clone(),
                        args: self.args.clone(),
                        code,
                        stdout,
                        stderr,
                    })
                }),
                None => Some(Err(CommandError::Terminated {
                    program: self.program.clone(),
                    args: self.args.clone(),
                })),
            },
            Ok(None) => None,
            Err(_) => Some(Err(CommandError::Terminated {
                program: self.program.clone(),
                args: self.args.clone(),
            })),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "camelCase")]
pub enum RemoteChoice {
    None,
    GoogleDrive,
    Custom,
}

impl RemoteChoice {
    pub const ALL: &[Self] = &[Self::None, Self::GoogleDrive, Self::Custom];
}

impl ToString for RemoteChoice {
    fn to_string(&self) -> String {
        match self {
            Self::None => "None",
            Self::GoogleDrive => "Google Drive",
            Self::Custom => "Custom",
        }
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "camelCase")]
pub enum Remote {
    GoogleDrive,
    Custom { name: String },
}

impl Remote {
    pub fn name(&self) -> &str {
        match self {
            Self::Custom { name } => name,
            _ => "ludusavi",
        }
    }

    pub fn slug(&self) -> &str {
        match self {
            Self::GoogleDrive => "drive",
            Self::Custom { .. } => "",
        }
    }

    pub fn scope(&self) -> &str {
        match self {
            Self::GoogleDrive => "scope=drive.file",
            Self::Custom { .. } => "",
        }
    }

    pub fn needs_configuration(&self) -> bool {
        match self {
            Self::GoogleDrive => true,
            Self::Custom { .. } => false,
        }
    }
}

impl From<Option<&Remote>> for RemoteChoice {
    fn from(value: Option<&Remote>) -> Self {
        if let Some(value) = value {
            match value {
                Remote::GoogleDrive => RemoteChoice::GoogleDrive,
                Remote::Custom { .. } => RemoteChoice::Custom,
            }
        } else {
            RemoteChoice::None
        }
    }
}

impl TryFrom<RemoteChoice> for Remote {
    type Error = ();

    fn try_from(value: RemoteChoice) -> Result<Self, Self::Error> {
        match value {
            RemoteChoice::None => Err(()),
            RemoteChoice::GoogleDrive => Ok(Remote::GoogleDrive),
            RemoteChoice::Custom => Ok(Remote::Custom {
                name: "ludusavi".to_string(),
            }),
        }
    }
}

pub struct Rclone {
    app: App,
    remote: Remote,
}

impl Rclone {
    pub fn new(app: App, remote: Remote) -> Self {
        Self { app, remote }
    }

    fn path(&self, path: &str) -> String {
        format!("{}:{}", self.remote.name(), path)
    }

    fn args(&self, args: &[&str]) -> Vec<String> {
        let mut collected = vec![];
        if !self.app.arguments.is_empty() {
            if let Some(parts) = shlex::split(&self.app.arguments) {
                collected.extend(parts);
            }
        }
        for arg in args {
            collected.push(arg.to_string());
        }
        collected
    }

    fn run(&self, args: &[&str], success: &[i32]) -> Result<i32, CommandError> {
        let args = self.args(args);
        let args: Vec<_> = args.iter().map(|x| x.as_str()).collect();
        let code = run_command(&self.app.path.raw(), &args, success)?;
        Ok(code)
    }

    pub fn configure_remote(&self) -> Result<(), CommandError> {
        self.run(
            &[
                "config",
                "create",
                self.remote.name(),
                self.remote.slug(),
                self.remote.scope(),
            ],
            &[0],
        )?;
        Ok(())
    }

    // pub fn exists(&self, remote_path: &str) -> Result<bool, CommandError> {
    //     let code = self.run(&["lsjson", "--stat", "--no-mimetype", "--no-modtime", &self.path(remote_path)], &[0, 3])?;
    //     Ok(code == 0)
    // }

    // pub fn is_synced(&self, local: &StrictPath, remote_path: &str) -> Result<bool, CommandError> {
    //     let code = self.run(&["check", &local.interpret(), &self.path(remote_path)], &[0, 1])?;
    //     Ok(code == 0)
    // }

    pub fn sync_from_local_to_remote(
        &self,
        local: &StrictPath,
        remote_path: &str,
    ) -> Result<RcloneProcess, CommandError> {
        RcloneProcess::launch(
            self.app.path.raw(),
            self.args(&[
                "sync",
                "-v",
                "--use-json-log",
                "--stats=1s",
                &local.render(),
                &self.path(remote_path),
            ]),
        )
    }

    pub fn sync_from_remote_to_local(
        &self,
        local: &StrictPath,
        remote_path: &str,
    ) -> Result<RcloneProcess, CommandError> {
        RcloneProcess::launch(
            self.app.path.raw(),
            self.args(&[
                "sync",
                "-v",
                "--use-json-log",
                "--stats=1s",
                &self.path(remote_path),
                &local.render(),
            ]),
        )
    }
}

pub mod rclone_monitor {
    use iced_native::{
        futures::{channel::mpsc, StreamExt},
        subscription::{self, Subscription},
    };

    use crate::{cloud::RcloneProcess, prelude::CommandError};

    #[derive(Debug, Clone)]
    pub enum Event {
        Ready(mpsc::Sender<Input>),
        Tick,
        Progress { current: f32, max: f32 },
        Succeeded,
        Failed(CommandError),
        Cancelled,
    }

    #[derive(Debug)]
    pub enum Input {
        Process(RcloneProcess),
        Tick,
        Cancel,
    }

    enum State {
        Starting,
        Ready {
            receiver: mpsc::Receiver<Input>,
            process: Option<RcloneProcess>,
        },
    }

    pub fn run() -> Subscription<Event> {
        struct Runner;

        subscription::unfold(std::any::TypeId::of::<Runner>(), State::Starting, |state| async move {
            match state {
                State::Starting => {
                    let (sender, receiver) = mpsc::channel(100);

                    (
                        Some(Event::Ready(sender)),
                        State::Ready {
                            receiver,
                            process: None,
                        },
                    )
                }
                State::Ready {
                    mut receiver,
                    mut process,
                } => {
                    let input = receiver.select_next_some().await;

                    match input {
                        Input::Process(new_process) => {
                            process = Some(new_process);
                            (Some(Event::Tick), State::Ready { receiver, process })
                        }
                        Input::Tick => {
                            if let Some(proc) = process.as_mut() {
                                if let Some(outcome) = proc.succeeded() {
                                    match outcome {
                                        Ok(_) => {
                                            return (
                                                Some(Event::Succeeded),
                                                State::Ready {
                                                    receiver,
                                                    process: None,
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            return (
                                                Some(Event::Failed(e)),
                                                State::Ready {
                                                    receiver,
                                                    process: None,
                                                },
                                            );
                                        }
                                    }
                                }
                                if let Some((current, max)) = proc.progress() {
                                    return (
                                        Some(Event::Progress { current, max }),
                                        State::Ready { receiver, process },
                                    );
                                }
                            }
                            (Some(Event::Tick), State::Ready { receiver, process })
                        }
                        Input::Cancel => {
                            if let Some(proc) = process.as_mut() {
                                let _ = proc.child.kill();
                            }
                            (
                                Some(Event::Cancelled),
                                State::Ready {
                                    receiver,
                                    process: None,
                                },
                            )
                        }
                    }
                }
            }
        })
    }
}
