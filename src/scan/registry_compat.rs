#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegistryItem {
    raw: String,
}

impl RegistryItem {
    pub fn new(raw: String) -> Self {
        Self { raw }
    }

    #[allow(unused)]
    pub fn from_hive_and_key(hive: &str, key: &str) -> Self {
        Self {
            raw: format!("{}/{}", hive, key).replace('\\', "/"),
        }
    }

    pub fn reset(&mut self, raw: String) {
        self.raw = raw;
    }

    pub fn raw(&self) -> String {
        self.raw.to_string()
    }

    pub fn render(&self) -> String {
        self.raw.replace('\\', "/")
    }

    #[allow(dead_code)]
    pub fn rendered(&self) -> Self {
        Self { raw: self.render() }
    }

    pub fn interpret(&self) -> String {
        self.raw.replace('/', "\\")
    }

    #[allow(dead_code)]
    pub fn interpreted(&self) -> Self {
        Self { raw: self.interpret() }
    }

    pub fn split(&self) -> Vec<String> {
        self.interpret().split('\\').map(|x| x.to_string()).collect()
    }

    #[allow(dead_code)]
    pub fn split_hive(&self) -> Option<(String, String)> {
        let interpreted = self.interpret();
        let parts: Vec<_> = interpreted.splitn(2, '\\').collect();
        (parts.len() == 2).then(|| (parts[0].to_string(), parts[1].to_string()))
    }

    pub fn is_prefix_of(&self, other: &Self) -> bool {
        let us_components = self.split();
        let them_components = other.split();

        if us_components.len() >= them_components.len() {
            return false;
        }
        us_components
            .iter()
            .zip(them_components.iter())
            .all(|(us, them)| us == them)
    }

    pub fn nearest_prefix(&self, others: Vec<Self>) -> Option<Self> {
        let us_components = self.split();
        let us_count = us_components.len();

        let mut nearest = None;
        let mut nearest_len = 0;
        for other in others {
            let them_components = other.split();
            let them_len = them_components.len();

            if us_count <= them_len {
                continue;
            }
            if us_components
                .iter()
                .clone()
                .zip(them_components.iter())
                .all(|(us, them)| us == them)
                && them_len > nearest_len
            {
                nearest = Some(other.clone());
                nearest_len = them_len;
            }
        }
        nearest
    }
}

// Based on:
// https://github.com/serde-rs/serde/issues/751#issuecomment-277580700
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RegistryItemSerdeHelper(String);

impl serde::Serialize for RegistryItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RegistryItemSerdeHelper(self.raw()).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for RegistryItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer).map(|RegistryItemSerdeHelper(raw)| RegistryItem { raw })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::testing::s;

    #[test]
    fn is_prefix_of() {
        assert!(RegistryItem::new(s(r#"HKCU"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
        assert!(RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo/bar"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/f"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/bar"))));
        assert!(!RegistryItem::new(s(r#""#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
    }

    #[test]
    fn nearest_prefix() {
        assert_eq!(
            Some(RegistryItem::new(s(r#"HKCU/foo/bar"#))),
            RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)).nearest_prefix(vec![
                RegistryItem::new(s(r#"HKCU/foo"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)),
            ])
        );
        assert_eq!(
            None,
            RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)).nearest_prefix(vec![
                RegistryItem::new(s(r#"HKCU/fo"#)),
                RegistryItem::new(s(r#"HKCU/fooo"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)),
            ])
        );
    }
}
