//! How the pieces of a value are read and written: numbers, days, plain words, quoted text and the
//! shape of an Issue ID. The lexer reads with these, the printer writes with these, so a value
//! that is printed is read back as the same value.

use bb_core::project::ShortName;
use bb_core::readable_id::ReadableId;

use crate::syntax::{Date, Decimal};
use crate::words::{Keyword, continues_field, starts_direction};

/// The dash of a day (`2026-10-25`) and the sign of a negative number. The dash of an Issue ID is
/// `bb_core::readable_id::KEY_SEPARATOR`.
pub(crate) const DASH: char = DASH_BYTE as char;
/// The character that marks the start and the end of a quoted text, and the one that escapes it.
pub(crate) const QUOTE: char = '"';
pub(crate) const ESCAPE: char = '\\';
/// The most digits of a number. A decimal number of this size fits `i64` and a JavaScript number.
pub(crate) const MAX_DIGITS: usize = 15;
const DASH_BYTE: u8 = b'-';
/// The first and the last year of a day.
const FIRST_YEAR: u16 = 1;
const LAST_YEAR: u16 = 9999;

impl Decimal {
    /// The number a text spells (`12`, `-3`, `1.25`), or `None` for any other text or for too many
    /// digits.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (negative, digits) = text
            .strip_prefix(DASH)
            .map_or((false, text), |rest| (true, rest));
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        let fraction = fraction.trim_end_matches('0');
        let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if whole.is_empty()
            || !all_digits(whole)
            || !all_digits(fraction)
            || whole.len() + fraction.len() > MAX_DIGITS
            || (digits.contains('.') && digits.ends_with('.'))
        {
            return None;
        }
        let magnitude: i64 = format!("{whole}{fraction}").parse().ok()?;
        Some(Self {
            value: if negative { -magnitude } else { magnitude },
            scale: u8::try_from(fraction.len()).ok()?,
        })
    }

    /// The shortest text that spells the number.
    #[must_use]
    pub fn text(self) -> String {
        let sign = if self.value < 0 { "-" } else { "" };
        let digits = self.value.unsigned_abs().to_string();
        let scale = usize::from(self.scale);
        if scale == 0 {
            return format!("{sign}{digits}");
        }
        let padded = format!("{digits:0>width$}", width = scale + 1);
        let (whole, fraction) = padded.split_at(padded.len() - scale);
        format!("{sign}{whole}.{fraction}")
    }
}

impl Date {
    /// The day a text spells in the form `2026-10-25`, or `None` when it is not a day of the
    /// calendar.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let shaped = bytes.len() == 10
            && bytes[4] == DASH_BYTE
            && bytes[7] == DASH_BYTE
            && bytes
                .iter()
                .enumerate()
                .all(|(at, byte)| at == 4 || at == 7 || byte.is_ascii_digit());
        if !shaped {
            return None;
        }
        Self::new(
            text[..4].parse().ok()?,
            text[5..7].parse().ok()?,
            text[8..].parse().ok()?,
        )
    }

    /// The day, when the calendar has it.
    #[must_use]
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let last_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            _ => return None,
        };
        ((FIRST_YEAR..=LAST_YEAR).contains(&year) && (1..=last_day).contains(&day))
            .then_some(Self { year, month, day })
    }

    /// The day as `2026-10-25`.
    #[must_use]
    pub fn text(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The Issue ID that `text` spells, written the one way (in capitals), by the rule of `bb-core`
/// that the server reads the same ID with; `None` when it is not an Issue ID.
#[must_use]
pub(crate) fn issue_id(text: &str) -> Option<String> {
    text.parse::<ReadableId>().ok().map(|id| id.to_string())
}

/// The short name that `text` spells, written in any case and kept in capitals, by the rule of
/// `bb-core` that the server reads it with; `None` when it is not a short name.
#[must_use]
pub(crate) fn short_name(text: &str) -> Option<String> {
    text.parse::<ShortName>().ok().map(|name| name.to_string())
}

/// Whether `text` is one plain word that can stand as a value: it starts with a letter and goes on with
/// letters, digits and the underscore, and a word of the language only needs quotes when it would
/// be read as the language.
pub(crate) fn is_plain_value(text: &str) -> bool {
    let mut characters = text.chars();
    characters.next().is_some_and(char::is_alphabetic)
        && characters.all(|c| c.is_alphanumeric() || c == '_')
        && Keyword::of(text).is_none_or(|keyword| !keyword.reserved_as_value())
}

/// The text in quotes, with the quote and the escape character escaped.
#[must_use]
pub(crate) fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push(QUOTE);
    for character in text.chars() {
        if character == QUOTE || character == ESCAPE {
            out.push(ESCAPE);
        }
        out.push(character);
    }
    out.push(QUOTE);
    out
}

/// The word of a field, or of a key of `sort by`: bare when it is words and numbers one space apart
/// that the language reads back as the one name (`assigned to`, `found in release`), quoted
/// otherwise (ADR-0005 amendment M5, decision 11).
#[must_use]
pub(crate) fn field_or_quoted(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    let plain = words.iter().all(|word| {
        let mut characters = word.chars();
        characters.next().is_some_and(char::is_alphabetic)
            && characters.all(|c| c.is_alphanumeric() || c == '_')
    }) && words
        .first()
        .is_some_and(|first| Keyword::of(first).is_none_or(Keyword::can_open_a_field))
        && words
            .iter()
            .enumerate()
            .skip(1)
            .all(|(at, word)| continues_field(word, words.get(at + 1).copied()));
    if plain { text.to_owned() } else { quoted(text) }
}

