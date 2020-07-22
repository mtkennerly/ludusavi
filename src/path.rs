#[cfg(target_os = "windows")]
const TYPICAL_SEPARATOR: &str = "\\";
#[cfg(target_os = "windows")]
const ATYPICAL_SEPARATOR: &str = "/";

#[cfg(not(target_os = "windows"))]
const TYPICAL_SEPARATOR: &str = "/";
#[cfg(not(target_os = "windows"))]
const ATYPICAL_SEPARATOR: &str = "\\";

const UNC_PREFIX: &str = "\\\\?\\";

fn parse_home(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        path.replacen("~", &dirs::home_dir().unwrap().to_string_lossy(), 1)
    } else {
        path.to_owned()
    }
}

fn normalize(path: &str) -> String {
    parse_home(path).replace(ATYPICAL_SEPARATOR, TYPICAL_SEPARATOR)
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
                ret.push(c);
            }
        }
    }

    render_pathbuf(&ret)
}

/// Convert a raw, possibly user-provided path into a suitable form for internal use.
/// On Windows, this produces UNC paths.
fn interpret<P: Into<String>>(path: P, basis: &Option<String>) -> String {
    let normalized = normalize(&path.into());
    let absolutized = if std::path::Path::new(&normalized).is_absolute() {
        normalized
    } else {
        render_pathbuf(
            &match basis {
                None => std::env::current_dir().unwrap(),
                Some(b) => std::path::Path::new(b).to_path_buf(),
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
                    Some(b) => std::path::Path::new(b).to_path_buf(),
                }),
            );
            format!(
                "{}{}",
                if cfg!(target_os = "windows") { UNC_PREFIX } else { "" },
                dedotted.replace(ATYPICAL_SEPARATOR, TYPICAL_SEPARATOR)
            )
        }
    }
}

/// Convert a path into a nice form for display and storage.
/// On Windows, this produces non-UNC paths.
fn render<P: Into<String>>(path: P) -> String {
    path.into().replace(UNC_PREFIX, "").replace("\\", "/")
}

fn render_pathbuf(value: &std::path::PathBuf) -> String {
    value.as_path().display().to_string()
}

fn count_subdirectories(path: &str) -> usize {
    walkdir::WalkDir::new(normalize(path))
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .skip(1)
        .filter_map(|e| e.ok())
        .filter(|x| x.file_type().is_dir())
        .count()
}

/// This is a wrapper around paths to make it more obvious when we're
/// converting between different representations. This also handles
/// things like `~`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StrictPath {
    raw: String,
    basis: Option<String>,
    interpreted: String,
}

impl StrictPath {
    pub fn new(raw: String) -> Self {
        let interpreted = interpret(&raw, &None);
        Self {
            raw,
            basis: None,
            interpreted,
        }
    }

    pub fn relative(raw: String, basis: Option<String>) -> Self {
        let interpreted = interpret(&raw, &basis);
        Self {
            raw,
            basis,
            interpreted,
        }
    }

    pub fn reset(&mut self, raw: String) {
        self.raw = raw;
        self.interpreted = interpret(&self.raw, &self.basis);
    }

    pub fn from_std_path_buf(path_buf: &std::path::PathBuf) -> Self {
        Self::new(render_pathbuf(&path_buf))
    }

