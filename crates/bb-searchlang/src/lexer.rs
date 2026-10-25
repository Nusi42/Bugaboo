//! The lexer: the text of a search as words, numbers, quoted texts and signs, each with the bytes
//! it covers. It never fails: a piece that means nothing becomes a token of its own kind, and the
//! parser says what is wrong with it.

use crate::literal::{ESCAPE, QUOTE, is_readable_id};
use crate::span::Span;
use crate::syntax::{Date, Decimal};

/// One piece of the text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

/// What a piece is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokenKind {
    /// A word: letters, digits and the underscore, starting with a letter.
    Word(String),
    /// A text in quotes. `closed` is false when the text ends before the closing quote.
    Quoted {
        text: String,
        closed: bool,
    },
    /// A number.
    Number(Decimal),
    /// A day.
    Date(Date),
    /// An Issue ID.
    Id(String),
    /// Something that looks like a day (`2026-13-40`) but is not one.
    NotADate(String),
    /// A piece that is nothing the language knows.
    Unreadable(String),
    OpenBracket,
    CloseBracket,
    Comma,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
}

impl TokenKind {
    /// Whether the token is a run of letters and digits that the person may still be typing.
    pub(crate) const fn is_chunk(&self) -> bool {
        matches!(
            self,
            Self::Word(_)
                | Self::Number(_)
                | Self::Date(_)
                | Self::Id(_)
                | Self::NotADate(_)
                | Self::Unreadable(_)
        )
    }
}

/// Whether `character` can be part of a run of letters and digits.
fn in_chunk(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '-'
}

/// Every token of `text`, in order.
pub(crate) fn tokens(text: &str) -> Vec<Token> {
    let mut found = Vec::new();
    let mut rest = text.char_indices().peekable();
    while let Some((start, character)) = rest.next() {
        let single = |kind| Token {
            kind,
            span: Span::new(start, start + character.len_utf8()),
        };
        let token = match character {
            c if c.is_whitespace() => continue,
            '(' => single(TokenKind::OpenBracket),
            ')' => single(TokenKind::CloseBracket),
            ',' => single(TokenKind::Comma),
            '<' | '>' => {
                let or_equal = rest.next_if(|&(_, next)| next == '=').is_some();
                let (kind, width) = match (character, or_equal) {
                    ('<', false) => (TokenKind::Less, 1),
                    ('<', true) => (TokenKind::LessOrEqual, 2),
                    (_, false) => (TokenKind::Greater, 1),
                    (_, true) => (TokenKind::GreaterOrEqual, 2),
                };
                Token {
                    kind,
                    span: Span::new(start, start + width),
                }
            }
            QUOTE => quoted(text, start, &mut rest),
            c if in_chunk(c) => {
                let mut end = start + c.len_utf8();
                while let Some(&(at, next)) = rest.peek() {
                    let decimal_point = next == '.'
                        && text[..at].ends_with(|d: char| d.is_ascii_digit())
                        && text[at + 1..].starts_with(|d: char| d.is_ascii_digit());
                    if !(in_chunk(next) || decimal_point) {
                        break;
                    }
                    rest.next();
                    end = at + next.len_utf8();
                }
                Token {
                    kind: chunk(&text[start..end]),
                    span: Span::new(start, end),
                }
            }
            other => single(TokenKind::Unreadable(other.to_string())),
        };
        found.push(token);
    }
    found
}

/// The quoted text that starts at `start` (the opening quote is read already).
fn quoted(
    text: &str,
    start: usize,
    rest: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut content = String::new();
    let mut end = text.len();
    let mut closed = false;
    while let Some((at, character)) = rest.next() {
        if character == ESCAPE {
            match rest.next_if(|&(_, next)| next == QUOTE || next == ESCAPE) {
                Some((_, escaped)) => content.push(escaped),
                None => content.push(character),
            }
        } else if character == QUOTE {
            end = at + 1;
            closed = true;
            break;
        } else {
            content.push(character);
        }
    }
    Token {
        kind: TokenKind::Quoted {
            text: content,
            closed,
        },
        span: Span::new(start, end),
    }
}