/// The word of a field as a key of `sort by`: as [`field_or_quoted`], and quoted also when its first
/// words begin a direction (`highest`, `z to`), which the parser reads after a key as its direction
/// (ADR-0005 amendment M5, decision 11).
#[must_use]
pub(crate) fn key_or_quoted(text: &str) -> String {
    let mut words = text.split(' ');
    if words
        .next()
        .is_some_and(|first| starts_direction(first, words.next()))
    {
        quoted(text)
    } else {
        field_or_quoted(text)
    }
}

/// A value: bare when it is one plain word that the language does not read as itself, quoted
/// otherwise.
#[must_use]
pub(crate) fn value_or_quoted(text: &str) -> String {
    if is_plain_value(text) {
        text.to_owned()
    } else {
        quoted(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_keep_the_shortest_form() {
        for (text, value, scale, back) in [
            ("12", 12, 0, "12"),
            ("-3", -3, 0, "-3"),
            ("1.50", 15, 1, "1.5"),
            ("0.05", 5, 2, "0.05"),
            ("-0.5", -5, 1, "-0.5"),
            ("7.0", 7, 0, "7"),
            ("-0", 0, 0, "0"),
        ] {
            let number = Decimal::parse(text).unwrap();
            assert_eq!((number.value, number.scale), (value, scale), "{text}");
            assert_eq!(number.text(), back, "{text}");
        }
        for text in ["", "-", "1.", ".5", "1.2.3", "12a", "1234567890123456"] {
            assert_eq!(Decimal::parse(text), None, "{text}");
        }
    }

    #[test]
    fn a_day_must_be_in_the_calendar() {
        assert_eq!(
            Date::parse("2026-10-25").map(Date::text).unwrap(),
            "2026-10-25"
        );
        assert!(Date::parse("2024-02-29").is_some());
        assert!(Date::parse("2026-02-29").is_none());
        assert!(Date::parse("2026-13-01").is_none());
        assert!(Date::parse("2026-1-01").is_none());
        assert!(Date::parse("0000-01-01").is_none());
    }

    #[test]
    fn only_a_plain_word_goes_without_quotes() {
        assert_eq!(field_or_quoted("title"), "title");
        assert_eq!(field_or_quoted("week"), "week");
        assert_eq!(field_or_quoted("hours logged"), "hours logged");
        assert_eq!(field_or_quoted("and"), "\"and\"");
        assert_eq!(field_or_quoted("under"), "\"under\"");
        assert_eq!(field_or_quoted("404"), "\"404\"");
        assert_eq!(field_or_quoted(""), "\"\"");
        assert_eq!(field_or_quoted("say \"hi\"\\"), "\"say \\\"hi\\\"\\\\\"");
        assert_eq!(value_or_quoted("Done"), "Done");
        assert_eq!(value_or_quoted("In progress"), "\"In progress\"");
        assert_eq!(value_or_quoted("highest"), "highest");
        assert_eq!(value_or_quoted("Blocked"), "Blocked");
        assert_eq!(value_or_quoted("And"), "\"And\"");
        assert_eq!(value_or_quoted("this"), "\"this\"");
    }

    #[test]
    fn a_field_of_several_words_goes_without_quotes_when_it_reads_back_as_one_name() {
        assert_eq!(field_or_quoted("status group"), "status group");
        assert_eq!(field_or_quoted("assigned to"), "assigned to");
        assert_eq!(field_or_quoted("found in release"), "found in release");
        // A word that starts a test, joins or orders ends the name, so the name needs quotes.
        assert_eq!(field_or_quoted("a is b"), "\"a is b\"");
        assert_eq!(field_or_quoted("x in last"), "\"x in last\"");
        assert_eq!(field_or_quoted("x a to"), "\"x a to\"");
        assert_eq!(field_or_quoted("two  spaces"), "\"two  spaces\"");
        assert_eq!(field_or_quoted("not x"), "\"not x\"");
    }

    #[test]
    fn a_key_of_sort_by_that_begins_like_a_direction_is_quoted() {
        assert_eq!(key_or_quoted("status group"), "status group");
        assert_eq!(key_or_quoted("a"), "a");
        assert_eq!(key_or_quoted("to do"), "to do");
        assert_eq!(key_or_quoted("highest"), "\"highest\"");
        assert_eq!(key_or_quoted("Newest items"), "\"Newest items\"");
        assert_eq!(key_or_quoted("z to"), "\"z to\"");
        assert_eq!(key_or_quoted("A to Z"), "\"A to Z\"");
        assert_eq!(key_or_quoted("and"), "\"and\"");
        // As the name of a field in a filter, such a name reads back bare.
        assert_eq!(field_or_quoted("highest"), "highest");
    }

    #[test]
    fn the_shapes_of_ids_and_short_names() {
        assert_eq!(issue_id("WEB-142").as_deref(), Some("WEB-142"));
        assert_eq!(issue_id("WEB-K7Q2X").as_deref(), Some("WEB-K7Q2X"));
        // The server reads lower case too, and the tree holds the one way of writing it.
        assert_eq!(issue_id("web-142").as_deref(), Some("WEB-142"));
        // The rules are those of bb-core: 2 to 10 letters, and no I, L, O, U or leading zero.
        for not_an_id in [
            "WEB-",
            "WEB",
            "W-1",
            "ABCDEFGHIJK-1",
            "WEB-0142",
            "WEB-KILO1",
        ] {
            assert_eq!(issue_id(not_an_id), None, "{not_an_id}");
        }
        assert_eq!(short_name("web").as_deref(), Some("WEB"));
        assert_eq!(short_name("Web2").as_deref(), Some("WEB2"));
        assert_eq!(short_name("1web"), None);
        assert_eq!(short_name("w"), None);
    }
}
