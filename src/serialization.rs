use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Serialize, Serializer};

#[allow(dead_code)]
pub fn ordered_map<S, V>(value: &HashMap<String, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    V: Serialize,
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub fn ordered_set<S>(value: &HashSet<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut ordered: Vec<_> = value.iter().collect();
    ordered.sort();
    ordered.serialize(serializer)
}

pub fn is_false(v: &bool) -> bool {
    !v
}

pub fn is_empty_set<T>(v: &HashSet<T>) -> bool {
    v.is_empty()
}

pub const fn default_true() -> bool {
    true
}
