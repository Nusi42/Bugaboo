//! The text of the API description as the program writes it: a JSON document that keeps the order
//! of its keys, so the parts that utoipa keeps in a hash map (the `x-…` extensions) come in one
//! order every run, and the tidying of the finished document (`super::tidy`) can move parts into
//! `components` without moving anything else.

/// A JSON document that keeps the order of its keys as it was written. The extensions of
/// utoipa (`x-…`) sit in a hash map, so two runs of the program write them in two orders;
/// [`Ordered::steady`] puts them in the order of their names and touches nothing else.
pub(super) enum Ordered {
    Object(Vec<(String, Ordered)>),
    Array(Vec<Ordered>),
    Leaf(serde_json::Value),
}

impl Ordered {
    /// The extensions of every object in the order of their names, in the places where they are.
    pub(super) fn steady(&mut self) {
        match self {
            Self::Object(entries) => {
                let places: Vec<usize> = entries
                    .iter()
                    .enumerate()
                    .filter(|(_, (key, _))| key.starts_with("x-"))
                    .map(|(place, _)| place)
                    .collect();
                let mut extensions: Vec<(String, Ordered)> = places
                    .iter()
                    .map(|place| {
                        std::mem::replace(
                            &mut entries[*place],
                            (String::new(), Self::Array(Vec::new())),
                        )
                    })
                    .collect();
                extensions.sort_by(|a, b| a.0.cmp(&b.0));
                for (place, entry) in places.into_iter().zip(extensions) {
                    entries[place] = entry;
                }
                for (_, value) in entries {
                    value.steady();
                }
            }
            Self::Array(items) => items.iter_mut().for_each(Self::steady),
            Self::Leaf(_) => {}
        }
    }
}

impl<'de> serde::Deserialize<'de> for Ordered {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reader;

        impl<'de> serde::de::Visitor<'de> for Reader {
            type Value = Ordered;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Ordered, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Ordered>()? {
                    entries.push(entry);
                }
                Ok(Ordered::Object(entries))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Ordered, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<Ordered>()? {
                    items.push(item);
                }
                Ok(Ordered::Array(items))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(serde_json::Value::from(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_unit<E>(self) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(serde_json::Value::Null))
            }
        }

        deserializer.deserialize_any(Reader)
    }
}

impl serde::Serialize for Ordered {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap as _, SerializeSeq as _};
        match self {
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Leaf(value) => value.serialize(serializer),
        }
    }
}

/// The text of the document with the order of the extensions fixed.
#[cfg(test)]
pub(super) fn steady_text(text: &str) -> Option<String> {
    let mut document: Ordered = serde_json::from_str(text).ok()?;
    document.steady();
    serde_json::to_string_pretty(&document).ok()
}

impl Ordered {
    /// The member `key` of an object.
    pub(super) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// The member `key` of an object, to change.
    pub(super) fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        match self {
            Self::Object(entries) => entries
                .iter_mut()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Puts `value` under `key`: in the place of the member that is there, or at the end.
    pub(super) fn set(&mut self, key: &str, value: Self) {
        if let Self::Object(entries) = self {
            match entries.iter_mut().find(|(name, _)| name == key) {
                Some((_, slot)) => *slot = value,
                None => entries.push((key.to_owned(), value)),
            }
        }
    }

    /// The names of the members of an object, in order.
    pub(super) fn keys(&self) -> Vec<String> {
        match self {
            Self::Object(entries) => entries.iter().map(|(name, _)| name.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// The text of a leaf that is a string.
    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Self::Leaf(value) => value.as_str(),
            _ => None,
        }
    }

    /// A string.
    pub(super) fn string(text: &str) -> Self {
        Self::Leaf(serde_json::Value::String(text.to_owned()))
    }

    /// A reference to a part of the document: `{"$ref": "#/components/schemas/X"}`.
    pub(super) fn reference(path: &str) -> Self {
        Self::Object(vec![("$ref".to_owned(), Self::string(path))])
    }

    /// An object without members.
    pub(super) fn empty_object() -> Self {
        Self::Object(Vec::new())
    }

    /// Whether this is a reference.
    pub(super) fn is_reference(&self) -> bool {
        self.get("$ref").is_some()
    }

    /// The text the value is written as, to tell whether two values are the same.
    pub(super) fn written(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Visits this value and every object and array below it, to change them.
    pub(super) fn for_each_mut(&mut self, visit: &mut impl FnMut(&mut Self)) {
        visit(self);
        match self {
            Self::Object(entries) => {
                for (_, value) in entries {
                    value.for_each_mut(visit);
                }
            }
            Self::Array(items) => {
                for item in items {
                    item.for_each_mut(visit);
                }
            }
            Self::Leaf(_) => {}
        }
    }
}

/// The text of the finished document: the extensions in order, then the tidying (`super::tidy`).
pub(super) fn finished_text(text: &str) -> Option<String> {
    let mut document: Ordered = serde_json::from_str(text).ok()?;
    document.steady();
    super::tidy::tidy(&mut document);
    serde_json::to_string_pretty(&document).ok()
}
