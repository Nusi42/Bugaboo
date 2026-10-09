//! Enums whose variants have a fixed word in files, in the API and in marks.
//!
//! The word of each variant is written next to it, once: `Identity => "identity"`. The code that
//! reads a word (`from_code`), writes it (`code`), lists all variants (`ALL`) and the serde form
//! come from that one line. `#[derive(Entity)]` looks words up through `from_code_const`, and
//! the tools read the enums themselves, so no second rule connects a word to its variant.
//!
//! The macros are exported (`bb_core::words!`, `bb_core::words_in_api!`), so every crate that has
//! a vocabulary of fixed words declares it the same way instead of writing the same code again
//! (`Topic`, `Counts`, `Kind`, `Permission`, `StartingRole` of `bb-policy`, `ErrorCode`).

use std::fmt;

// The macro writes its code at the place of use, where these names may mean something else or
// already be imported; it reaches them by this path.
#[doc(hidden)]
pub use ::serde as __serde;
#[doc(hidden)]
pub use std::fmt as __fmt;

/// What every enum declared with `words!` offers, so code that works for any vocabulary is written
/// once (the levels of `bb-policy` and the tables of `bb-store` are generic over it).
pub trait Word: Copy + fmt::Debug + std::hash::Hash + Ord + Send + Sync + 'static {
    /// Every variant, in the order they are declared.
    const ALL: &'static [Self];

    /// The word of this variant.
    fn code(self) -> &'static str;

    /// The variant with this word, if there is one.
    fn from_code(code: &str) -> Option<Self>;
}

/// The text is not one of the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownWord;

impl fmt::Display for UnknownWord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not one of the known words")
    }
}

impl std::error::Error for UnknownWord {}

/// Whether two texts are equal. `str` cannot be compared in a `const fn` yet, so the bytes are.
#[doc(hidden)]
#[must_use]
pub const fn same_text(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut at = 0;
    while at < a.len() {
        if a[at] != b[at] {
            return false;
        }
        at += 1;
    }
    true
}

/// Declares an enum with its words. A trailing `messages name = "prefix."` also gives the enum a
/// function `name` that returns the message of each word, `prefix.word` ([`crate::derived_key!`]),
/// so the key is not written a second time next to the word.
#[macro_export]
macro_rules! words {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$variant_meta:meta])* $variant:ident => $word:literal),+ $(,)?
        }
        messages $method:ident = $prefix:literal $(;)?
    ) => {
        $crate::words! {
            $(#[$meta])*
            $vis enum $name {
                $($(#[$variant_meta])* $variant => $word),+
            }
        }

        impl $name {
            /// The message that belongs to this word.
            #[must_use]
            $vis const fn $method(self) -> $crate::message::MessageKey {
                match self { $(Self::$variant => $crate::derived_key!($prefix, $word)),+ }
            }
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$variant_meta:meta])* $variant:ident => $word:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl $name {
            /// Every variant, in the order they are declared.
            $vis const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The word of this variant.
            #[must_use]
            $vis const fn code(self) -> &'static str {
                match self { $(Self::$variant => $word),+ }
            }

            /// The variant with this word, if there is one.
            #[must_use]
            $vis const fn from_code(code: &str) -> Option<Self> {
                $(
                    if $crate::words::same_text(code, $word) {
                        return Some(Self::$variant);
                    }
                )+
                None
            }

            /// Like `from_code`, for constants: an unknown word stops the build. Written by
            /// `#[derive(Entity)]`.
            #[doc(hidden)]
            #[must_use]
            $vis const fn from_code_const(code: &str) -> Self {
                let found = Self::from_code(code);
                assert!(
                    found.is_some(),
                    concat!("not a known ", stringify!($name), " word; see the list in bb_core")
                );
                match found {
                    Some(variant) => variant,
                    None => Self::ALL[0],
                }
            }
        }

        impl $crate::words::Word for $name {
            const ALL: &'static [Self] = <$name>::ALL;

            fn code(self) -> &'static str {
                <$name>::code(self)
            }

            fn from_code(code: &str) -> Option<Self> {
                <$name>::from_code(code)
            }
        }

        impl $crate::words::__fmt::Display for $name {
            fn fmt(&self, formatter: &mut $crate::words::__fmt::Formatter<'_>) -> $crate::words::__fmt::Result {
                formatter.write_str(self.code())
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::words::UnknownWord;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::from_code(text).ok_or($crate::words::UnknownWord)
            }
        }

        impl $crate::words::__serde::Serialize for $name {
            fn serialize<S: $crate::words::__serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.code())
            }
        }

        impl<'de> $crate::words::__serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::words::__serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
                text.parse().map_err($crate::words::__serde::de::Error::custom)
            }
        }

        // Words are fixed vocabulary, never personal data.
        impl $crate::personal::NotPersonal for $name {}
        impl $crate::plain::Plain for $name {}
    };
}

/// Describes a words enum in the API description as one of its words, so a client gets a union
/// type and the list of words is the one in the Rust enum (`words_in_api!(Field, "…")`).
#[macro_export]
macro_rules! words_in_api {
    ($name:ident, $description:literal) => {
        #[cfg(feature = "openapi")]
        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                use utoipa::openapi::schema::{ObjectBuilder, Type};
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some($description))
                    .enum_values(Some(Self::ALL.iter().map(|word| word.code())))
                    .into()
            }
        }

        #[cfg(feature = "openapi")]
        impl utoipa::ToSchema for $name {}
    };
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    words! {
        /// A sample.
        pub(crate) enum Sample {
            /// First.
            One => "one",
            /// Second.
            TwoWords => "two_words",
        }
    }

    #[test]
    fn the_word_and_the_variant_find_each_other() {
        for variant in Sample::ALL {
            assert_eq!(Sample::from_code(variant.code()), Some(*variant));
            assert_eq!(variant.code().parse(), Ok(*variant));
            assert_eq!(Sample::from_code_const(variant.code()), *variant);
        }
        assert_eq!(Sample::from_code("three"), None);
        assert_eq!(Sample::ALL.len(), 2);
    }

    #[test]
    fn words_are_written_in_json_and_checked_when_read() {
        assert_eq!(
            serde_json::to_string(&Sample::TwoWords).unwrap(),
            "\"two_words\""
        );
        assert_eq!(
            serde_json::from_str::<Sample>("\"one\"").unwrap(),
            Sample::One
        );
        assert!(serde_json::from_str::<Sample>("\"One\"").is_err());
    }

    #[test]
    fn real_vocabularies_have_unique_lower_case_words_and_complete_lists() {
        use crate::event::Channel;
        use crate::personal::{Basis, Category};
        let mut seen = HashSet::new();
        // The `match` in `code` has no fallback arm, so a variant without a word does not
        // compile; here the lists are checked for every variant being listed once.
        for code in Category::ALL
            .iter()
            .map(|v| v.code())
            .chain(Basis::ALL.iter().map(|v| v.code()))
            .chain(Channel::ALL.iter().map(|v| v.code()))
        {
            assert!(
                code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code}"
            );
            seen.insert(code);
        }
        assert_eq!(
            seen.len(),
            Category::ALL.len() + Basis::ALL.len() + Channel::ALL.len()
        );
    }
}
