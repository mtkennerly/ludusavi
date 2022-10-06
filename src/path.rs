use crate::prelude::AnyError;

use filetime::FileTime;

#[cfg(target_os = "windows")]
const TYPICAL_SEPARATOR: &str = "\\";
#[cfg(target_os = "windows")]
const ATYPICAL_SEPARATOR: &str = "/";

#[cfg(not(target_os = "windows"))]
const TYPICAL_SEPARATOR: &str = "/";
#[cfg(not(target_os = "windows"))]
const ATYPICAL_SEPARATOR: &str = "\\";

#[allow(dead_code)]
const UNC_PREFIX: &str = "\\\\";
#[allow(dead_code)]
const UNC_LOCAL_PREFIX: &str = "\\\\?\\";

fn parse_home(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        path.replacen('~', &dirs::home_dir().unwrap().to_string_lossy(), 1)
    } else {
        path.to_owned()
    }
}

fn normalize(path: &str) -> String {
    let mut path = path.trim().to_string();

    #[cfg(target_os = "windows")]
    if path.starts_with('/') {
        let drive = &render_pathbuf(&std::env::current_dir().unwrap())[..2];
        path = format!("{}{}", drive, path)
    }

    path = parse_home(&path).replace(ATYPICAL_SEPARATOR, TYPICAL_SEPARATOR);

    // On Windows, canonicalizing "C:" or "C:/" yields the current directory,
    // but "C:\" works.
    #[cfg(target_os = "windows")]
    if path.ends_with(':') {
        path += TYPICAL_SEPARATOR;
    }

    path
}

// Based on:
// https://github.com/rust-lang/cargo/blob/f84f3f8c630c75a1ec01b818ff469d3496228c6b/src/cargo/util/paths.rs#L61-L86
fn parse_dots(path: &str, basis: &str) -> String {
    let mut components = std::path::Path::new(&path).components().peekable();
    let mut ret = if let Some(c @ std::path::Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        std::path::PathBuf::from(c.as_os_str())
    } else {
        std::path::PathBuf::from(basis)
    };

    for component in components {
        match component {
            std::path::Component::Prefix(..) => unreachable!(),
            std::path::Component::RootDir => {
                ret.push(component.as_os_str());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                ret.pop();
            }
            std::path::Component::Normal(c) => {
                let lossy = c.to_string_lossy();
                if lossy.contains(':') {
                    // This can happen if the manifest contains invalid paths,
                    // such as `<winDocuments>/<home>`. In this example, `<home>`
                    // means we could try to push `C:` in the middle of the path,
                    // which would truncate the rest of the path up to that point,
                    // causing us to check the entire home folder.
                    // We escape it so that it (likely) just won't be found,
                    // rather than finding something irrelevant.
                    ret.push(lossy.replace(':', "_"));
                } else {
                    ret.push(c);
                }
            }
        }
    }

    render_pathbuf(&ret)
}

/// Convert a raw, possibly user-provided path into a suitable form for internal use.
/// On Windows, this produces UNC paths.
fn interpret<P: Into<String>>(path: P, basis: &Option<String>) -> String {
    let normalized = normalize(&path.into());
    if normalized.is_empty() {
        return normalized;
    }

    let absolutized = if std::path::Path::new(&normalized).is_absolute() {
        normalized
    } else {
        render_pathbuf(
            &match basis {
                None => std::env::current_dir().unwrap(),
                Some(b) => std::path::Path::new(&normalize(b)).to_path_buf(),
            }
            .join(normalized),
        )
    };

    match std::fs::canonicalize(&absolutized) {
        Ok(x) => render_pathbuf(&x),
        Err(_) => {
            let dedotted = parse_dots(
                &absolutized,
                &render_pathbuf(&match basis {
                    None => std::env::current_dir().unwrap(),
                    Some(b) => std::path::Path::new(&normalize(b)).to_path_buf(),
                }),
            );
            format!(
                "{}{}",
                if cfg!(target_os = "windows") && !dedotted.starts_with(UNC_LOCAL_PREFIX) {
                    UNC_LOCAL_PREFIX
                } else {
                    ""
                },
                dedotted.replace(ATYPICAL_SEPARATOR, TYPICAL_SEPARATOR)
            )
        }
    }
}

/// Convert a path into a nice form for display and storage.
/// On Windows, this produces non-UNC paths.
fn render<P: Into<String>>(path: P) -> String {
    path.into().replace(UNC_LOCAL_PREFIX, "").replace('\\', "/")
}

