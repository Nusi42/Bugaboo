//! The readable issue ID (ISS-01a, ADR-0021): the short name of the project, a dash and a code that
//! is either a counting number (`WEB-142`) or a random code (`WEB-K7Q2X`).
//!
//! Every issue also keeps its internal ID (`IssueId`). The readable ID is what people say and
//! type; it can change (the mode of the project changes, the issue moves), and the old ones keep
//! working through the aliases of the store. This file holds the rules that need no I/O: what a
//! code looks like, how a key is read, and how a code is made from numbers and random bytes the
//! caller passes in.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::project::ShortName;
use crate::settings::ISSUES_ID_LENGTH;
use crate::text::CROCKFORD;

/// The most digits of a counting number, so it always fits a signed 64-bit number.
const COUNT_DIGITS_MAX: usize = 18;
/// What separates the short name from the code.
pub const KEY_SEPARATOR: char = '-';

words! {
    /// How a project gives out the codes of its issue IDs (ISS-01a).
    pub enum IdMode {
        /// *Random IDs*: `WEB-K7Q2X`, which reveal nothing about how many issues there are.
        Random => "random",
        /// *Counting IDs*: `WEB-142`, easy to say aloud.
        Counting => "counting",
    }
    messages name = "issue-id-mode."
}

words_in_api!(IdMode, "How a project gives out issue IDs.");

/// The text is not the code of an issue ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidIssueCode;

impl fmt::Display for InvalidIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "an issue code is a counting number or {} to {} capital letters and digits without I, L, O and U",
            shortest_code(),
            longest_code()
        )
    }
}

impl std::error::Error for InvalidIssueCode {}

/// The part of an issue ID after the dash: a counting number (`142`, no leading zero) or a random
/// code of 4 to 8 symbols of Crockford's base 32 (`K7Q2X`). It holds nothing but those symbols,
/// so it can never carry personal data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IssueCode(String);

fn is_counting(text: &str) -> bool {
    (1..=COUNT_DIGITS_MAX).contains(&text.len())
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && !text.starts_with('0')
}

/// A number of symbols as the length of a random code: the least or the most when `wanted` is out
/// of the bounds of the setting *Length of random issue IDs*, which are the one definition of
/// them.
#[must_use]
pub fn code_length(wanted: i64) -> usize {
    usize::try_from(ISSUES_ID_LENGTH.definition().clamp(wanted)).unwrap_or_default()
}

/// The fewest symbols of a random code.
#[must_use]
pub fn shortest_code() -> usize {
    code_length(i64::MIN)
}

/// The most symbols of a random code.
#[must_use]
pub fn longest_code() -> usize {
    code_length(i64::MAX)
}

fn is_random(text: &str) -> bool {
    (shortest_code()..=longest_code()).contains(&text.len())
        && text.bytes().all(|byte| CROCKFORD.contains(&byte))
}

impl IssueCode {
    /// The code `text`, as stored or as typed in capitals.
    ///
    /// # Errors
    /// Fails unless `text` is a counting number or a random code.
    pub fn new(text: &str) -> Result<Self, InvalidIssueCode> {
        if is_counting(text) || is_random(text) {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidIssueCode)
        }
    }

    /// The counting code of the number `number` (which starts at 1).
    #[must_use]
    pub fn counting(number: i64) -> Self {
        Self(number.max(1).to_string())
    }

    /// A random code made of `symbols` (from `bb_core::text::CodeDraws`): `None` when it is not
    /// one of the lengths of a random code.
    #[must_use]
    pub fn random(symbols: &str) -> Option<Self> {
        is_random(symbols).then(|| Self(symbols.to_owned()))
    }

    /// The code as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IssueCode {
    type Error = InvalidIssueCode;

    fn try_from(text: String) -> Result<Self, InvalidIssueCode> {
        Self::new(&text)
    }
}

impl From<IssueCode> for String {
    fn from(code: IssueCode) -> Self {
        code.0
    }
}

impl fmt::Display for IssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

// Only digits and the letters of the alphabet: nothing a person could write about themselves.
impl crate::personal::NotPersonal for IssueCode {}

/// The text is not an issue ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidIssueKey;

impl fmt::Display for InvalidIssueKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an issue ID is a short name, a dash and a code, like WEB-142")
    }
}

impl std::error::Error for InvalidIssueKey {}

/// A readable issue ID, as written (`WEB-142`): the short name of a project and a code. It may be
/// an old one: the store finds the issue that has it now.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssueKey {
    /// The project's short name (an old one counts: it stays with the project).
    pub short_name: ShortName,
    /// The code.
    pub code: IssueCode,
}

impl IssueKey {
    /// The key of `code` in the project that has `short_name`.
    #[must_use]
    pub const fn new(short_name: ShortName, code: IssueCode) -> Self {
        Self { short_name, code }
    }
}

impl FromStr for IssueKey {
    type Err = InvalidIssueKey;

