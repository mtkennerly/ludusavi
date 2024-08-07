use std::collections::{BTreeMap, HashMap};

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

pub const fn default_true() -> bool {
    true
}