    pub fn as_std_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.interpret())
    }

    pub fn raw(&self) -> String {
        self.raw.to_string()
    }

    pub fn interpret(&self) -> String {
        self.interpreted.to_string()
    }

    pub fn render(&self) -> String {
        render(self.interpreted.to_string())
    }

    pub fn is_file(&self) -> bool {
        std::path::Path::new(&self.interpreted).is_file()
    }

    pub fn is_dir(&self) -> bool {
        std::path::Path::new(&self.interpreted).is_dir()
    }

    pub fn exists(&self) -> bool {
        self.is_file() || self.is_dir()
    }

    pub fn remove(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_file() {
            std::fs::remove_file(&self.interpreted)?;
        } else if self.is_dir() {
            std::fs::remove_dir_all(&self.interpreted)?;
        }
        Ok(())
    }

    pub fn count_subdirectories(&self) -> usize {
        count_subdirectories(&self.interpret())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> String {
        env!("CARGO_MANIFEST_DIR").to_owned()
    }

    fn username() -> String {
        whoami::username()
    }

    mod strict_path {
        use super::*;

        #[test]
        fn expands_relative_paths_from_working_dir_by_default() {
            let sp = StrictPath::new("README.md".to_owned());
            #[cfg(target_os = "windows")]
            {
                assert_eq!(sp.interpret(), format!("\\\\?\\{}\\README.md", repo()));
            }
            #[cfg(target_os = "linux")]
            {
                assert_eq!(sp.interpret(), format!("{}/README.md", repo()));
            }
        }

        #[test]
        fn expands_relative_paths_from_specified_basis_dir() {
            #[cfg(target_os = "windows")]
            {
                let sp = StrictPath::relative("README.md".to_owned(), Some("C:\\tmp".to_string()));
                assert_eq!(sp.interpret(), "\\\\?\\C:\\tmp\\README.md");
            }
            #[cfg(target_os = "linux")]
            {
                let sp = StrictPath::relative("README.md".to_owned(), Some("/tmp".to_string()));
                assert_eq!(sp.interpret(), "/tmp/README.md");
            }
        }

        #[test]
        fn converts_single_dot_at_start_of_real_path() {
            assert_eq!(
                StrictPath::new("./README.md".to_owned()).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_single_dots_at_start_of_real_path() {
            assert_eq!(
                StrictPath::new("./././README.md".to_owned()).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_single_dot_at_start_of_fake_path() {
            assert_eq!(
                StrictPath::relative("./README.md".to_owned(), Some(format!("{}/fake", repo()))).render(),
                format!("{}/fake/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_single_dot_within_real_path() {
            assert_eq!(
                StrictPath::new(format!("{}/./README.md", repo())).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_single_dots_within_real_path() {
            assert_eq!(
                StrictPath::new(format!("{}/./././README.md", repo())).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_single_dot_within_fake_path() {
            assert_eq!(
                StrictPath::new(format!("{}/fake/./README.md", repo())).render(),
                format!("{}/fake/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_double_dots_at_start_of_real_path() {
            assert_eq!(
                StrictPath::relative("../README.md".to_owned(), Some(format!("{}/src", repo()))).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_double_dots_at_start_of_fake_path() {
            assert_eq!(
                StrictPath::relative("../fake.md".to_owned(), Some(format!("{}/fake", repo()))).render(),
                format!("{}/fake.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_double_dots_within_real_path() {
            assert_eq!(
                StrictPath::new(format!("{}/src/../README.md", repo())).render(),
                format!("{}/README.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn converts_double_dots_within_fake_path() {
            assert_eq!(
                StrictPath::new(format!("{}/fake/../fake.md", repo())).render(),
                format!("{}/fake.md", repo()).replace("\\", "/")
            );
        }

        #[test]
        fn treats_absolute_paths_as_such() {
            #[cfg(target_os = "windows")]
            {
                let sp = StrictPath::new("C:\\tmp\\README.md".to_owned());
                assert_eq!(sp.interpret(), "\\\\?\\C:\\tmp\\README.md");
            }
            #[cfg(target_os = "linux")]
            {
                let sp = StrictPath::new("/tmp/README.md".to_owned());
                assert_eq!(sp.interpret(), "/tmp/README.md");
            }
        }

        #[test]
        fn converts_tilde_in_isolation() {
            #[cfg(target_os = "windows")]
            {
                let sp = StrictPath::new("~".to_owned());
                assert_eq!(sp.interpret(), format!("\\\\?\\C:\\Users\\{}", username()));
                assert_eq!(sp.render(), format!("C:/Users/{}", username()));
            }
            #[cfg(target_os = "linux")]
            {
                let sp = StrictPath::new("~".to_owned());
                assert_eq!(sp.interpret(), format!("/home/{}", username()));
                assert_eq!(sp.render(), format!("/home/{}", username()));
            }
        }

        #[test]
        fn converts_tilde_before_forward_slash() {
            #[cfg(target_os = "windows")]
            {
                let sp = StrictPath::new("~/~".to_owned());
                assert_eq!(sp.interpret(), format!("\\\\?\\C:\\Users\\{}\\~", username()));
                assert_eq!(sp.render(), format!("C:/Users/{}/~", username()));
            }
            #[cfg(target_os = "linux")]
            {
                let sp = StrictPath::new("~/~".to_owned());
                assert_eq!(sp.interpret(), format!("/home/{}/~", username()));
                assert_eq!(sp.render(), format!("/home/{}/~", username()));
            }
        }

        #[test]
        fn converts_tilde_before_backslash() {
            #[cfg(target_os = "windows")]
            {
                let sp = StrictPath::new("~\\~".to_owned());
                assert_eq!(sp.interpret(), format!("\\\\?\\C:\\Users\\{}\\~", username()));
                assert_eq!(sp.render(), format!("C:/Users/{}/~", username()));
            }
            #[cfg(target_os = "linux")]
            {
                let sp = StrictPath::new("~\\~".to_owned());
                assert_eq!(sp.interpret(), format!("/home/{}/~", username()));
                assert_eq!(sp.render(), format!("/home/{}/~", username()));
            }
        }

        #[test]
        fn does_not_convert_tilde_before_a_nonslash_character() {
            let sp = StrictPath::new("~a".to_owned());
            #[cfg(target_os = "windows")]
            {
                assert_eq!(sp.interpret(), format!("\\\\?\\{}\\~a", repo()));
            }
            #[cfg(target_os = "linux")]
            {
                assert_eq!(sp.interpret(), format!("{}/~a", repo()));
            }
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
    }
}
