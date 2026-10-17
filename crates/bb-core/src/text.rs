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

/// Whether the text holds a character that takes no room on the screen or turns the text round: a
/// zero-width space, a word joiner, a soft hyphen, a byte order mark, a mark of direction or an
/// override of it (Unicode format characters). Two names that look alike but differ in one of these
/// are a trap (a tag that cannot be told from another), and an override can turn a name round. The
/// zero-width joiner and non-joiner stay allowed: emoji sequences and several scripts need them.
#[must_use]
pub fn has_invisible_format(text: &str) -> bool {
    text.chars().any(is_invisible_format)
}

/// Whether the character is one of those [`has_invisible_format`] looks for.
const fn is_invisible_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
    )
}

/// The longest subject of an e-mail, in characters.
pub const MAIL_SUBJECT_MAX: usize = 200;

/// Text that goes out as plain text, where nothing may turn it round or hide in it (an e-mail): the
/// one cleaner for every text of a person that is put into one. Control characters become a space
/// (a line break stays a line break with `keep_lines`, a carriage return goes), every format
/// character goes (the marks of direction and their overrides, the zero-width characters, the
/// joiners), and what is longer than `longest` characters is cut and ends in `…`.
fn plain(text: &str, longest: usize, keep_lines: bool) -> String {
    let mut cleaned = String::new();
    for (count, c) in text
        .chars()
        .filter(|c| !is_invisible_format(*c) && !matches!(c, '\u{200C}' | '\u{200D}' | '\r'))
        .enumerate()
    {
        if count == longest {
            cleaned.push('…');
            break;
        }
        if c == '\n' && keep_lines {
            cleaned.push('\n');
        } else if c.is_control() {
            cleaned.push(' ');
        } else {
            cleaned.push(c);
        }
    }
    cleaned
}

/// Text of a person for an e-mail, line breaks kept: see [`plain`].
#[must_use]
pub fn plain_text(text: &str, longest: usize) -> String {
    plain(text, longest, true)
}

/// Text of a person for a place of one line (a subject, a name in a sentence): the same, with line
/// breaks as spaces, so it cannot add a header or a line, and trimmed.
#[must_use]
pub fn plain_line(text: &str, longest: usize) -> String {
    plain(text, longest, false).trim().to_owned()
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
    fn text_for_an_e_mail_loses_what_turns_it_round_or_hides_and_is_cut() {
        // A mark of direction, a zero-width space and a joiner go; a line break stays or not.
        let tricky = "pay\u{202E}gnp.\u{200B}now\u{200D} ok\r\nnext\tline";
        assert_eq!(plain_text(tricky, 100), "paygnp.now ok\nnext line");
        assert_eq!(plain_line(tricky, 100), "paygnp.now ok next line");
        assert_eq!(
            plain_line("  Subject\nBcc: x@y.z ", 100),
            "Subject Bcc: x@y.z"
        );
        // Cut to the length, with a mark that it was cut.
        assert_eq!(plain_text("abcdef", 3), "abc…");
        assert_eq!(plain_text("abc", 3), "abc");
        assert_eq!(plain_line("", 3), "");
    }

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
    fn invisible_and_turning_characters_are_found_but_joiners_are_allowed() {
        for found in [
            "a\u{200B}b",
            "\u{202E}evil",
            "a\u{FEFF}",
            "x\u{2066}y",
            "s\u{00AD}hy",
        ] {
            assert!(has_invisible_format(found), "{found:?}");
        }
        for plain in ["tag", "Größe", "👨\u{200D}👩", "می\u{200C}خواهم", "日本語"] {
            assert!(!has_invisible_format(plain), "{plain:?}");
        }
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