pub fn render_pathbuf(value: &std::path::Path) -> String {
    value.display().to_string()
}

/// Convert a path into a format that is amenable to zipped comparison when splitting on `/`.
/// The resulting path should not be used for actual file lookup.
/// This relies on `render()` removing UNC prefixes when possible, so that
/// `C:` and `\\?\C:` will end up normalizing to `C:`.
/// For Linux-style paths, `C:` is inserted before path-initial `/` to avoid the split vec
/// starting with `""`.
fn splittable(path: &StrictPath) -> String {
    let rendered = path.render();
    let prefixed = if rendered.starts_with('/') {
        format!("C:{}", rendered)
    } else {
        rendered
    };
    match prefixed.strip_suffix('/') {
        Some(x) => x.to_string(),
        _ => prefixed,
    }
}

/// This is a wrapper around paths to make it more obvious when we're
/// converting between different representations. This also handles
/// things like `~`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StrictPath {
    raw: String,
    basis: Option<String>,
}

impl StrictPath {
    pub fn new(raw: String) -> Self {
        Self { raw, basis: None }
    }

    pub fn relative(raw: String, basis: Option<String>) -> Self {
        Self { raw, basis }
    }

    pub fn reset(&mut self, raw: String) {
        self.raw = raw;
    }

    pub fn from_std_path_buf(path_buf: &std::path::Path) -> Self {
        Self::new(render_pathbuf(path_buf))
    }

