pub fn normalize(path: &str) -> String {
    if path.starts_with('~') {
        path.replace("\\", "/")
            .replacen("~", &dirs::home_dir().unwrap().to_string_lossy(), 1)
    } else {
        path.replace("\\", "/")
    }
}

pub fn is_file(path: &str) -> bool {
    std::path::Path::new(&normalize(path)).is_file()
}

pub fn is_dir(path: &str) -> bool {
    std::path::Path::new(&normalize(path)).is_dir()
}

pub fn exists(path: &str) -> bool {
    is_file(path) || is_dir(path)
}

pub fn absolute(path: &str) -> String {
    match std::fs::canonicalize(&normalize(path)) {
        Ok(x) => render_pathbuf(&x).replace("\\\\?\\", ""),
        Err(_) => normalize(path),
    }
}

pub fn render_pathbuf(value: &std::path::PathBuf) -> String {
    value.as_path().display().to_string()
}
