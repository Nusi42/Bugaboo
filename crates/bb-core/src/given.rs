//! A member of a request that is left out, given a value, or given empty (`null`, which takes the
//! value away). The member is an `Option<SetTo<T>>`: `None` is left out (it stays as it is). Serde
//! reads a `null` as a missing value on its own, so such a member names this module:
//! `#[serde(default, skip_serializing_if = "Option::is_none", deserialize_with =
//! "bb_core::given::deserialize")]`. A request kept as JSON (the action of a run of many changes)
//! reads back the same way.

use serde::{Deserialize, Deserializer, Serialize};

/// What a request sets a member to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SetTo<T> {
    /// A value.
    Value(T),
    /// No value: `null`, which takes the value away.
    Nothing,
}

impl<T> SetTo<T> {
    /// The value it sets, none when it takes the value away.
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Nothing => None,
        }
    }
}

/// Reads a member that is there: `null` is given empty, anything else a value. A member that is
/// not there is never read, so `#[serde(default)]` makes it left out.
///
/// # Errors
/// When the value is not one of the type.
pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<SetTo<T>>, D::Error> {
    SetTo::<T>::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::SetTo;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Request {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::deserialize"
        )]
        length: Option<SetTo<u32>>,
    }

    #[test]
    fn a_member_is_left_out_given_empty_or_given_a_value_and_reads_back() {
        for (text, member) in [
            (json!({}), None),
            (json!({"length": null}), Some(SetTo::Nothing)),
            (json!({"length": 90}), Some(SetTo::Value(90))),
        ] {
            let read: Request = serde_json::from_value(text.clone()).unwrap();
            assert_eq!(read.length, member);
            assert_eq!(serde_json::to_value(&read).unwrap(), text);
        }
        assert!(serde_json::from_value::<Request>(json!({"length": -5})).is_err());
        assert_eq!(SetTo::Value(3).into_option(), Some(3));
        assert_eq!(SetTo::<u32>::Nothing.into_option(), None);
    }
}