    pub fn as_std_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.interpret())
    }

    pub fn raw(&self) -> String {
        self.raw.to_string()
    }

    pub fn interpret(&self) -> String {
        interpret(&self.raw, &self.basis)
    }

    pub fn interpreted(&self) -> Self {
        Self {
            raw: self.interpret(),
            basis: self.basis.clone(),
        }
    }

    pub fn render(&self) -> String {
        render(self.interpret())
    }

    pub fn rendered(&self) -> Self {
        Self {
            raw: self.render(),
            basis: self.basis.clone(),
        }
    }

    pub fn is_file(&self) -> bool {
        std::path::Path::new(&self.interpret()).is_file()
    }

    pub fn is_dir(&self) -> bool {
        std::path::Path::new(&self.interpret()).is_dir()
    }

    pub fn exists(&self) -> bool {
        self.is_file() || self.is_dir()
    }

    pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
        self.as_std_path_buf().metadata()
    }

    pub fn remove(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_file() {
            std::fs::remove_file(&self.interpret())?;
        } else if self.is_dir() {
            std::fs::remove_dir_all(&self.interpret())?;
        }
        Ok(())
    }

    pub fn joined(&self, other: &str) -> Self {
        Self::new(format!("{}{}{}", self.interpret(), TYPICAL_SEPARATOR, other))
    }

    pub fn create_parent_dir(&self) -> std::io::Result<()> {
        let mut pb = self.as_std_path_buf();
        pb.pop();
        std::fs::create_dir_all(&pb)?;
        Ok(())
    }

    pub fn copy_to_path(&self, name: &String, attempt: u8, target_file: &StrictPath) -> Result<(), std::io::Error> {
        log::trace!(
            "[{name}] copy_to_path {} -> {}",
            self.interpret(),
            target_file.interpret()
        );

        if let Err(e) = target_file.create_parent_dir() {
            log::error!(
                "[{}] unable to create parent directories: {} -> {} | {e}",
                name,
                self.raw(),
                target_file.raw()
            );
            return Err(e);
        }

        // SL: I wonder which circumstances will have a ro target_file... maybe
        // a Windows specific issue?
        //
        // taken from GameLayout::restore_file_from_simple
        if let Err(e) = target_file.unset_readonly() {
            log::warn!(
                "[{}] try {attempt}, failed to unset read-only on target: {} | {e}",
                name,
                target_file.raw()
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to unset read-only",
            ));
        } else if let Err(e) = std::fs::copy(&self.interpret(), &target_file.interpret()) {
            log::error!(
                "[{}] unable to copy: {} -> {} | {e}",
                name,
                self.raw(),
                target_file.raw()
            );
            return Err(e);
        } else {
            // #132: SL honor timestamps - set timestamp of file based on file metadata
            let mtime = FileTime::from_system_time(self.metadata().unwrap().modified().unwrap());
            if let Err(e) = filetime::set_file_mtime(target_file.interpret(), mtime) {
                log::error!(
                    "[{}] unable to set modification time: {} -> {} to {:#?} | {e}",
                    name,
                    self.raw(),
                    target_file.raw(),
                    mtime
                );
                return Err(e);
            }
        }
        return Ok(());
    }

    /// This splits a path into a drive (e.g., `C:` or `\\?\D:`) and the remainder.
    /// This is only used during backups to record drives in mapping.yaml, so it
    /// only has to deal with paths that can occur on the host OS.
    #[cfg(target_os = "windows")]
    pub fn split_drive(&self) -> (String, String) {
        if &self.raw[0..1] == "/" && &self.raw[1..2] != "/" {
            // Needed when restoring Linux created backups on Windows
            (
                "".to_owned(),
                if self.raw.starts_with('/') {
                    self.raw[1..].to_string()
                } else {
                    self.raw.to_string()
                },
            )
        } else {
            let interpreted = self.interpret();

            if let Some(stripped) = interpreted.strip_prefix(UNC_LOCAL_PREFIX) {
                // Local UNC path - simplify to a classic drive for user-friendliness:
                let split: Vec<_> = stripped.splitn(2, '\\').collect();
                if split.len() == 2 {
                    return (split[0].to_owned(), split[1].replace('\\', "/"));
                }
            } else if let Some(stripped) = interpreted.strip_prefix(UNC_PREFIX) {
                // Remote UNC path - can't simplify to classic drive:
                let split: Vec<_> = stripped.splitn(2, '\\').collect();
                if split.len() == 2 {
                    return (format!("{}{}", UNC_PREFIX, split[0]), split[1].replace('\\', "/"));
                }
            }

            // This shouldn't normally happen, but we have a fallback just in case.
            ("".to_owned(), self.raw.replace('\\', "/"))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn split_drive(&self) -> (String, String) {
        if &self.raw[1..3] == ":/" {
            // Needed for the cased that a ZIP was created on Windows but we restore via Linux
            (self.raw[0..1].to_owned(), self.raw[3..].to_owned())
        } else {
            (
                "".to_owned(),
                if self.raw.starts_with('/') {
                    self.raw[1..].to_string()
                } else {
                    self.raw.to_string()
                },
            )
        }
    }

    pub fn unset_readonly(&self) -> Result<(), AnyError> {
        let interpreted = self.interpret();
        if self.is_file() {
            let mut perms = std::fs::metadata(&interpreted)?.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                std::fs::set_permissions(&interpreted, perms)?;
            }
        } else {
            for entry in walkdir::WalkDir::new(interpreted)
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .skip(1) // the base path itself
                .filter_map(crate::prelude::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let file = &mut entry.path().display().to_string();
                let mut perms = std::fs::metadata(&file)?.permissions();
                if perms.readonly() {
                    perms.set_readonly(false);
                    std::fs::set_permissions(&file, perms)?;
                }
            }
        }

        Ok(())
    }

    pub fn is_prefix_of(&self, other: &StrictPath) -> bool {
        let us_rendered = splittable(self);
        let them_rendered = splittable(other);

        let us_components = us_rendered.split('/');
        let them_components = them_rendered.split('/');

        if us_components.clone().count() >= them_components.clone().count() {
            return false;
        }
        us_components.zip(them_components).all(|(us, them)| us == them)
    }

    pub fn nearest_prefix(&self, others: Vec<StrictPath>) -> Option<StrictPath> {
        let us_rendered = splittable(self);
        let us_components = us_rendered.split('/');
        let us_count = us_components.clone().count();

        let mut nearest = None;
        let mut nearest_len = 0;
        for other in others {
            let them_rendered = splittable(&other);
            let them_components = them_rendered.split('/');
            let them_len = them_components.clone().count();

            if us_count <= them_len {
                continue;
            }
            if us_components.clone().zip(them_components).all(|(us, them)| us == them) && them_len > nearest_len {
                nearest = Some(other.clone());
                nearest_len = them_len;
            }
        }
        nearest
    }

    pub fn glob(&self) -> Vec<StrictPath> {
        let options = glob::MatchOptions {
            case_sensitive: crate::prelude::CASE_INSENSITIVE_OS,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };
        match glob::glob_with(&self.render(), options) {
            Ok(xs) => xs.filter_map(|r| r.ok()).map(StrictPath::from).collect(),
            Err(_) => vec![],
        }
    }

    pub fn same_content(&self, other: &StrictPath) -> bool {
        self.try_same_content(other).unwrap_or(false)
    }

    pub fn try_same_content(&self, other: &StrictPath) -> Result<bool, Box<dyn std::error::Error>> {
        use std::io::Read;

        let f1 = std::fs::File::open(self.interpret())?;
        let mut f1r = std::io::BufReader::new(f1);
        let f2 = std::fs::File::open(other.interpret())?;
        let mut f2r = std::io::BufReader::new(f2);

        let mut f1b = [0; 1024];
        let mut f2b = [0; 1024];
        loop {
            let f1n = f1r.read(&mut f1b[..])?;
            let f2n = f2r.read(&mut f2b[..])?;

            if f1n != f2n || f1b.iter().zip(f2b.iter()).any(|(a, b)| a != b) {
                return Ok(false);
            }
            if f1n == 0 || f2n == 0 {
                break;
            }
        }
        Ok(true)
    }

    pub fn try_same_content_as_zip(&self, other: &mut zip::read::ZipFile) -> Result<bool, Box<dyn std::error::Error>> {
        use std::io::Read;

        let handle = std::fs::File::open(self.interpret())?;
        let mut reader = std::io::BufReader::new(handle);

        let mut disk_buffer = [0; 1024];
        let mut zip_buffer = [0; 1024];
        loop {
            let read_disk = reader.read(&mut disk_buffer[..])?;
            let read_zip = other.read(&mut zip_buffer[..])?;

            if read_disk != read_zip || disk_buffer.iter().zip(zip_buffer.iter()).any(|(a, b)| a != b) {
                return Ok(false);
            }
            if read_disk == 0 || read_zip == 0 {
                break;
            }
        }
        Ok(true)
    }

    pub fn read(&self) -> Option<String> {
        std::fs::read_to_string(&std::path::Path::new(&self.interpret())).ok()
    }

    pub fn size(&self) -> u64 {
        match self.metadata() {
            Ok(m) => m.len(),
            _ => 0,
        }
    }

    pub fn sha1(&self) -> String {
        self.try_sha1().unwrap_or_default()
    }

    fn try_sha1(&self) -> Result<String, Box<dyn std::error::Error>> {
        use sha1::Digest;
        use std::io::Read;

        let mut hasher = sha1::Sha1::new();

        let file = std::fs::File::open(self.interpret())?;
        let mut reader = std::io::BufReader::new(file);

        let mut buffer = [0; 1024];
        loop {
            let read = reader.read(&mut buffer[..])?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

impl From<&str> for StrictPath {
    fn from(source: &str) -> Self {
        StrictPath::new(source.to_string())
    }
}

impl From<&String> for StrictPath {
    fn from(source: &String) -> Self {
        StrictPath::new(source.clone())
    }
}

impl From<std::path::PathBuf> for StrictPath {
    fn from(source: std::path::PathBuf) -> Self {
        StrictPath::from_std_path_buf(&source)
    }
}

impl From<&std::path::Path> for StrictPath {
    fn from(source: &std::path::Path) -> Self {
        StrictPath::from_std_path_buf(source)
    }
}

impl From<&walkdir::DirEntry> for StrictPath {
    fn from(source: &walkdir::DirEntry) -> Self {
        StrictPath::from_std_path_buf(source.path())
    }
}

// Based on:
// https://github.com/serde-rs/serde/issues/751#issuecomment-277580700
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct StrictPathSerdeHelper(String);

impl serde::Serialize for StrictPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        StrictPathSerdeHelper(self.raw()).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for StrictPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer).map(|StrictPathSerdeHelper(raw)| StrictPath::new(raw))
    }
}