/// What a run of letters, digits, dashes and decimal points is.
fn chunk(text: &str) -> TokenKind {
    let first = text.chars().next().unwrap_or_default();
    if let Some(number) = Decimal::parse(text) {
        TokenKind::Number(number)
    } else if let Some(date) = Date::parse(text) {
        TokenKind::Date(date)
    } else if is_readable_id(text) {
        TokenKind::Id(text.to_owned())
    } else if looks_like_a_date(text) {
        TokenKind::NotADate(text.to_owned())
    } else if first.is_alphabetic() && !text.contains(['-', '.']) {
        TokenKind::Word(text.to_owned())
    } else {
        TokenKind::Unreadable(text.to_owned())
    }
}

/// Whether the text is made like a day (`dddd-dd-dd`) whatever the numbers are.
fn looks_like_a_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    matches!(parts.as_slice(), [year, month, day]
        if [(year, 4), (month, 2), (day, 2)]
            .iter()
            .all(|(part, width)| part.len() == *width && part.bytes().all(|b| b.is_ascii_digit())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<TokenKind> {
        tokens(text).into_iter().map(|token| token.kind).collect()
    }

    #[test]
    fn words_numbers_days_ids_and_signs_are_told_apart() {
        let found = kinds("priority >= 2.5 and due < 2026-10-25 and under WEB-K7Q2X (x)");
        assert_eq!(found[0], TokenKind::Word("priority".into()));
        assert_eq!(found[1], TokenKind::GreaterOrEqual);
        assert!(matches!(
            found[2],
            TokenKind::Number(Decimal {
                value: 25,
                scale: 1
            })
        ));
        assert_eq!(found[5], TokenKind::Less);
        assert!(matches!(found[6], TokenKind::Date(_)));
        assert_eq!(found[9], TokenKind::Id("WEB-K7Q2X".into()));
        assert_eq!(found[10], TokenKind::OpenBracket);
    }

    #[test]
    fn a_quoted_text_unescapes_and_may_be_left_open() {
        assert_eq!(
            kinds(r#""say \"hi\" \\ \n""#),
            [TokenKind::Quoted {
                text: "say \"hi\" \\ \\n".into(),
                closed: true
            }]
        );
        let open = tokens("a \"unfinished");
        assert_eq!(
            open[1].kind,
            TokenKind::Quoted {
                text: "unfinished".into(),
                closed: false
            }
        );
        assert_eq!(open[1].span, Span::new(2, 13));
        assert_eq!(open[1].span.end, 13);
    }

    #[test]
    fn spans_are_bytes_of_the_text() {
        let found = tokens("ünï is é");
        assert_eq!(found[0].span.end, "ünï".len());
        assert_eq!(found[2].span.start, "ünï is ".len());
    }

    #[test]
    fn pieces_that_mean_nothing_are_kept_for_the_parser_to_name() {
        assert_eq!(
            kinds("2026-13-40"),
            [TokenKind::NotADate("2026-13-40".into())]
        );
        assert_eq!(kinds("7days"), [TokenKind::Unreadable("7days".into())]);
        assert_eq!(kinds("a=b")[1], TokenKind::Unreadable("=".into()));
        assert_eq!(kinds("web-1"), [TokenKind::Unreadable("web-1".into())]);
    }

    proptest::proptest! {
        #[test]
        fn the_tokens_of_any_text_lie_in_order_on_the_boundaries_of_characters(text in ".{0,80}") {
            let mut end = 0;
            for token in tokens(&text) {
                proptest::prop_assert!(token.span.start >= end);
                proptest::prop_assert!(token.span.start < token.span.end);
                proptest::prop_assert!(text.is_char_boundary(token.span.start));
                proptest::prop_assert!(text.is_char_boundary(token.span.end));
                end = token.span.end;
            }
            proptest::prop_assert!(end <= text.len());
        }
    }
}
