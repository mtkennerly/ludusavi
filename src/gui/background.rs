use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::{File, TryLockError},
    net::{Ipv4Addr, UdpSocket},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem},
};

use crate::{
    lang::TRANSLATOR,
    prelude::{StrictPath, app_dir},
    resource::ResourceFile,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    pub poll_seconds: u64,
    pub delay_seconds: u64,
    pub games: Vec<Game>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_seconds: 10,
            delay_seconds: 5,
            games: Vec::new(),
        }
    }
}

impl ResourceFile for Config {
    const FILE_NAME: &'static str = "background.yaml";
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Game {
    pub name: String,
    pub executables: Vec<StrictPath>,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

impl Config {
    pub fn persist(&self) -> Result<(), String> {
        self.validate()?;
        Self::path().create_parent_dir().map_err(|e| e.to_string())?;
        Self::path()
            .write_with_content(&serde_yaml::to_string(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    pub fn add_executable(&mut self, name: String, executable: StrictPath) -> Result<(), String> {
        let executable = StrictPath::from(resolve_executable(PathBuf::from(executable.raw()))?);
        if !Path::new(executable.raw()).is_absolute() || !executable.is_file() {
            return Err(TRANSLATOR.automatic_backup_text("automatic-backup-choose-executable"));
        }
        if let Some(game) = self.games.iter_mut().find(|game| game.name == name) {
            if !game.executables.contains(&executable) {
                game.executables.push(executable);
            }
        } else {
            self.games.push(Game {
                name,
                executables: vec![executable],
                enabled: true,
            });
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=60).contains(&self.poll_seconds) || self.delay_seconds > 3600 {
            return Err(TRANSLATOR.automatic_backup_text("automatic-backup-invalid-interval"));
        }
        let mut names = HashSet::new();
        for game in &self.games {
            if game.name.trim().is_empty() || !names.insert(&game.name) || game.executables.is_empty() {
                return Err(TRANSLATOR.automatic_backup_text("automatic-backup-invalid-games"));
            }
            for executable in &game.executables {
                if !Path::new(executable.raw()).is_absolute() {
                    return Err(TRANSLATOR.automatic_backup_game("automatic-backup-absolute-path", &game.name));
                }
            }
        }
        Ok(())
    }
}

fn resolve_executable(path: PathBuf) -> Result<PathBuf, String> {
    Ok(path)
}

#[derive(Default)]
pub struct ProcessTracker {
    system: sysinfo::System,
    games: Vec<Game>,
    paths: HashMap<String, Vec<String>>,
    filenames: HashSet<String>,
    tracked: HashMap<sysinfo::Pid, GameProcess>,
    last_discovery: Option<Instant>,
}

impl ProcessTracker {
    pub fn running_games(&mut self, config: &Config, now: Instant, confirm: bool) -> Result<HashSet<String>, String> {
        if self.games != config.games {
            self.games = config.games.clone();
            self.paths.clear();
            for game in self.games.iter().filter(|game| game.enabled) {
                for path in &game.executables {
                    if let Ok(path) = path
                        .as_std_path_buf()
                        .map_err(|why| why.to_string())
                        .and_then(resolve_executable)
                    {
                        self.paths
                            .entry(executable_key(&path))
                            .or_default()
                            .push(game.name.clone());
                    }
                }
            }
            self.filenames = self
                .paths
                .keys()
                .filter_map(|path| path.rsplit('/').next().map(str::to_owned))
                .collect();
            self.last_discovery = None;
        }
        if self.paths.is_empty() {
            self.tracked.clear();
            return Ok(HashSet::new());
        }
        let mut discover = confirm
            || self
                .last_discovery
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(config.poll_seconds));
        let refresh = sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always);
        if !discover && !self.tracked.is_empty() {
            let mut check = Vec::new();
            for (pid, process) in &self.tracked {
                if let Some(exit) = &process.exit {
                    match exit.running() {
                        Some(true) => continue,
                        Some(false) => {
                            discover = true;
                            continue;
                        }
                        None => {}
                    }
                }
                check.push(*pid);
            }
            if !discover && !check.is_empty() {
                self.system
                    .refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&check), true, refresh);
                discover = check.iter().any(|pid| {
                    let previous = &self.tracked[pid];
                    self.system.process(*pid).is_none_or(|process| {
                        process.start_time() != previous.started
                            || process.exe().is_none_or(|path| process_path_key(path) != previous.host)
                    })
                });
            }
        }
        if discover {
            self.system
                .refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, refresh);
            if self.system.processes().is_empty() {
                return Err(TRANSLATOR.automatic_backup_text("automatic-backup-process-error"));
            }
            self.last_discovery = Some(now);

            let mut tracked = HashMap::new();
            for (pid, process) in self.system.processes() {
                let Some(host) = process.exe().map(process_path_key) else {
                    continue;
                };
                let path = if self.paths.contains_key(&host) {
                    Some(host.clone())
                } else if self.filenames.contains(host.rsplit('/').next().unwrap_or_default()) {
                    process
                        .exe()
                        .map(executable_key)
                        .filter(|key| self.paths.contains_key(key))
                } else {
                    None
                };

                let Some(path) = path else {
                    continue;
                };
                let previous = self.tracked.remove(pid).filter(|previous| {
                    previous.started == process.start_time()
                        && previous.path == path
                        && previous.host == host
                        && previous.running()
                });
                let is_new = previous.is_none();
                let games = self.paths.get(&path).cloned().unwrap_or_default();
                tracked.insert(
                    *pid,
                    previous.unwrap_or_else(|| GameProcess::new(*pid, path, host, process.start_time())),
                );
                if is_new {
                    for name in games {
                        log::info!("Game process detected: {name} (pid {})", pid.as_u32());
                    }
                }
            }
            self.tracked = tracked;
        }
        Ok(self
            .tracked
            .values()
            .filter_map(|process| self.paths.get(&process.path))
            .flatten()
            .cloned()
            .collect())
    }
}

