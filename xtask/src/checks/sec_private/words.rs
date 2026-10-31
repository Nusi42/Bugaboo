//! The entries of the private list as patterns, and the GitHub forms in which a login is public.

use super::Span;

/// The characters that may stand between the words of an entry with several words.
const GAP_CHARACTERS: [char; 3] = ['_', '-', '.'];
/// The characters of the part of an e-mail address before the `@`, besides letters and digits.
const EMAIL_LOCAL_CHARACTERS: [char; 5] = ['.', '_', '%', '+', '-'];
/// The domain of the noreply addresses of GitHub.
const NOREPLY_DOMAIN: &str = "@users.noreply.github.com";
/// The starts of a link to an account or a repository of GitHub (the web and the SSH form).
const GITHUB_LINKS: [&str; 2] = ["github.com/", "github.com:"];

/// A character of a lower-case text with the bytes of the character it came from.
pub(super) type Lowered = (char, usize, usize);

/// A piece of an entry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Character(char),
    /// White space between the words of an entry: any run of spaces, `_`, `-` and `.` matches.
    Gap,
}

/// What an entry is, which decides where it ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// One or more words: a letter before or after continues the word.
    Words,
    /// An e-mail address (the entry has an `@`): it is not the end of a longer address.
    Email,
}

/// One entry of the private list, ready to be found in a text.
pub(super) struct Pattern {
    pieces: Vec<Piece>,
    kind: Kind,
}

fn is_gap(c: char) -> bool {
    c.is_whitespace() || GAP_CHARACTERS.contains(&c)
}

impl Pattern {
    /// The pattern of an entry in lower case, without white space at either end.
    pub(super) fn new(entry: &str) -> Self {
        let mut pieces = Vec::new();
        for c in entry.chars() {
            if !c.is_whitespace() {
                pieces.push(Piece::Character(c));
            } else if pieces.last() != Some(&Piece::Gap) {
                pieces.push(Piece::Gap);
            }
        }
        let kind = if entry.contains('@') {
            Kind::Email
        } else {
            Kind::Words
        };
        Self { pieces, kind }
    }

    /// Where a match that starts at `from` ends (the index after its last character).
    fn end_from(&self, text: &[Lowered], from: usize) -> Option<usize> {
        let mut at = from;
        for piece in &self.pieces {
            match piece {
                Piece::Character(c) => {
                    if text.get(at).map(|found| found.0) != Some(*c) {
                        return None;
                    }
                    at += 1;
                }
                Piece::Gap => {
                    let start = at;
                    while text.get(at).is_some_and(|found| is_gap(found.0)) {
                        at += 1;
                    }
                    if at == start {
                        return None;
                    }
                }
            }
        }
        Some(at)
    }

    /// Whether the match from `from` to `end` is a whole word (or a whole address).
    fn is_whole(&self, text: &[Lowered], from: usize, end: usize) -> bool {
        let before = from.checked_sub(1).map(|at| text[at].0);
        let after = text.get(end).map(|next| next.0);
        match self.kind {
            Kind::Words => {
                !before.is_some_and(|c| c.is_ascii_alphabetic())
                    && !after.is_some_and(|c| c.is_ascii_alphabetic())
            }
            Kind::Email => {
                // An entry that starts with `@` is a domain: the address in front of it is part
                // of what is found.
                let starts_domain = self.pieces.first() == Some(&Piece::Character('@'));
                let continued_before = before
                    .is_some_and(|c| c.is_alphanumeric() || EMAIL_LOCAL_CHARACTERS.contains(&c));
                (starts_domain || !continued_before) && !after.is_some_and(char::is_alphanumeric)
            }
        }
    }

    /// Where this entry stands in the text: the bytes from and to of every match.
    pub(super) fn spans(&self, text: &[Lowered]) -> Vec<Span> {
        (0..text.len())
            .filter_map(|from| {
                let end = self.end_from(text, from)?;
                (end > from && self.is_whole(text, from, end))
                    .then(|| (text[from].1, text[end - 1].2))
            })
            .collect()
    }
}

/// Where a login stands in a GitHub form, in the text: the login of `github.com/<login>` and a
/// noreply address as a whole. A private word inside one of these is public by nature.
pub(super) fn github_forms(text: &str) -> Vec<Span> {
    let lower = text.to_ascii_lowercase();
    let login = |c: char| c.is_ascii_alphanumeric() || c == '-';
    let mut spans = Vec::new();
    for link in GITHUB_LINKS {
        for (at, _) in lower.match_indices(link) {
            let start = at + link.len();
            let length = lower[start..]
                .find(|c: char| !login(c))
                .unwrap_or(lower.len() - start);
            if length > 0 {
                spans.push((start, start + length));
            }
        }
    }
    for (at, _) in lower.match_indices(NOREPLY_DOMAIN) {
        let start = lower[..at]
            .rfind(|c: char| !(login(c) || c == '+'))
            .map_or(0, |found| found + 1);
        if start < at {
            spans.push((start, at + NOREPLY_DOMAIN.len()));
        }
    }
    spans
}
