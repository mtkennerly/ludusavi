use std::collections::BTreeMap;

use velcro::btree_map;

use crate::path::StrictPath;

pub const EMPTY_HASH: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

pub fn repo() -> String {
    repo_raw().replace('\\', "/")
}

pub fn repo_raw() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

pub fn repo_file(path: &str) -> String {
    repo_file_raw(path).replace('\\', "/")
}

pub fn repo_file_raw(path: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}\\{}", repo_raw(), path.replace('/', "\\"))
    } else {
        format!("{}/{}", repo_raw(), path)
    }
}

pub fn repo_path(path: &str) -> StrictPath {
    StrictPath::new(repo_file(path))
}

pub fn repo_path_raw(path: &str) -> StrictPath {
    StrictPath::new(repo_file_raw(path))
}

pub fn absolute_path(file: &str) -> StrictPath {
    if cfg!(target_os = "windows") {
        StrictPath::new(format!("X:{file}"))
    } else {
        StrictPath::new(file.to_string())
    }
}

pub fn mapping_file_key(file: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("X:{file}")
    } else {
        file.to_string()
    }
}

pub fn drives_x() -> BTreeMap<String, String> {
    if cfg!(target_os = "windows") {
        btree_map! { "drive-X".into(): "X:".into() }
    } else {
        btree_map! { "drive-0".into(): "".into() }
    }
}

pub fn drives_x_always() -> BTreeMap<String, String> {
    if cfg!(target_os = "windows") {
        btree_map! { "drive-X".into(): "X:".into() }
    } else {
        btree_map! { "drive-X".into(): "".into() }
    }
}

pub fn drives_x_static() -> BTreeMap<String, String> {
    btree_map! { "drive-X".into(): "X:".into() }
}

pub fn make_original_path(file: &str) -> StrictPath {
    StrictPath::new(format!("{}{file}", if cfg!(target_os = "windows") { "X:" } else { "" }))
}

pub fn s(text: &str) -> String {
    text.to_string()
}
