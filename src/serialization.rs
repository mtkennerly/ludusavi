use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap, HashSet};

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