struct GameProcess {
    path: String,
    host: String,
    started: u64,
    exit: Option<ProcessExit>,
}

impl GameProcess {
    fn new(pid: sysinfo::Pid, path: String, host: String, started: u64) -> Self {
        Self {
            path,
            host,
            started,
            exit: ProcessExit::open(pid),
        }
    }

    fn running(&self) -> bool {
        if let Some(exit) = &self.exit {
            return exit.running().unwrap_or(true);
        }
        true
    }
}

struct ProcessExit(windows::Win32::Foundation::HANDLE);

// This owned process handle is used only for waiting and closing, both valid across threads.
unsafe impl Send for ProcessExit {}

impl ProcessExit {
    fn open(pid: sysinfo::Pid) -> Option<Self> {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};
        unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid.as_u32()).ok().map(Self) }
    }

    fn running(&self) -> Option<bool> {
        use windows::Win32::{
            Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::WaitForSingleObject,
        };
        match unsafe { WaitForSingleObject(self.0, 0) } {
            WAIT_TIMEOUT => Some(true),
            WAIT_OBJECT_0 => Some(false),
            _ => None,
        }
    }
}

impl Drop for ProcessExit {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.0) };
    }
}

fn process_path_key(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    path.trim_start_matches("//?/").to_lowercase()
}

fn executable_key(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    process_path_key(&path)
}

#[derive(Default)]
pub struct Monitor {
    running: HashSet<String>,
    stopped: HashMap<String, Instant>,
    ready: BTreeSet<String>,
}

impl Monitor {
    pub fn confirm_discovery(&self, idle: bool) -> bool {
        // Catch restarts during the delay, and recheck queued backups before starting them.
        !self.stopped.is_empty() || (idle && !self.ready.is_empty())
    }
    pub fn reconfigure(&mut self, old: &Config, new: &Config) {
        let retained: HashSet<_> = new
            .games
            .iter()
            .filter(|game| {
                game.enabled
                    && old
                        .games
                        .iter()
                        .any(|previous| previous.name == game.name && previous.executables == game.executables)
            })
            .map(|game| game.name.as_str())
            .collect();
        self.running.retain(|name| retained.contains(name.as_str()));
        self.stopped.retain(|name, _| retained.contains(name.as_str()));
        self.ready.retain(|name| retained.contains(name.as_str()));
    }

    #[cfg(test)]
    fn observe(&mut self, config: &Config, paths: &[PathBuf], now: Instant) {
        let paths: HashSet<_> = paths.iter().map(|p| executable_key(p)).collect();
        let running = config
            .games
            .iter()
            .filter(|game| {
                game.executables
                    .iter()
                    .filter_map(|p| p.as_std_path_buf().ok())
                    .any(|p| paths.contains(&executable_key(&p)))
            })
            .map(|game| game.name.clone())
            .collect();
        self.observe_games(config, &running, now);
    }

    pub fn observe_games(&mut self, config: &Config, running_games: &HashSet<String>, now: Instant) {
        for game in config.games.iter().filter(|g| g.enabled) {
            let running = running_games.contains(&game.name);
            if running {
                self.running.insert(game.name.clone());
                self.stopped.remove(&game.name);
                self.ready.remove(&game.name);
            } else if self.running.remove(&game.name) {
                self.stopped.insert(game.name.clone(), now);
            }
        }
        self.stopped.retain(|name, since| {
            if now.duration_since(*since) >= Duration::from_secs(config.delay_seconds) {
                self.ready.insert(name.clone());
                false
            } else {
                true
            }
        });
    }

    pub fn take_ready(&mut self) -> Option<String> {
        self.ready.pop_first()
    }
}

pub enum Action {
    Show,
    OpenCustomGame(String),
    Pause(bool),
    Configure,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BackupResult {
    Saved,
    Unchanged,
    Skipped,
    Failed(String),
}

pub struct BackupRun {
    pub game: String,
    started: Instant,
    pub result: BackupResult,
}

#[derive(Clone, Debug)]
pub struct BackupNotice {
    pub title: String,
    pub body: String,
    pub failed: bool,
}

impl BackupRun {
    pub fn new(game: String) -> Self {
        Self {
            game,
            started: Instant::now(),
            result: BackupResult::Skipped,
        }
    }

