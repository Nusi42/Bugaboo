//! Issue IDs and mentions in a text (ADR-0021, M3-Q7): where they are, never what they mean.
//! Whether a person may see the issue or be mentioned is for the server to decide.

use std::ops::RangeInclusive;

use crate::MAX_MENTION_CHARS;

/// What kind of token it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// An issue ID: `WEB-142` (counting) or `WEB-K7Q2X` (random).
    IssueId,
    /// A mention: `@` and a name.
    Mention,
}

impl TokenKind {
    /// The word the HTML carries for this kind (`data-kind`).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::IssueId => "issue-id",
            Self::Mention => "mention",
        }
    }
}

/// A token and its place in the text that was rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: TokenKind,
    /// Where it starts: a byte offset into the text, at a character.
    pub start: usize,
    /// Where it ends (after its last byte). The token is `text[start..end]`.
    pub end: usize,
}

/// The length of the short name at the start of an issue ID: 2 to 10 capital letters or digits,
/// starting with a letter (glossary: *Short name*).
const SHORT_NAME_LENGTH: RangeInclusive<usize> = 2..=10;
/// The length of a random code (ADR-0021: 4 to 8 characters).
const RANDOM_CODE_LENGTH: RangeInclusive<usize> = 4..=8;
/// The most digits of a counting number.
const MAX_COUNT_DIGITS: usize = 9;
/// The letters a random code never uses, because they look like other characters (ADR-0021).
const LOOK_ALIKE_LETTERS: [char; 4] = ['I', 'L', 'O', 'U'];
/// The mark of an issue ID between the short name and the number or code.
const ID_SEPARATOR: char = '-';
/// The mark of a mention.
const MENTION_MARK: char = '@';
/// Characters, besides letters and digits, that may be inside a name after `@`.
const MENTION_INNER: [char; 3] = ['_', '.', '-'];
/// Characters, besides letters and digits, that a token must not follow (a path, an address).
const NOT_BEFORE_A_TOKEN: [char; 7] = ['_', '@', '.', '/', ':', '-', '\\'];

/// The tokens of `text`, in order, never overlapping, at most `limit` of them. `base` is added to
/// every place, so a piece of a bigger text can be searched.
pub(crate) fn find(text: &str, base: usize, limit: usize) -> Vec<Token> {
    let mut found = Vec::new();
    let mut at = 0;
    while found.len() < limit
        && let Some(next) = text[at..].chars().next()
    {
        let before = text[..at].chars().next_back();
        let matched = if next == MENTION_MARK && starts_a_word(before) {
            mention_length(&text[at..]).map(|length| (TokenKind::Mention, length))
        } else if next.is_ascii_uppercase() && starts_a_word(before) {
            issue_id_length(&text[at..]).map(|length| (TokenKind::IssueId, length))
        } else {
            None
        };
        if let Some((kind, length)) = matched {
            found.push(Token {
                kind,
                start: base + at,
                end: base + at + length,
            });
            at += length;
        } else {
            at += next.len_utf8();
        }
    }
    found
}

/// A token may only start where a word starts: not inside `ab@cd` (an e-mail address), not in
/// `xWEB-1`, not after a path or a dot.
fn starts_a_word(before: Option<char>) -> bool {
    before.is_none_or(|c| !(c.is_alphanumeric() || NOT_BEFORE_A_TOKEN.contains(&c)))
}

fn issue_id_length(text: &str) -> Option<usize> {
    let name_end = text
        .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit()))
        .unwrap_or(text.len());
    if !SHORT_NAME_LENGTH.contains(&name_end) {
        return None;
    }
    let after_name = text[name_end..].strip_prefix(ID_SEPARATOR)?;
    let code_end = after_name
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(after_name.len());
    let code = &after_name[..code_end];
    let counting = !code.is_empty()
        && code.len() <= MAX_COUNT_DIGITS
        && code.bytes().all(|b| b.is_ascii_digit());
    let random = RANDOM_CODE_LENGTH.contains(&code.len())
        && code.chars().all(|c| {
            (c.is_ascii_uppercase() || c.is_ascii_digit()) && !LOOK_ALIKE_LETTERS.contains(&c)
        });
    let rest = &after_name[code_end..];
    let continues = rest.starts_with('_')
        || rest
            .strip_prefix(ID_SEPARATOR)
            .is_some_and(|more| more.starts_with(char::is_alphanumeric));
    (!continues && (counting || random)).then_some(name_end + ID_SEPARATOR.len_utf8() + code_end)
}

fn mention_length(text: &str) -> Option<usize> {
    let name = &text[MENTION_MARK.len_utf8()..];
    let mut end = 0;
    for (count, (at, c)) in name.char_indices().enumerate() {
        if count == MAX_MENTION_CHARS || !(c.is_alphanumeric() || MENTION_INNER.contains(&c)) {
            break;
        }
        end = at + c.len_utf8();
    }
    let trimmed = name[..end].trim_end_matches(['.', '-']);
    let followed_by_more = name[trimmed.len()..].starts_with(MENTION_MARK);
    (!trimmed.is_empty() && !followed_by_more).then_some(MENTION_MARK.len_utf8() + trimmed.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<(&'static str, &str)> {
        find(text, 0, usize::MAX)
            .iter()
            .map(|t| (t.kind.word(), &text[t.start..t.end]))
            .collect()
    }

    #[test]
    fn counting_and_random_ids_are_found() {
        assert_eq!(
            words("see WEB-142, and (WEB-K7Q2X)."),
            [("issue-id", "WEB-142"), ("issue-id", "WEB-K7Q2X")]
        );
    }

    #[test]
    fn things_that_look_like_ids_but_are_not_are_left_alone() {
        for text in [
            "xWEB-142",
            "web-142",
            "W-1",
            "WEB-",
            "WEB-12ab",
            "WEBWEBWEBWEB-1",
            "WEB-142_x",
            "WEB-142-2",
            "WEB-ILOU9",
            "https://example.com/WEB-142",
            "WEB-1234567890",
        ] {
            assert!(words(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn mentions_are_found_but_not_in_addresses() {
        assert_eq!(
            words("hi @ana.lopez, @Bo_2 and a@b.com @ @"),
            [("mention", "@ana.lopez"), ("mention", "@Bo_2")]
        );
    }

    #[test]
    fn a_mention_stops_at_the_limit() {
        let long = format!("@{}", "a".repeat(MAX_MENTION_CHARS + 10));
        let found = find(&long, 0, usize::MAX);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].end, 1 + MAX_MENTION_CHARS);
    }

    #[test]
    fn places_are_shifted_by_the_base() {
        let found = find("a WEB-1", 10, usize::MAX);
        assert_eq!((found[0].start, found[0].end), (12, 17));
    }
}
