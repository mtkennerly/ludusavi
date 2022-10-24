use std::collections::HashMap;

use maplit::*;

use crate::path::StrictPath;

pub const EMPTY_HASH: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

pub fn repo() -> String {
    env!("CARGO_MANIFEST_DIR").replace('\\', "/")
}

pub fn mapping_file_key(file: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("X:{file}")
    } else {
        file.to_string()
    }
}

pub fn drives_x() -> HashMap<String, String> {
    if cfg!(target_os = "windows") {
        hashmap! { "X:".into() => "drive-X".into() }
    } else {
        hashmap! { "".into() => "drive-0".into() }
    }
}

pub fn make_original_path(file: &str) -> StrictPath {
    StrictPath::new(format!("{}{file}", if cfg!(target_os = "windows") { "X:" } else { "" }))
}