    pub fn scanned(&mut self, scan: &crate::scan::ScanInfo, backup: Option<&crate::scan::BackupInfo>) {
        if scan.game_name != self.game {
            return;
        }
        self.result = if let Some(backup) = backup
            && !backup.successful()
        {
            let reason = backup
                .failed_files
                .values()
                .chain(backup.failed_registry.values())
                .next()
                .map(|error| error.message())
                .unwrap_or_else(|| TRANSLATOR.automatic_backup_text("automatic-backup-failed-detail"));
            BackupResult::Failed(reason)
        } else if !scan.can_report_game() || scan.all_ignored() {
            BackupResult::Skipped
        } else if backup.is_some() {
            BackupResult::Saved
        } else if !scan.found_anything_processable() {
            BackupResult::Unchanged
        } else {
            BackupResult::Skipped
        };
    }

    pub fn finished(self, errors: &[crate::prelude::Error], cancelled: bool) -> BackupNotice {
        use crate::{lang::TRANSLATOR, prelude::Error};
        let error = errors.iter().find(|error| **error != Error::SomeEntriesFailed);
        let (title, message, failed) = if cancelled {
            (
                TRANSLATOR.automatic_backup_text("automatic-backup-cancelled"),
                TRANSLATOR.automatic_backup_text("automatic-backup-cancelled-detail"),
                true,
            )
        } else if let BackupResult::Failed(reason) = self.result {
            (
                TRANSLATOR.automatic_backup_text("automatic-backup-failed"),
                reason,
                true,
            )
        } else if let Some(error) = error {
            let title = match (&self.result, error) {
                (
                    BackupResult::Saved | BackupResult::Unchanged,
                    Error::CloudConflict | Error::UnableToSynchronizeCloud(_),
                ) => TRANSLATOR.automatic_backup_text("automatic-backup-cloud-failed"),
                _ => TRANSLATOR.automatic_backup_text("automatic-backup-failed"),
            };
            (title, TRANSLATOR.handle_error(error), true)
        } else {
            match self.result {
                BackupResult::Saved => (
                    TRANSLATOR.automatic_backup_text("automatic-backup-complete"),
                    TRANSLATOR.automatic_backup_text("automatic-backup-complete-detail"),
                    false,
                ),
                BackupResult::Unchanged => (
                    TRANSLATOR.automatic_backup_text("automatic-backup-unchanged"),
                    TRANSLATOR.automatic_backup_text("automatic-backup-unchanged-detail"),
                    false,
                ),
                BackupResult::Skipped => (
                    TRANSLATOR.automatic_backup_text("automatic-backup-skipped"),
                    TRANSLATOR.automatic_backup_text("automatic-backup-skipped-detail"),
                    true,
                ),
                BackupResult::Failed(_) => unreachable!(),
            }
        };
        BackupNotice {
            title,
            body: format!(
                "{}: {message} ({:.1} s)",
                self.game,
                self.started.elapsed().as_secs_f64()
            ),
            failed,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Edit {
    Enable(bool),
    Pause(bool),
    Delay(i32),
    Poll(i32),
    Search(String),
    ChooseGame(String),
    AddExecutable(String, StrictPath),
    GameEnabled(usize, bool),
    Remove(usize),
    RemoveExecutable(usize, usize),
}

#[derive(Default)]
pub struct Settings {
    pub config: Config,
    pub search: String,
}

impl Settings {
    pub fn game_names(log: &super::game_list::GameList) -> Vec<String> {
        let mut names: Vec<_> = log
            .entries
            .iter()
            .map(|entry| entry.scan_info.game_name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn view<'a>(&'a self, log: &super::game_list::GameList, paused: bool) -> super::widget::Element<'a> {
        use super::{
            button,
            common::{BrowseFileSubject, Message},
            style,
            widget::{
                Button, Column, Container, IcedParentExt, Row, TextInput, checkbox, number_input, pick_list, text,
            },
        };
        use iced::{Alignment, Length};
        let edit = Message::BackgroundEdit;
        let filter = self.search.to_lowercase();
        let names: Vec<_> = Self::game_names(log)
            .into_iter()
            .filter(|name| name.to_lowercase().contains(&filter))
            .collect();
        let mut content = Column::new()
            .spacing(10)
            .push(checkbox(
                TRANSLATOR.automatic_backup_text("automatic-backup-enable"),
                self.config.enabled,
                move |v| edit(Edit::Enable(v)),
            ))
            .push(text(TRANSLATOR.automatic_backup_text("automatic-backup-close-help")));
        content = content.push(
            Row::new()
                .spacing(20)
                .align_y(Alignment::Center)
                .push(number_input(
                    self.config.delay_seconds as i32,
                    TRANSLATOR.automatic_backup_text("automatic-backup-delay"),
                    0..=3600,
                    move |v| edit(Edit::Delay(v)),
                ))
                .push(number_input(
                    self.config.poll_seconds as i32,
                    TRANSLATOR.automatic_backup_text("automatic-backup-poll"),
                    1..=60,
                    move |v| edit(Edit::Poll(v)),
                )),
        );
        content = content.push(
            Row::new()
                .spacing(20)
                .align_y(Alignment::Center)
                .push(checkbox(
                    TRANSLATOR.automatic_backup_text("automatic-backup-pause"),
                    paused,
                    move |v| edit(Edit::Pause(v)),
                ))
                .push(
                    Button::new(text(TRANSLATOR.automatic_backup_text("automatic-backup-quit")))
                        .padding(5)
                        .class(style::Button::Negative)
                        .on_press(Message::Exit { user: true }),
                ),
        );
        for (i, game) in self.config.games.iter().enumerate() {
            let mut paths = Column::new().spacing(5);
            for (j, path) in game.executables.iter().enumerate() {
                paths = paths.push(
                    Row::new()
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .push(text(path.render()).width(Length::Fill))
                        .push(
                            Button::new(text(
                                TRANSLATOR.automatic_backup_text("automatic-backup-remove-executable"),
                            ))
                            .padding(5)
                            .class(style::Button::Negative)
                            .on_press(edit(Edit::RemoveExecutable(i, j))),
                        ),
                );
            }
            content = content.push(
                Container::new(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .spacing(20)
                                .align_y(Alignment::Center)
                                .push(text(game.name.clone()).width(Length::Fill))
                                .push(
                                    Button::new(text(TRANSLATOR.automatic_backup_text("automatic-backup-remove-game")))
                                        .padding(5)
                                        .class(style::Button::Negative)
                                        .on_press(edit(Edit::Remove(i))),
                                ),
                        )
                        .push(checkbox(
                            TRANSLATOR.automatic_backup_text("automatic-backup-enable-game"),
                            game.enabled,
                            move |v| edit(Edit::GameEnabled(i, v)),
                        ))
                        .push(paths)
                        .push(
                            Row::new()
                                .spacing(10)
                                .align_y(Alignment::Center)
                                .push(text(
                                    TRANSLATOR.automatic_backup_text("automatic-backup-add-executable"),
                                ))
                                .push(button::choose_file(
                                    BrowseFileSubject::BackgroundExecutable(game.name.clone()),
                                    &iced::keyboard::Modifiers::default(),
                                )),
                        ),
                )
                .padding(5)
                .class(style::Container::GameListEntry),
            );
        }
        content = content
            .push(text(TRANSLATOR.automatic_backup_text("automatic-backup-choose-help")))
            .push_if(log.entries.is_empty(), || {
                text(TRANSLATOR.automatic_backup_text("automatic-backup-preview-help"))
            })
            .push(
                Row::new()
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(
                        TextInput::new(
                            &TRANSLATOR.automatic_backup_text("automatic-backup-search"),
                            &self.search,
                        )
                        .on_input(move |v| edit(Edit::Search(v)))
                        .padding(5),
                    )
                    .push(
                        pick_list(names, None::<String>, move |v| edit(Edit::ChooseGame(v)))
                            .placeholder(TRANSLATOR.automatic_backup_text("automatic-backup-choose-game"))
                            .class(style::PickList::Primary),
                    ),
            );
        Column::new()
            .spacing(5)
            .push(text(TRANSLATOR.automatic_backup_text("automatic-backup-title")))
            .push(
                Container::new(content)
                    .padding(5)
                    .class(style::Container::GameListEntry),
            )
            .into()
    }
}

pub struct Instance {
    _lock: File,
    socket: UdpSocket,
}

impl Instance {
    pub fn acquire(path: &Path, custom_game: Option<&str>) -> Result<Option<Self>, String> {
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        match lock.try_lock() {
            Ok(()) => {
                let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
                socket.set_nonblocking(true).map_err(|e| e.to_string())?;
                // Windows prevents other processes from reading a locked file.
                std::fs::write(
                    path.with_extension("port"),
                    format!(
                        "{}\n{}",
                        socket.local_addr().map_err(|e| e.to_string())?.port(),
                        std::process::id()
                    ),
                )
                .map_err(|e| e.to_string())?;
                Ok(Some(Self { _lock: lock, socket }))
            }
            Err(TryLockError::WouldBlock) => {
                Self::open_existing(path, custom_game)?;
                Ok(None)
            }
            Err(TryLockError::Error(why)) => {
                Err(TRANSLATOR.automatic_backup_error("automatic-backup-start-error", None, &why.to_string()))
            }
        }
    }

    fn open_existing(path: &Path, custom_game: Option<&str>) -> Result<(), String> {
        Self::request_open(path, custom_game, Duration::from_secs(5)).map_err(|why| {
            log::warn!("Unable to reopen running Ludusavi: {why}");
            TRANSLATOR.automatic_backup_text("automatic-backup-already-running")
        })
    }

    fn request_open(
        path: &Path,
        custom_game: Option<&str>,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
        let message = format!("ludusavi:open:{}", serde_json::to_string(&custom_game)?);
        let deadline = Instant::now() + timeout;
        loop {
            let attempt = || -> Result<(), Box<dyn std::error::Error>> {
                let address = std::fs::read_to_string(path.with_extension("port"))?;
                let mut address = address.lines();
                let port: u16 = address.next().ok_or("Missing instance port")?.parse()?;
                if let Some(pid) = address.next().and_then(|pid| pid.parse().ok()) {
                    // A shortcut launch can transfer its foreground permission to the owner.
                    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(pid) };
                }
                socket.connect((Ipv4Addr::LOCALHOST, port))?;
                socket.set_read_timeout(Some(
                    (deadline.saturating_duration_since(Instant::now()))
                        .min(Duration::from_secs(1))
                        .max(Duration::from_millis(1)),
                ))?;
                socket.send(message.as_bytes())?;
                let mut reply = [0; 16];
                let size = socket.recv(&mut reply)?;
                if &reply[..size] != b"ludusavi:opened" {
                    return Err("Unexpected reply from the running app".into());
                }
                Ok(())
            };
            match attempt() {
                Ok(()) => return Ok(()),
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }

    fn actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut message = [0; 8192];
        for _ in 0..8 {
            let Ok((size, sender)) = self.socket.recv_from(&mut message) else {
                break;
            };
            let Some(message) = message[..size].strip_prefix(b"ludusavi:open:") else {
                continue;
            };
            let Ok(game) = serde_json::from_slice::<Option<String>>(message) else {
                continue;
            };
            if !sender.ip().is_loopback() {
                continue;
            }
            actions.push(match game {
                Some(game) => Action::OpenCustomGame(game),
                None => Action::Show,
            });
            let _ = self.socket.send_to(b"ludusavi:opened", sender);
            log::info!("Reopening Ludusavi after another launch");
        }
        actions
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = self._lock.unlock();
    }
}

pub struct Background {
    pub config: Config,
    pub monitor: Monitor,
    pub paused: bool,
    pub polling: bool,
    pub processes: Arc<Mutex<ProcessTracker>>,
    instance: Instance,
    _tray: Option<TrayIcon>,
    notification_guid: u128,
    show: MenuItem,
    pause: CheckMenuItem,
    configure: MenuItem,
    quit: MenuItem,
}

impl Background {
    pub fn new(config: Config, instance: Option<Instance>) -> Result<Self, String> {
        let lock_path = app_dir().joined("background.lock");
        lock_path.create_parent_dir().map_err(|e| e.to_string())?;
        let instance = match instance {
            Some(instance) => instance,
            None => Instance::acquire(&lock_path.as_std_path_buf().map_err(|e| e.to_string())?, None)?
                .ok_or(TRANSLATOR.automatic_backup_text("automatic-backup-other-window"))?,
        };
        let menu = Menu::new();
        let show = MenuItem::new(TRANSLATOR.automatic_backup_text("automatic-backup-show"), true, None);
        let pause = CheckMenuItem::new(
            TRANSLATOR.automatic_backup_text("automatic-backup-pause"),
            true,
            false,
            None,
        );
        let configure = MenuItem::new(
            TRANSLATOR.automatic_backup_text("automatic-backup-settings"),
            true,
            None,
        );
        let quit = MenuItem::new(
            TRANSLATOR.automatic_backup_text("automatic-backup-menu-quit"),
            true,
            None,
        );
        menu.append_items(&[&show, &pause, &configure, &quit])
            .map_err(|e| e.to_string())?;
        let image = image::load_from_memory(include_bytes!("../../assets/icon.png"))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let icon =
            tray_icon::Icon::from_rgba(image.to_vec(), image.width(), image.height()).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("Ludusavi")
            .with_icon(icon)
            .with_menu(Box::new(menu));
        let notification_guid = {
            use sha1::Digest;
            // Different executable paths or config directories need distinct notification icons.
            let mut hash = sha1::Sha1::new();
            hash.update(
                std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .as_bytes(),
            );
            hash.update([0]);
            hash.update(app_dir().resolve().as_bytes());
            u128::from_be_bytes(hash.finalize()[..16].try_into().unwrap())
        };
        let tray = tray.with_guid(notification_guid);

        let tray = Some(tray.build().map_err(|e| e.to_string())?);
        let background = Self {
            config,
            monitor: Monitor::default(),
            paused: false,
            polling: false,
            processes: Arc::new(Mutex::new(ProcessTracker::default())),
            instance,
            _tray: tray,
            notification_guid,
            show,
            pause,
            configure,
            quit,
        };
        Ok(background)
    }

    pub fn notify_backup_start(&self, game: &str) -> Result<(), String> {
        self.notify_backup(&BackupNotice {
            title: TRANSLATOR.automatic_backup_text("automatic-backup-start-title"),
            body: TRANSLATOR.automatic_backup_game("automatic-backup-start-body", game),
            failed: false,
        })
    }

    pub fn notify_backup(&self, notice: &BackupNotice) -> Result<(), String> {
        use windows::Win32::{
            Foundation::HWND,
            UI::Shell::{NIF_GUID, NIF_INFO, NIIF_INFO, NIIF_WARNING, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW},
        };
        let data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: HWND(
                self._tray
                    .as_ref()
                    .ok_or_else(|| TRANSLATOR.automatic_backup_text("automatic-backup-no-tray"))?
                    .window_handle(),
            ),
            uFlags: NIF_GUID | NIF_INFO,
            guidItem: windows::core::GUID::from_u128(self.notification_guid),
            szInfoTitle: notification_text(&notice.title),
            szInfo: notification_text(&notice.body),
            dwInfoFlags: if notice.failed { NIIF_WARNING } else { NIIF_INFO },
            ..Default::default()
        };
        if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
            Ok(())
        } else {
            Err(TRANSLATOR.automatic_backup_text("automatic-backup-notification-error"))
        }
    }