#[allow(dead_code)]
pub fn is_raw_path_relative(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.is_empty() || path == "." || path == ".." || path.starts_with("./") || path.starts_with("../")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> String {
        text.to_string()
    }

    fn repo() -> String {
        env!("CARGO_MANIFEST_DIR").to_owned()
    }

    fn username() -> String {
        whoami::username()
    }

    fn home() -> String {
        render_pathbuf(&dirs::home_dir().unwrap())
    }

    fn drive() -> String {
        if cfg!(target_os = "windows") {
            StrictPath::new(s("foo")).render()[..2].to_string()
        } else {
            s("")
        }
    }

    mod strict_path {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn can_interpret_general_paths() {
            if cfg!(target_os = "windows") {
                assert_eq!("".to_string(), interpret("", &Some("/foo".to_string())));
                assert_eq!(
                    format!(r#"\\?\{}\foo\bar"#, drive()),
                    interpret("bar", &Some("/foo".to_string()))
                );
            } else {
                assert_eq!("".to_string(), interpret("", &Some("/foo".to_string())));
                assert_eq!("/foo/bar".to_string(), interpret("bar", &Some("/foo".to_string())));
            }
        }

        #[test]
        fn can_interpret_linux_style_paths() {
            if cfg!(target_os = "windows") {
                assert_eq!(format!(r#"\\?\{}\"#, drive()), interpret("/", &None));
                assert_eq!(format!(r#"\\?\{}\foo"#, drive()), interpret("/foo", &None));
                assert_eq!(format!(r#"\\?\{}\foo\bar"#, drive()), interpret("/foo/bar", &None));
            } else {
                assert_eq!("/".to_string(), interpret("/", &None));
                assert_eq!("/foo".to_string(), interpret("/foo", &None));
                assert_eq!("/foo/bar".to_string(), interpret("/foo/bar", &None));
            }
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_interpret_windows_drive_letter() {
            assert_eq!(r#"\\?\C:\foo"#.to_string(), interpret("C:/foo", &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret("C:\\", &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret("C:/", &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret("C:", &None));
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_interpret_unc_path() {
            assert_eq!(r#"\\?\C:\foo"#.to_string(), interpret(r#"\\?\C:\foo"#, &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret(r#"\\?\C:\"#, &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret(r#"\\?\C:/"#, &None));
            assert_eq!(r#"\\?\C:\"#.to_string(), interpret(r#"\\?\C:"#, &None));
        }

        #[test]
        fn can_render() {
            assert_eq!("".to_string(), render(""));
            assert_eq!("/".to_string(), render("/"));
            assert_eq!("/foo".to_string(), render("/foo"));
            assert_eq!("/foo/bar".to_string(), render("/foo/bar"));
            assert_eq!("/foo/bar/".to_string(), render("\\foo/bar/"));
            assert_eq!("C:/foo".to_string(), render("C:/foo"));
        }

        #[test]
        fn expands_relative_paths_from_working_dir_by_default() {
            let sp = StrictPath::new("README.md".to_owned());
            if cfg!(target_os = "windows") {
                assert_eq!(format!("\\\\?\\{}\\README.md", repo()), sp.interpret());
            } else {
                assert_eq!(format!("{}/README.md", repo()), sp.interpret());
            }
        }

        #[test]
        fn expands_relative_paths_from_specified_basis_dir() {
            if cfg!(target_os = "windows") {
                let sp = StrictPath::relative("README.md".to_owned(), Some("C:\\tmp".to_string()));
                assert_eq!("\\\\?\\C:\\tmp\\README.md", sp.interpret());
            } else {
                let sp = StrictPath::relative("README.md".to_owned(), Some("/tmp".to_string()));
                assert_eq!("/tmp/README.md", sp.interpret());
            }
        }

        #[test]
        fn converts_single_dot_at_start_of_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::new("./README.md".to_owned()).render(),
            );
        }

        #[test]
        fn converts_single_dots_at_start_of_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::new("./././README.md".to_owned()).render(),
            );
        }

        #[test]
        fn converts_single_dot_at_start_of_fake_path() {
            assert_eq!(
                format!("{}/fake/README.md", repo()).replace('\\', "/"),
                StrictPath::relative("./README.md".to_owned(), Some(format!("{}/fake", repo()))).render(),
            );
        }

        #[test]
        fn converts_single_dot_within_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::new(format!("{}/./README.md", repo())).render(),
            );
        }

        #[test]
        fn converts_single_dots_within_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::new(format!("{}/./././README.md", repo())).render(),
            );
        }

        #[test]
        fn converts_single_dot_within_fake_path() {
            assert_eq!(
                format!("{}/fake/README.md", repo()).replace('\\', "/"),
                StrictPath::new(format!("{}/fake/./README.md", repo())).render(),
            );
        }

        #[test]
        fn converts_double_dots_at_start_of_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::relative("../README.md".to_owned(), Some(format!("{}/src", repo()))).render(),
            );
        }

        #[test]
        fn converts_double_dots_at_start_of_fake_path() {
            assert_eq!(
                format!("{}/fake.md", repo()).replace('\\', "/"),
                StrictPath::relative("../fake.md".to_owned(), Some(format!("{}/fake", repo()))).render(),
            );
        }

        #[test]
        fn converts_double_dots_within_real_path() {
            assert_eq!(
                format!("{}/README.md", repo()).replace('\\', "/"),
                StrictPath::new(format!("{}/src/../README.md", repo())).render(),
            );
        }

        #[test]
        fn converts_double_dots_within_fake_path() {
            assert_eq!(
                format!("{}/fake.md", repo()).replace('\\', "/"),
                StrictPath::new(format!("{}/fake/../fake.md", repo())).render(),
            );
        }

        #[test]
        fn treats_absolute_paths_as_such() {
            if cfg!(target_os = "windows") {
                let sp = StrictPath::new("C:\\tmp\\README.md".to_owned());
                assert_eq!("\\\\?\\C:\\tmp\\README.md", sp.interpret());
            } else {
                let sp = StrictPath::new("/tmp/README.md".to_owned());
                assert_eq!("/tmp/README.md", sp.interpret());
            }
        }

        #[test]
        fn converts_tilde_in_isolation() {
            let sp = StrictPath::new("~".to_owned());
            if cfg!(target_os = "windows") {
                assert_eq!(format!("\\\\?\\C:\\Users\\{}", username()), sp.interpret());
                assert_eq!(format!("C:/Users/{}", username()), sp.render());
            } else {
                assert_eq!(home(), sp.interpret());
                assert_eq!(home(), sp.render());
            }
        }

        #[test]
        fn converts_tilde_before_forward_slash() {
            let sp = StrictPath::new("~/~".to_owned());
            if cfg!(target_os = "windows") {
                assert_eq!(format!("\\\\?\\C:\\Users\\{}\\~", username()), sp.interpret());
                assert_eq!(format!("C:/Users/{}/~", username()), sp.render());
            } else {
                assert_eq!(format!("{}/~", home()), sp.interpret());
                assert_eq!(format!("{}/~", home()), sp.render());
            }
        }

        #[test]
        fn converts_tilde_before_backslash() {
            let sp = StrictPath::new("~\\~".to_owned());
            if cfg!(target_os = "windows") {
                assert_eq!(format!("\\\\?\\C:\\Users\\{}\\~", username()), sp.interpret());
                assert_eq!(format!("C:/Users/{}/~", username()), sp.render());
            } else {
                assert_eq!(format!("{}/~", home()), sp.interpret());
                assert_eq!(format!("{}/~", home()), sp.render());
            }
        }

        #[test]
        fn does_not_convert_tilde_before_a_nonslash_character() {
            let sp = StrictPath::new("~a".to_owned());
            if cfg!(target_os = "windows") {
                assert_eq!(format!("\\\\?\\{}\\~a", repo()), sp.interpret());
            } else {
                assert_eq!(format!("{}/~a", repo()), sp.interpret());
            }
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn does_not_truncate_path_up_to_drive_letter_in_classic_path() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let sp = StrictPath {
                raw: "C:\\Users\\Foo\\Documents/C:\\Users\\Bar".to_string(),
                basis: Some("\\\\?\\C:\\Users\\Foo\\.config\\ludusavi".to_string()),
            };
            assert_eq!(r#"\\?\C:\Users\Foo\Documents\C_\Users\Bar"#, sp.interpret());
            assert_eq!("C:/Users/Foo/Documents/C_/Users/Bar", sp.render());
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn does_not_truncate_path_up_to_drive_letter_in_unc_path() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let sp = StrictPath {
                raw: "\\\\?\\C:\\Users\\Foo\\Documents\\C:\\Users\\Bar".to_string(),
                basis: Some("\\\\?\\C:\\Users\\Foo\\.config\\ludusavi".to_string()),
            };
            assert_eq!(r#"\\?\C:\Users\Foo\Documents\C_\Users\Bar"#, sp.interpret());
            assert_eq!("C:/Users/Foo/Documents/C_/Users/Bar", sp.render());
        }

        #[test]
        fn can_check_if_it_is_a_file() {
            assert!(StrictPath::new(format!("{}/README.md", repo())).is_file());
            assert!(!StrictPath::new(repo()).is_file());
        }

        #[test]
        fn can_check_if_it_is_a_directory() {
            assert!(StrictPath::new(repo()).is_dir());
            assert!(!StrictPath::new(format!("{}/README.md", repo())).is_dir());
        }

        #[test]
        fn can_check_if_it_exists() {
            assert!(StrictPath::new(repo()).exists());
            assert!(StrictPath::new(format!("{}/README.md", repo())).exists());
            assert!(!StrictPath::new(format!("{}/fake", repo())).exists());
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_split_drive_for_windows_path() {
            assert_eq!((s("C:"), s("foo/bar")), StrictPath::new(s("C:/foo/bar")).split_drive());
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_split_drive_for_local_unc_path() {
            assert_eq!(
                (s("C:"), s("foo/bar")),
                StrictPath::new(s(r#"\\?\C:\foo\bar"#)).split_drive()
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_split_drive_for_remote_unc_path() {
            // TODO: Should be `\\remote` and `foo\bar`.
            // Despite this, when backing up to a machine-local network share,
            // it gets resolved to the actual local drive and therefore works.
            // Unsure about behavior for a remote network share at this time.
            assert_eq!(
                (s(""), s("/remote/foo/bar")),
                StrictPath::new(s(r#"\\remote\foo\bar"#)).split_drive()
            );
        }

        #[test]
        #[cfg(not(target_os = "windows"))]
        fn can_split_drive_for_nonwindows_path() {
            assert_eq!((s(""), s("foo/bar")), StrictPath::new(s("/foo/bar")).split_drive());
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_split_drive_for_linux_path_in_windows() {
            assert_eq!(
                (s(""), s("Users/foo/AppData")),
                StrictPath::new(s("/Users/foo/AppData")).split_drive()
            );
        }

        #[test]
        #[cfg(not(target_os = "windows"))]
        fn can_split_drive_for_windows_path_in_linux() {
            assert_eq!(
                (s("C"), s("Users/foo/AppData")),
                StrictPath::new(s("C:/Users/foo/AppData")).split_drive()
            );
        }

        #[test]
        fn is_prefix_of() {
            assert!(StrictPath::new(s("/")).is_prefix_of(&StrictPath::new(s("/foo"))));
            assert!(StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/foo/bar"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/f"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/foo"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/bar"))));
            assert!(!StrictPath::new(s("")).is_prefix_of(&StrictPath::new(s("/foo"))));
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn is_prefix_of_with_windows_drive_letters() {
            assert!(StrictPath::new(s(r#"C:"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:/"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:\"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn is_prefix_of_with_unc_drives() {
            assert!(!StrictPath::new(s(r#"\\?\C:\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"\\?\C:\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo/bar"))));
            assert!(!StrictPath::new(s(r#"\\remote\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:\"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
        }

        #[test]
        fn nearest_prefix() {
            assert_eq!(
                Some(StrictPath::new(s(r#"/foo/bar"#))),
                StrictPath::new(s(r#"/foo/bar/baz"#)).nearest_prefix(vec![
                    StrictPath::new(s(r#"/foo"#)),
                    StrictPath::new(s(r#"/foo/bar"#)),
                    StrictPath::new(s(r#"/foo/bar/baz"#)),
                ])
            );
            assert_eq!(
                None,
                StrictPath::new(s(r#"/foo/bar/baz"#)).nearest_prefix(vec![
                    StrictPath::new(s(r#"/fo"#)),
                    StrictPath::new(s(r#"/fooo"#)),
                    StrictPath::new(s(r#"/foo/bar/baz"#)),
                ])
            );
        }

        #[test]
        fn checks_if_files_are_identical() {
            assert!(StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo()))
                .same_content(&StrictPath::new(format!("{}/tests/root2/game2/file1.txt", repo()))));
            assert!(
                !StrictPath::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()))
                    .same_content(&StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo())))
            );
            assert!(!StrictPath::new(format!("{}/tests/root1/game1/file1.txt", repo()))
                .same_content(&StrictPath::new(format!("{}/nonexistent.txt", repo()))));
        }

        #[test]
        fn tries_to_check_if_files_are_identical() {
            assert!(StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo()))
                .try_same_content(&StrictPath::new(format!("{}/tests/root2/game2/file1.txt", repo())))
                .unwrap());
            assert!(
                !StrictPath::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()))
                    .try_same_content(&StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo())))
                    .unwrap()
            );
            assert!(StrictPath::new(format!("{}/tests/root1/game1/file1.txt", repo()))
                .try_same_content(&StrictPath::new(format!("{}/nonexistent.txt", repo())))
                .is_err());
        }
    }
}