    /// Reads `WEB-142` and `WEB-K7Q2X`, also in lower case.
    fn from_str(text: &str) -> Result<Self, InvalidIssueKey> {
        let (short_name, code) = text.split_once(KEY_SEPARATOR).ok_or(InvalidIssueKey)?;
        let capitals = text.to_ascii_uppercase();
        let (short_name_capitals, code_capitals) = (
            &capitals[..short_name.len()],
            &capitals[short_name.len() + KEY_SEPARATOR.len_utf8()..],
        );
        debug_assert_eq!(code.len(), code_capitals.len());
        Ok(Self {
            short_name: ShortName::new(short_name_capitals).map_err(|_| InvalidIssueKey)?,
            code: IssueCode::new(code_capitals).map_err(|_| InvalidIssueKey)?,
        })
    }
}

impl fmt::Display for IssueKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{KEY_SEPARATOR}{}", self.short_name, self.code)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use jiff::Timestamp;
    use proptest::prelude::*;

    use super::*;
    use crate::id::{IdSource, SequentialIds};
    use crate::text::CodeDraws;

    /// A source of random bytes that hands out the blocks it was given, over and over: with few
    /// different blocks, the draws clash a lot.
    struct Blocks {
        blocks: Vec<[u8; 16]>,
        at: usize,
    }

    impl IdSource for Blocks {
        fn now(&mut self) -> Timestamp {
            Timestamp::UNIX_EPOCH
        }

        fn random(&mut self) -> [u8; 16] {
            let block = self.blocks[self.at % self.blocks.len()];
            self.at += 1;
            block
        }
    }

    proptest! {
        #[test]
        fn iss_01a_random_ids_use_the_readable_alphabet_and_never_repeat(
            blocks in prop::collection::vec(prop::array::uniform16(0_u8..6), 1..40),
            wanted in 1_usize..30,
        ) {
            let mut source = Blocks { blocks, at: 0 };
            let mut given: HashSet<String> = HashSet::new();
            for _ in 0..wanted {
                // What a project does for every new issue: the first draw it has not given out.
                let drawn = CodeDraws::new(&mut source, shortest_code(), longest_code())
                    .find(|symbols| !given.contains(symbols));
                // Draws that all clash give no code at all, never a code twice.
                let Some(symbols) = drawn else { break };
                let code = IssueCode::random(&symbols).expect("a drawn code is a random code");
                prop_assert!(symbols.bytes().all(|symbol| CROCKFORD.contains(&symbol)));
                prop_assert!((shortest_code()..=longest_code()).contains(&symbols.len()));
                prop_assert!(!symbols.contains(['I', 'L', 'O', 'U']));
                prop_assert!(given.insert(code.to_string()), "{code} was given out twice");
            }
        }
    }

    #[test]
    fn iss_01a_a_code_is_a_counting_number_or_a_readable_random_code() {
        for good in [
            "1",
            "142",
            "999999999999999999",
            "K7Q2",
            "K7Q2X",
            "ABCDEFGH",
            "0123",
        ] {
            assert!(IssueCode::new(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "0",
            "012",
            "1000000000000000000",
            "K7Q",
            "K7Q2XABCD",
            "k7q2x",
            "K7I2X",
            "K7L2X",
            "K7O2X",
            "K7U2X",
            "K7 2X",
            "K7-2X",
            "ÄÄÄÄ",
        ] {
            assert_eq!(IssueCode::new(bad), Err(InvalidIssueCode), "{bad:?}");
        }
        assert_eq!(IssueCode::counting(142).as_str(), "142");
    }

    #[test]
    fn iss_01a_a_key_is_read_in_both_forms_and_written_the_same() {
        for text in ["WEB-142", "WEB-K7Q2X", "AB-1", "APP2-ZZZZ"] {
            let key: IssueKey = text.parse().unwrap();
            assert_eq!(key.to_string(), text);
        }
        let lower: IssueKey = "web-k7q2x".parse().unwrap();
        assert_eq!(lower.to_string(), "WEB-K7Q2X");
        for bad in [
            "", "WEB", "WEB-", "-142", "W-1", "WEB--1", "WEB-0", "WEB-K7", "WEB 142", "WEB-1-2",
        ] {
            assert_eq!(bad.parse::<IssueKey>(), Err(InvalidIssueKey), "{bad:?}");
        }
    }

    #[test]
    fn iss_01a_a_code_survives_json_and_is_checked_when_read() {
        let code = IssueCode::new("K7Q2X").unwrap();
        assert_eq!(serde_json::to_string(&code).unwrap(), "\"K7Q2X\"");
        assert_eq!(
            serde_json::from_str::<IssueCode>("\"K7Q2X\"").unwrap(),
            code
        );
        assert!(serde_json::from_str::<IssueCode>("\"k7q2x\"").is_err());
    }

    #[test]
    fn iss_01a_random_draws_make_readable_codes_of_the_asked_length_and_then_longer_ones() {
        let mut source = SequentialIds::default();
        let draws: Vec<String> = CodeDraws::new(&mut source, 4, 6).collect();
        assert_eq!((shortest_code(), longest_code()), (4, 8));
        assert_eq!(code_length(0), 4);
        assert_eq!(code_length(99), 8);
        assert_eq!(draws.len(), 3 * crate::text::TRIES_PER_LENGTH);
        for (at, code) in draws.iter().enumerate() {
            assert_eq!(code.len(), 4 + at / crate::text::TRIES_PER_LENGTH);
            assert!(IssueCode::random(code).is_some(), "{code}");
        }
    }
}
