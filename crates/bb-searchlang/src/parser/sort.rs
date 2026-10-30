//! `sort by`: the keys, their fields and their directions.

use super::{Parser, Step};
use crate::lexer::TokenKind;
use crate::situation::{ParseError, Reason, Situation};
use crate::span::Span;
use crate::syntax::{Direction, SortKey};
use crate::words::{Keyword, direction_phrase, starts_direction, starts_phrase};

impl Parser<'_> {
    pub(super) fn sort_clause(&mut self) -> Step<Vec<SortKey>> {
        self.advance();
        if self.eat_keyword(Keyword::By).is_none() {
            return self.fail(Situation::SortBy);
        }
        let mut keys = Vec::new();
        loop {
            let field = self.field_ref(Situation::SortField)?;
            self.hint = Situation::SortAfterField;
            let (direction, end) = self.sort_direction(field.span.end)?;
            if direction.is_some() {
                self.hint = Situation::SortAfterKey;
            }
            keys.push(SortKey {
                span: Span::new(field.span.start, end),
                field,
                direction,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                return Ok(keys);
            }
            self.hint = Situation::SortField;
        }
    }

    /// The direction after a field of `sort by`, with or without a comma before it, and where the
    /// key ends.
    fn sort_direction(&mut self, field_end: usize) -> Step<(Option<Direction>, usize)> {
        let after_comma = usize::from(self.peek_kind(0) == Some(&TokenKind::Comma));
        if !self.direction_starts(after_comma) {
            return Ok((None, field_end));
        }
        if after_comma == 1 {
            self.advance();
        }
        for direction in Direction::ALL {
            let phrase = direction_phrase(direction);
            let words: Vec<&str> = (0..phrase.len())
                .map_while(|ahead| match self.peek_kind(ahead) {
                    Some(TokenKind::Word(text)) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            if words.len() == phrase.len() && starts_phrase(phrase, &words) {
                let mut end = field_end;
                for _ in phrase {
                    end = self.advance().end;
                }
                return Ok((Some(direction), end));
            }
        }
        let typed = self.typed_direction_words();
        let first = self.peek().map_or_else(Span::default, |token| token.span);
        Err(ParseError {
            at_end: self.at + typed.len() >= self.tokens.len(),
            reason: Reason::Expected(Situation::Direction { typed }),
            span: first,
        })
    }

    /// Whether a direction begins `ahead` tokens on (`words::starts_direction`).
    fn direction_starts(&self, ahead: usize) -> bool {
        let Some(TokenKind::Word(word)) = self.peek_kind(ahead) else {
            return false;
        };
        let next = match self.peek_kind(ahead + 1) {
            Some(TokenKind::Word(next)) => Some(next.as_str()),
            _ => None,
        };
        starts_direction(word, next)
    }

    /// The words of a direction that are typed and fit the start of one of the directions.
    fn typed_direction_words(&self) -> Vec<String> {
        let mut typed: Vec<&str> = Vec::new();
        while let Some(TokenKind::Word(word)) = self.peek_kind(typed.len()) {
            typed.push(word);
            if !Direction::ALL
                .iter()
                .any(|direction| starts_phrase(direction_phrase(*direction), &typed))
            {
                typed.pop();
                break;
            }
        }
        typed.into_iter().map(str::to_lowercase).collect()
    }
}