    pub fn has_tray(&self) -> bool {
        self._tray.is_some()
    }

    pub fn actions(&self) -> Vec<Action> {
        let mut actions = self.instance.actions();
        actions.extend(MenuEvent::receiver().try_iter().filter_map(|event| {
            if event.id == *self.show.id() {
                Some(Action::Show)
            } else if event.id == *self.pause.id() {
                Some(Action::Pause(self.pause.is_checked()))
            } else if event.id == *self.configure.id() {
                Some(Action::Configure)
            } else if event.id == *self.quit.id() {
                Some(Action::Quit)
            } else {
                None
            }
        }));
        actions
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.pause.set_checked(paused);
        self.monitor = Monitor::default();
    }
}

fn notification_text<const N: usize>(text: &str) -> [u16; N] {
    let mut buffer = [0; N];
    let mut offset = 0;
    for character in text.chars().take_while(|c| *c != '\0') {
        let mut units = [0; 2];
        let units = character.encode_utf16(&mut units);
        if offset + units.len() >= N {
            break;
        }
        buffer[offset..offset + units.len()].copy_from_slice(units);
        offset += units.len();
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_launch_reopens_the_owner_and_preserves_custom_game_requests() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("background.lock");
        let owner = Instance::acquire(&path, None).unwrap().unwrap();
        for game in [None, Some("日本語 game")] {
            std::thread::scope(|scope| {
                let client = scope.spawn(|| assert!(Instance::acquire(&path, game).unwrap().is_none()));
                let started = Instant::now();
                loop {
                    let actions = owner.actions();
                    if let Some(action) = actions.first() {
                        assert_eq!(actions.len(), 1);
                        assert!(match (game, action) {
                            (None, Action::Show) => true,
                            (Some(expected), Action::OpenCustomGame(actual)) => expected == actual,
                            _ => false,
                        });
                        break;
                    }
                    assert!(started.elapsed() < Duration::from_secs(3));
                    std::thread::sleep(Duration::from_millis(5));
                }
                client.join().unwrap();
            });
        }
    }

    #[test]
    fn concurrent_launch_waits_for_missing_invalid_or_stale_port() {
        for initial in [None, Some("invalid"), Some("1")] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("background.lock");
            let lock = File::options()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            lock.try_lock().unwrap();
            if let Some(initial) = initial {
                std::fs::write(path.with_extension("port"), initial).unwrap();
            }
            let started = std::sync::Barrier::new(2);
            std::thread::scope(|scope| {
                let client = scope.spawn(|| {
                    started.wait();
                    assert!(Instance::request_open(&path, None, Duration::from_secs(5)).is_ok());
                });
                started.wait();
                std::thread::sleep(Duration::from_millis(150));
                let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
                socket.set_nonblocking(true).unwrap();
                std::fs::write(
                    path.with_extension("port"),
                    socket.local_addr().unwrap().port().to_string(),
                )
                .unwrap();
                let owner = Instance { _lock: lock, socket };
                let deadline = Instant::now() + Duration::from_secs(4);
                while owner.actions().is_empty() {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(5));
                }
                client.join().unwrap();
            });
        }
    }

    #[test]
    fn unresponsive_owner_has_a_bounded_handshake() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("background.lock");
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        std::fs::write(
            path.with_extension("port"),
            socket.local_addr().unwrap().port().to_string(),
        )
        .unwrap();
        let start = Instant::now();
        assert!(Instance::request_open(&path, None, Duration::from_millis(150)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn separate_configs_and_stale_files_do_not_block_startup() {
        let root = tempfile::tempdir().unwrap();
        let first_path = root.path().join("first.lock");
        let second_path = root.path().join("second.lock");
        let first = Instance::acquire(&first_path, None).unwrap().unwrap();
        let second = Instance::acquire(&second_path, None).unwrap().unwrap();
        let sender = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        sender
            .send_to(b"unrelated message", first.socket.local_addr().unwrap())
            .unwrap();
        assert!(first.actions().is_empty());
        assert!(second.actions().is_empty());
        drop(first);
        assert!(Instance::acquire(&first_path, None).unwrap().is_some());
    }

    #[test]
    fn dropping_instance_releases_lock_with_inherited_file_handle() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("background.lock");
        let instance = Instance::acquire(&path, None).unwrap().unwrap();
        let inherited = instance._lock.try_clone().unwrap();

        drop(instance);

        let replacement = Instance::acquire(&path, None).unwrap();
        assert!(replacement.is_some());
        drop(replacement);
        drop(inherited);
    }

    #[test]
    fn backup_results_distinguish_saved_unchanged_skipped_and_failed() {
        use crate::scan::{BackupError, BackupInfo, ScanChange, ScanInfo, ScannedFile};
        let file = StrictPath::new("save.dat");
        let mut scan = ScanInfo {
            game_name: "Example".into(),
            found_files: HashMap::from([(
                file.clone(),
                ScannedFile {
                    change: ScanChange::New,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let mut run = BackupRun::new("Example".into());
        run.scanned(&scan, Some(&BackupInfo::default()));
        assert_eq!(run.finished(&[], false).title, "Automatic backup complete");
        scan.has_backups = true;
        scan.found_files.get_mut(&file).unwrap().change = ScanChange::Same;
        let mut run = BackupRun::new("Example".into());
        run.scanned(&scan, None);
        assert_eq!(run.finished(&[], false).title, "No backup needed");
        let failure = BackupInfo {
            failed_files: HashMap::from([(file, BackupError::Raw("Disk full".into()))]),
            ..Default::default()
        };
        let mut run = BackupRun::new("Example".into());
        run.scanned(&scan, Some(&failure));
        let notice = run.finished(&[], false);
        assert_eq!(notice.title, "Automatic backup failed");
        assert!(notice.body.contains("Disk full"));
        assert_eq!(
            BackupRun::new("Example".into()).finished(&[], false).title,
            "Automatic backup skipped"
        );
    }

    #[test]
    fn results_wait_for_cloud_and_never_call_cancellation_a_success() {
        use crate::prelude::Error;
        let mut run = BackupRun::new("Example".into());
        run.result = BackupResult::Saved;
        let notice = run.finished(&[Error::CloudConflict], false);
        assert_eq!(notice.title, "Backup saved; cloud sync failed");
        assert!(notice.failed);
        let mut run = BackupRun::new("Example".into());
        run.result = BackupResult::Saved;
        assert_eq!(run.finished(&[], true).title, "Automatic backup cancelled");
    }

    #[test]
    fn game_choices_use_the_backup_list_including_previous_discoveries() {
        use crate::{
            gui::game_list::GameList,
            resource::{cache::Cache, config::Config},
            scan::ScanKind,
        };
        let mut cache = Cache::default();
        cache.backup.recent_games.insert("Previously discovered".into());
        cache.restore.recent_games.insert("Only in Restore".into());
        let mut log = GameList::with_recent_games(ScanKind::Backup, &Config::default(), &cache);
        assert_eq!(Settings::game_names(&log), vec!["Previously discovered"]);
        log.entries.push(super::super::game_list::GameListEntry {
            scan_info: crate::scan::ScanInfo {
                game_name: "Newly discovered".into(),
                ..Default::default()
            },
            scanned: true,
            ..Default::default()
        });
        assert_eq!(
            Settings::game_names(&log),
            vec!["Newly discovered", "Previously discovered"]
        );
    }

    #[test]
    fn selected_executables_belong_to_their_game_and_invalid_choices_leave_config_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let first = StrictPath::from(temp.path().join("first.exe"));
        let second = StrictPath::from(temp.path().join("second.exe"));
        first.write_with_content("").unwrap();
        second.write_with_content("").unwrap();
        let mut config = Config::default();
        config.add_executable("First game".into(), first.clone()).unwrap();
        config.add_executable("Second game".into(), second.clone()).unwrap();
        config.add_executable("First game".into(), first.clone()).unwrap();
        assert_eq!(config.games.len(), 2);
        assert_eq!(config.games[0].executables, vec![first]);
        assert_eq!(config.games[1].executables, vec![second]);
        config.games[0].enabled = false;
        config
            .add_executable("First game".into(), config.games[1].executables[0].clone())
            .unwrap();
        assert!(!config.games[0].enabled);
        let before = config.clone();
        assert!(
            config
                .add_executable("First game".into(), StrictPath::from(temp.path()))
                .is_err()
        );
        assert_eq!(config, before);
        assert!(
            config
                .add_executable("First game".into(), StrictPath::from(temp.path().join("missing.exe")))
                .is_err()
        );
        assert_eq!(config, before);
    }

    fn fixture() -> (Config, PathBuf) {
        let path = std::env::current_dir().unwrap().join("game.exe");
        (
            Config {
                games: vec![Game {
                    name: "game".into(),
                    executables: vec![StrictPath::new(path.to_string_lossy().into_owned())],
                    enabled: true,
                }],
                ..Default::default()
            },
            path,
        )
    }

    #[test]
    fn only_observed_exits_trigger_one_backup_after_delay() {
        let (config, path) = fixture();
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, &[], start);
        assert!(monitor.take_ready().is_none());
        monitor.observe(&config, &[path.clone(), path], start);
        monitor.observe(&config, &[], start + Duration::from_secs(1));
        assert!(monitor.take_ready().is_none());
        monitor.observe(&config, &[], start + Duration::from_secs(6));
        assert_eq!(monitor.take_ready().as_deref(), Some("game"));
        monitor.observe(&config, &[], start + Duration::from_secs(100));
        assert!(monitor.take_ready().is_none());
    }

    #[test]
    fn restarting_game_cancels_pending_and_queued_backup() {
        let (config, path) = fixture();
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, std::slice::from_ref(&path), start);
        monitor.observe(&config, &[], start);
        monitor.observe(&config, std::slice::from_ref(&path), start + Duration::from_secs(4));
        monitor.observe(&config, &[], start + Duration::from_secs(5));
        monitor.observe(&config, &[], start + Duration::from_secs(10));
        monitor.observe(&config, &[path], start + Duration::from_secs(11));
        assert!(monitor.take_ready().is_none());
    }

    #[test]
    fn unrelated_processes_and_disabled_games_never_trigger_backup() {
        let (mut config, path) = fixture();
        config.games[0].enabled = false;
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, &[path], start);
        monitor.observe(&config, &[], start + Duration::from_secs(100));
        assert!(monitor.take_ready().is_none());
    }

    #[test]
    fn queued_exits_survive_while_backup_is_busy() {
        let (config, path) = fixture();
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, &[path], start);
        for seconds in [1, 10, 20, 30] {
            monitor.observe(&config, &[], start + Duration::from_secs(seconds));
        }
        assert_eq!(monitor.take_ready().as_deref(), Some("game"));
    }

    #[test]
    fn settings_reload_keeps_unaffected_pending_exits() {
        let (config, path) = fixture();
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, &[path], start);
        monitor.observe(&config, &[], start);
        let mut changed = config.clone();
        changed.poll_seconds = 3;
        monitor.reconfigure(&config, &changed);
        monitor.observe(&changed, &[], start + Duration::from_secs(10));
        assert_eq!(monitor.take_ready().as_deref(), Some("game"));
    }

    #[test]
    fn disabling_a_game_cancels_its_pending_and_queued_backups() {
        let (config, path) = fixture();
        let start = Instant::now();
        for elapsed in [1, 10] {
            let mut monitor = Monitor::default();
            monitor.observe(&config, std::slice::from_ref(&path), start);
            monitor.observe(&config, &[], start);
            monitor.observe(&config, &[], start + Duration::from_secs(elapsed));
            let mut disabled = config.clone();
            disabled.games[0].enabled = false;
            monitor.reconfigure(&config, &disabled);
            monitor.observe(&disabled, &[], start + Duration::from_secs(20));
            assert!(monitor.take_ready().is_none());
        }
    }

    #[test]
    fn notification_text_is_terminated_without_splitting_unicode_characters() {
        assert_eq!(
            notification_text::<8>("ゲーム"),
            [0x30b2, 0x30fc, 0x30e0, 0, 0, 0, 0, 0]
        );
        assert_eq!(notification_text::<4>("ab🎮"), [b'a' as u16, b'b' as u16, 0, 0]);
        assert_eq!(
            notification_text::<5>("ab🎮"),
            [b'a' as u16, b'b' as u16, 0xd83c, 0xdfae, 0]
        );
        assert_eq!(
            notification_text::<4>("abcde"),
            [b'a' as u16, b'b' as u16, b'c' as u16, 0]
        );
        assert_eq!(notification_text::<4>("a\0b"), [b'a' as u16, 0, 0, 0]);
    }

    #[test]
    fn malformed_configuration_is_rejected() {
        let (mut config, _) = fixture();
        config.poll_seconds = 0;
        assert!(config.validate().is_err());
        assert!(Config::load_from_string("games: []\npollSecond: 2").is_err());
        config.poll_seconds = 2;
        config.games[0].executables = vec![StrictPath::new("relative.exe")];
        assert!(config.validate().is_err());
    }

    #[test]
    fn waits_for_last_process_and_supports_multiple_game_executables() {
        let (mut config, first) = fixture();
        let second = first.with_file_name("second.exe");
        config.games[0]
            .executables
            .push(StrictPath::new(second.to_string_lossy().into_owned()));
        let start = Instant::now();
        let mut monitor = Monitor::default();
        monitor.observe(&config, &[first.clone(), first, second.clone()], start);
        monitor.observe(&config, &[second], start + Duration::from_secs(10));
        assert!(monitor.take_ready().is_none());
        monitor.observe(&config, &[], start + Duration::from_secs(11));
        monitor.observe(&config, &[], start + Duration::from_secs(16));
        assert_eq!(monitor.take_ready().as_deref(), Some("game"));
    }

    #[test]
    fn system_process_snapshot_includes_this_executable() {
        let config = Config {
            games: vec![Game {
                name: "current process".into(),
                executables: vec![StrictPath::from(std::env::current_exe().unwrap())],
                enabled: true,
            }],
            ..Config::default()
        };
        let mut tracker = ProcessTracker::default();
        let start = Instant::now();
        assert!(
            tracker
                .running_games(&config, start, false)
                .unwrap()
                .contains("current process")
        );
        assert!(
            tracker
                .running_games(&config, start + Duration::from_secs(1), false)
                .unwrap()
                .contains("current process")
        );
        assert!(tracker.tracked.contains_key(&sysinfo::get_current_pid().unwrap()));
    }

    #[test]
    fn no_process_scan_without_enabled_games() {
        let mut tracker = ProcessTracker::default();
        let config = Config::default();
        assert!(
            tracker
                .running_games(&config, Instant::now(), false)
                .unwrap()
                .is_empty()
        );
        let config = Config {
            games: vec![Game {
                name: "disabled".into(),
                enabled: false,
                executables: vec![StrictPath::from(std::env::current_exe().unwrap())],
            }],
            ..config
        };
        assert!(
            tracker
                .running_games(&config, Instant::now(), false)
                .unwrap()
                .is_empty()
        );
        assert!(tracker.system.processes().is_empty());
        assert!(tracker.last_discovery.is_none());
    }
}
