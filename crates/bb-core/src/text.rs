//! The rule for the short texts people type: names of people, groups, roles, projects and access
//! keys, and the labels of passkeys. Defined once, so every use case that takes such a text keeps
//! it the same way. Also the alphabet of the codes that people read out (Crockford's base 32).

/// The longest name of a person, group, role, project or access key, in characters.
pub const NAME_MAX: usize = 100;

/// The longest label of a passkey, in characters.
pub const LABEL_MAX: usize = 64;

/// The longest title of an issue, in characters.
pub const TITLE_MAX: usize = 300;

/// The longest long text (the description of an issue), in characters.
pub const LONG_TEXT_MAX: usize = 100_000;

/// The 32 symbols of Crockford's base 32 in capitals: digits and letters without `I`, `L`, `O` and
/// `U`, so nothing is mistaken for something else when a person reads a code out.
pub const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The symbol of [`CROCKFORD`] for the lowest five bits of `byte`. A random byte gives every
/// symbol the same chance, because 256 is a multiple of 32.
#[must_use]
pub const fn crockford_symbol(byte: u8) -> char {
    CROCKFORD[(byte & 0b1_1111) as usize] as char
}

/// How often a length of a random code is tried again after a clash before the code gets longer.
pub const TRIES_PER_LENGTH: usize = 16;

/// A code of `length` symbols of [`CROCKFORD`] made from the random bytes (at most 16 symbols).
#[must_use]
pub fn crockford_code(random: [u8; 16], length: usize) -> String {
    random
        .iter()
        .take(length)
        .map(|byte| crockford_symbol(*byte))
        .collect()
}

/// The random codes people read out, one after the other: [`TRIES_PER_LENGTH`] codes of the
/// shortest length, then the same number one symbol longer, and so on up to the longest length.
/// The caller takes the first one that is free (a clash is not an error: the next draw is
/// another code), so a place that runs short of short codes moves on to longer ones. Used for
/// the labels of removed people and for the random issue IDs; the bytes come from the source
/// that `bb-core` is given, never from a clock or a counter (PRIV-13).
pub struct CodeDraws<'a, S> {
    source: &'a mut S,
    length: usize,
    longest: usize,
    tried: usize,
}

impl<'a, S: crate::id::IdSource> CodeDraws<'a, S> {
    /// The draws of codes from `shortest` to `longest` symbols (at most 16).
    pub fn new(source: &'a mut S, shortest: usize, longest: usize) -> Self {
        Self {
            source,
            length: shortest,
            longest: longest.min(16),
            tried: 0,
        }
    }
}

impl<S: crate::id::IdSource> Iterator for CodeDraws<'_, S> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.length > self.longest {
            return None;
        }
        let code = crockford_code(self.source.random(), self.length);
        self.tried += 1;
        if self.tried == TRIES_PER_LENGTH {
            self.tried = 0;
            self.length += 1;
        }
        Some(code)
    }
}

/// The text as it is kept: trimmed, not empty, at most `longest` characters and without control
/// characters. `None` for a text that breaks one of these.
#[must_use]
pub fn clean(text: &str, longest: usize) -> Option<&str> {
    let text = text.trim();
    let fine =
        !text.is_empty() && text.chars().count() <= longest && !text.chars().any(char::is_control);
    fine.then_some(text)
}

/// A long text as it is kept: at most `longest` characters, line breaks and tabs allowed, no other
/// control characters. It may be empty and is not trimmed (a person's indentation is theirs).
/// `None` for a text that breaks one of these.
#[must_use]
pub fn clean_long(text: &str, longest: usize) -> Option<&str> {
    let fine = text.chars().count() <= longest
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'));
    fine.then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_trimmed_and_kept_when_it_is_plain_and_not_too_long() {
        assert_eq!(clean("  Ada Lovelace ", NAME_MAX), Some("Ada Lovelace"));
        assert_eq!(
            clean(&"é".repeat(NAME_MAX), NAME_MAX).map(str::len),
            Some(2 * NAME_MAX)
        );
    }

    #[test]
    fn an_empty_long_or_control_text_is_refused() {
        assert_eq!(clean("", NAME_MAX), None);
        assert_eq!(clean("   ", NAME_MAX), None);
        assert_eq!(clean(&"a".repeat(NAME_MAX + 1), NAME_MAX), None);
        assert_eq!(clean("two\nlines", NAME_MAX), None);
        assert_eq!(clean("tab\tinside", LABEL_MAX), None);
        assert_eq!(clean(&"a".repeat(LABEL_MAX + 1), LABEL_MAX), None);
    }

    #[test]
    fn a_long_text_may_hold_line_breaks_and_be_empty_but_not_other_control_characters() {
        assert_eq!(clean_long("", LONG_TEXT_MAX), Some(""));
        assert_eq!(clean_long("a\r\n\tb", LONG_TEXT_MAX), Some("a\r\n\tb"));
        assert_eq!(clean_long("a\u{0}b", LONG_TEXT_MAX), None);
        assert_eq!(clean_long("a\u{1b}b", LONG_TEXT_MAX), None);
        assert_eq!(clean_long(&"a".repeat(11), 10), None);
    }

    #[test]
    fn the_symbols_are_the_32_of_crockford_and_every_byte_has_one() {
        assert_eq!(CROCKFORD.len(), 32);
        assert!(CROCKFORD.iter().all(|symbol| !b"ILOU".contains(symbol)));
        assert_eq!(crockford_symbol(0), '0');
        assert_eq!(crockford_symbol(31), 'Z');
        assert_eq!(crockford_symbol(32), '0');
        assert_eq!(crockford_symbol(255), 'Z');
    }
}
