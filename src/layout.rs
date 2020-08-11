use crate::{path::StrictPath, prelude::ScannedFile};

const SAFE: &str = "_";

fn encode_base64_for_folder(name: &str) -> String {
    base64::encode(&name).replace("/", SAFE)
}

fn escape_folder_name(name: &str) -> String {
    let mut escaped = String::from(name);

    // Technically, dots should be fine as long as the folder name isn't
    // exactly `.` or `..`. However, leading dots will often cause items
    // to be hidden by default, which could be confusing for users, so we
    // escape those. And Windows Explorer has a fun bug where, if you try
    // to open a folder whose name ends with a dot, then it will say that
    // the folder no longer exists at that location, so we also escape dots
    // at the end of the name. The combination of these two rules also
    // happens to cover the `.` and `..` cases.
    if escaped.starts_with('.') {
        escaped.replace_range(..1, SAFE);
    }
    if escaped.ends_with('.') {
        escaped.replace_range(escaped.len() - 1.., SAFE);
    }

    escaped
        .replace("\\", SAFE)
        .replace("/", SAFE)
        .replace(":", SAFE)
        .replace("*", SAFE)
        .replace("?", SAFE)
        .replace("\"", SAFE)
        .replace("<", SAFE)
        .replace(">", SAFE)
        .replace("|", SAFE)
        .replace("\0", SAFE)
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IndividualMapping {
    pub name: String,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    pub drives: std::collections::HashMap<String, String>,
}

impl IndividualMapping {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    fn reversed_drives(&self) -> std::collections::HashMap<String, String> {
        self.drives.iter().map(|(k, v)| (v.to_owned(), k.to_owned())).collect()
    }

    pub fn drive_folder_name(&mut self, drive: &str) -> String {
        let reversed = self.reversed_drives();
        match reversed.get::<str>(&drive) {
            Some(mapped) => mapped.to_string(),
            None => {
                let key = if drive.is_empty() {
                    "drive-0".to_string()
                } else {
                    // Simplify "C:" to "drive-C" instead of "drive-C_" for the common case.
                    format!("drive-{}", escape_folder_name(&drive.replace(":", "")))
                };
                self.drives.insert(key.to_string(), drive.to_string());
                key
            }
        }
    }

    pub fn save(&self, file: &StrictPath) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old) = Self::load(&file) {
            let old_content = serde_yaml::to_string(&old).unwrap();
            if old_content == new_content {
                return;
            }
        }

        if file.create_parent_dir().is_ok() {
            std::fs::write(file.interpret(), self.serialize().as_bytes()).unwrap();
        }
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }

    pub fn load(file: &StrictPath) -> Result<Self, ()> {
        if !file.is_file() {
            return Err(());
        }
        let content = std::fs::read_to_string(&file.interpret()).unwrap();
        Self::load_from_string(&content)
    }

    pub fn load_from_string(content: &str) -> Result<Self, ()> {
        match serde_yaml::from_str(&content) {
            Ok(x) => Ok(x),
            Err(_) => Err(()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct OverallMapping {
    pub games: std::collections::HashMap<String, OverallMappingGame>,
}

#[derive(Clone, Debug, Default)]
pub struct OverallMappingGame {
    pub drives: std::collections::HashMap<String, String>,
    pub base: StrictPath,
}

impl OverallMapping {
    pub fn load(base: &StrictPath) -> Self {
        let mut overall = Self::default();

        for game_dir in walkdir::WalkDir::new(base.interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .skip(1) // the base path itself
            .filter_map(|e| e.ok())
            .filter(|x| x.file_type().is_dir())
        {
            let individual_file = &mut game_dir.path().to_path_buf();
            individual_file.push("mapping.yaml");
            if individual_file.is_file() {
                let game = match IndividualMapping::load(&StrictPath::from_std_path_buf(&individual_file)) {
                    Ok(x) => x,
                    Err(_) => continue,
                };
                overall.games.insert(
                    game.name,
                    OverallMappingGame {
                        base: StrictPath::from_std_path_buf(&game_dir.path().to_path_buf()),
                        drives: game.drives,
                    },
                );
            }
        }

        overall
    }
}

#[derive(Clone, Debug, Default)]
pub struct BackupLayout {
    pub base: StrictPath,
    pub mapping: OverallMapping,
}

impl BackupLayout {
    pub fn new(base: StrictPath) -> Self {
        let mapping = OverallMapping::load(&base);
        Self { base, mapping }
    }

    fn generate_total_rename(original_name: &str) -> String {
        format!("ludusavi-renamed-{}", encode_base64_for_folder(&original_name))
    }

    pub fn game_folder(&self, game_name: &str) -> StrictPath {
        match self.mapping.games.get::<str>(&game_name) {
            Some(game) => game.base.clone(),
            None => {
                let mut safe_name = escape_folder_name(game_name);

                if safe_name.matches(SAFE).count() == safe_name.len() {
                    // It's unreadable now, so do a total rename.
                    safe_name = Self::generate_total_rename(&game_name);
                }

                self.base.joined(&safe_name)
            }
        }
    }

    pub fn game_file(
        &self,
        game_folder: &StrictPath,
        original_file: &StrictPath,
        mapping: &mut IndividualMapping,
    ) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = mapping.drive_folder_name(&drive);
        StrictPath::relative(
            format!("{}/{}", drive_folder, plain_path),
            Some(game_folder.interpret()),
        )
    }

    pub fn game_mapping_file(&self, game_folder: &StrictPath) -> StrictPath {
        game_folder.joined("mapping.yaml")
    }

    #[allow(dead_code)]
    pub fn game_registry_file(&self, game_folder: &StrictPath) -> StrictPath {
        game_folder.joined("registry.yaml")
    }

    pub fn restorable_files(
        &self,
        game_name: &str,
        game_folder: &StrictPath,
    ) -> std::collections::HashSet<ScannedFile> {
        let mut files = std::collections::HashSet::new();
        for drive_dir in walkdir::WalkDir::new(game_folder.interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let raw_drive_dir = drive_dir.path().display().to_string();
            let drive_mapping = match self.mapping.games.get::<str>(&game_name) {
                Some(x) => match x.drives.get::<str>(&drive_dir.file_name().to_string_lossy()) {
                    Some(y) => y,
                    None => continue,
                },
                None => continue,
            };

            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|x| x.file_type().is_file())
            {
                let raw_file = file.path().display().to_string();
                let original_path = Some(StrictPath::new(raw_file.replace(&raw_drive_dir, drive_mapping)));
                files.insert(ScannedFile {
                    path: StrictPath::new(raw_file),
                    size: match file.metadata() {
                        Ok(m) => m.len(),
                        _ => 0,
                    },
                    original_path,
                });
            }
        }
        files
    }

    pub fn remove_irrelevant_backup_files(&self, game_folder: &StrictPath, relevant_files: &[StrictPath]) {
        let relevant_files: Vec<_> = relevant_files.iter().map(|x| x.interpret()).collect();

        for drive_dir in walkdir::WalkDir::new(game_folder.interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|x| x.file_type().is_file())
            {
                let backup_file = StrictPath::new(file.path().display().to_string());
                if !relevant_files.contains(&backup_file.interpret()) {
                    let _ = backup_file.remove();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> String {
        env!("CARGO_MANIFEST_DIR").to_string()
    }

    mod individual_mapping {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn can_generate_drive_folder_name() {
            let mut mapping = IndividualMapping::new("foo".to_owned());
            assert_eq!("drive-0", mapping.drive_folder_name(""));
            assert_eq!("drive-C", mapping.drive_folder_name("C:"));
            assert_eq!("drive-D", mapping.drive_folder_name("D:"));
            assert_eq!("drive-____C", mapping.drive_folder_name(r#"\\?\C:"#));
            assert_eq!("drive-__remote", mapping.drive_folder_name(r#"\\remote"#));
        }
    }

    mod backup_layout {
        use super::*;
        use pretty_assertions::assert_eq;

        fn layout() -> BackupLayout {
            BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo())))
        }

        #[test]
        fn can_find_existing_game_folder_with_matching_name() {
            assert_eq!(
                StrictPath::new(if cfg!(target_os = "windows") {
                    format!("\\\\?\\{}\\tests\\backup\\game1", repo())
                } else {
                    format!("{}/tests/backup/game1", repo())
                }),
                layout().game_folder("game1")
            );
        }

        #[test]
        fn can_find_existing_game_folder_with_rename() {
            assert_eq!(
                StrictPath::new(if cfg!(target_os = "windows") {
                    format!("\\\\?\\{}\\tests\\backup\\game3-renamed", repo())
                } else {
                    format!("{}/tests/backup/game3-renamed", repo())
                }),
                layout().game_folder("game3")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_without_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/nonexistent", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/nonexistent", repo()))
                },
                layout().game_folder("nonexistent")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_partial_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/foo_bar", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/foo_bar", repo()))
                },
                layout().game_folder("foo:bar")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_total_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/ludusavi-renamed-Kioq", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/ludusavi-renamed-Kioq", repo()))
                },
                layout().game_folder("***")
            );
        }

        #[test]
        fn can_determine_game_folder_by_escaping_dots_at_start_and_end() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/_._", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/_._", repo()))
                },
                layout().game_folder("...")
            );
        }
    }
}
