//! The parser: recursive descent over the tokens, one function for each rule of the grammar in
//! architecture section 7. It never panics, on any text: nesting is limited (`MAX_DEPTH`), numbers
//! are checked, and every way to run out of tokens is an error that says what was expected.
//!
//! When a text ends where more is expected, the error holds the `Situation`: what the grammar
//! wants at that place. The mistakes in plain words and the suggestions are both made from it, so
//! what the person is told and what is offered cannot disagree.

use crate::lexer::{Token, TokenKind, tokens};
use crate::limits::MAX_DEPTH;
use crate::literal::is_short_name;
use crate::span::Span;
use crate::syntax::{
    Comparison, Direction, Expr, FieldRef, ReadableId, Search, ShortName, SortKey, Steps, Test,
    Text, TimeWord, Value,
};
use crate::words::{Keyword, Part, direction_phrase, starts_phrase, time_phrase, unit_of};

/// What the grammar wants at a place of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Situation {
    /// The start of the text: a condition, or `sort by`.
    Start,
    /// A condition: a field, `not`, a bracket or a helper.
    Condition,
    /// The operator of a field.
    Operator { field: String },
    /// A value for a field, after `is not`, a comparison or inside `in (…)`.
    Value { field: String },
    /// After `is`: `not`, `empty` or a value.
    IsRest { field: String },
    /// After `is not`: `empty` or a value.
    IsNotRest { field: String },
    /// After `in`: `last` or an opening bracket.
    InRest,
    /// After a value in `in (…)`: a comma or the closing bracket.
    ListNext,
    /// A whole number of at least one.
    Count,
    /// `days`, `weeks` or `months`.
    Unit,
    /// `step` or `steps`.
    StepsWord,
    /// The text of `contains`.
    Text,
    /// The name of the group of `member of`.
    Group,
    /// After `member`: `of`.
    MemberOf,
    /// The short name of a project.
    ShortName,
    /// An Issue ID.
    ReadableId,
    /// After `blocked`: `by`.
    BlockedBy,
    /// After `connected`: `to`.
    ConnectedTo,
    /// After a whole condition: `and`, `or`, `sort by`, a closing bracket, or the end.
    Continue { closing: bool },
    /// After `sort`: `by`.
    SortBy,
    /// A field to sort by.
    SortField,
    /// After the field of a sort key: a direction, a comma, or the end.
    SortAfterField,
    /// After a whole sort key: a comma or the end.
    SortAfterKey,
    /// A direction, of which the person has typed the first `typed` words.
    Direction { typed: Vec<String> },
}

/// Why a text is not a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The grammar wants something else here.
    Expected(Situation),
    /// A word of the language where a value is needed: it has to be quoted.
    ReservedWord(String),
    /// A quoted text that does not end.
    UnclosedQuote,
    /// Something that looks like a day but is not one.
    NotADate(String),
    /// A piece the language does not know.
    Unreadable(String),
    /// Brackets or `not` nested deeper than the limit.
    TooDeep,
}

/// A failed parse: why, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParseError {
    pub(crate) reason: Reason,
    pub(crate) span: Span,
    /// Whether the text ended before the error: nothing was found where something is needed.
    pub(crate) at_end: bool,
}

/// What a parse of a text leaves: the search, or the error, and what the grammar wants next.
pub(crate) struct Parsed {
    pub(crate) result: Result<Search, ParseError>,
    /// What the grammar wants where the text ends (for the suggestions); `None` when the text is
    /// wrong before its end.
    pub(crate) next: Option<Situation>,
}

/// Parses the text of a search.
pub(crate) fn parse(text: &str) -> Parsed {
    let mut parser = Parser {
        text,
        tokens: tokens(text),
        at: 0,
        depth: 0,
        hint: Situation::Start,
    };
    let result = parser.search();
    let next = match &result {
        Err(ParseError {
            reason: Reason::Expected(situation),
            at_end: true,
            ..
        }) => Some(situation.clone()),
        Err(_) => None,
        Ok(_) => Some(parser.hint),
    };
    Parsed { result, next }
}

type Step<T> = Result<T, ParseError>;

struct Parser<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
    /// What may come where the text ends, if the text is a whole search so far.
    hint: Situation,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn peek_kind(&self, ahead: usize) -> Option<&TokenKind> {
        self.tokens.get(self.at + ahead).map(|token| &token.kind)
    }

    /// The keyword that the token `ahead` of the next one spells, if it is a word and a keyword.
    fn keyword_at(&self, ahead: usize) -> Option<Keyword> {
        match self.peek_kind(ahead) {
            Some(TokenKind::Word(word)) => Keyword::of(word),
            _ => None,
        }
    }

    fn advance(&mut self) -> Span {
        let span = self
            .peek()
            .map_or(Span::new(self.text.len(), self.text.len()), |t| t.span);
        self.at += 1;
        span
    }

    fn eat_keyword(&mut self, keyword: Keyword) -> Option<Span> {
        (self.keyword_at(0) == Some(keyword)).then(|| self.advance())
    }

    fn eat(&mut self, kind: &TokenKind) -> Option<Span> {
        (self.peek_kind(0) == Some(kind)).then(|| self.advance())
    }

    /// The end of the previous token.
    fn last_end(&self) -> usize {
        self.at
            .checked_sub(1)
            .and_then(|before| self.tokens.get(before))
            .map_or(0, |token| token.span.end)
    }

    /// The error for the token that is next (or for the end of the text): it is what the grammar
    /// found where `situation` is wanted.
    fn fail<T>(&self, situation: Situation) -> Step<T> {
        let end = Span::new(self.text.len(), self.text.len());
        let (reason, span, at_end) = match self.peek() {
            None => (Reason::Expected(situation), end, true),
            Some(Token { kind, span }) => {
                let reason = match kind {
                    TokenKind::Quoted { closed: false, .. } => Reason::UnclosedQuote,
                    TokenKind::NotADate(text) => Reason::NotADate(text.clone()),
                    TokenKind::Unreadable(text) => Reason::Unreadable(text.clone()),
                    _ => Reason::Expected(situation),
                };
                (reason, *span, false)
            }
        };
        Err(ParseError {
            reason,
            span,
            at_end,
        })
    }

    fn reserved<T>(&self, word: &str) -> Step<T> {
        Err(ParseError {
            reason: Reason::ReservedWord(word.to_owned()),
            span: self.peek().map_or_else(Span::default, |token| token.span),
            at_end: false,
        })
    }

    fn enter(&mut self) -> Step<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ParseError {
                reason: Reason::TooDeep,
                span: self.peek().map_or_else(Span::default, |token| token.span),
                at_end: false,
            });
        }
        Ok(())
    }

    fn at_sort(&self) -> bool {
        self.keyword_at(0) == Some(Keyword::Sort)
    }

    fn search(&mut self) -> Step<Search> {
        let filter = if self.peek().is_none() || self.at_sort() {
            None
        } else {
            Some(self.or()?)
        };
        let sort = if self.at_sort() {
            self.sort_clause()?
        } else {
            Vec::new()
        };
        if self.peek().is_some() {
            return self.fail(Situation::Continue { closing: false });
        }
        Ok(Search {
            filter,
            sort,
            span: Span::new(0, self.text.len()),
        })
    }

    fn or(&mut self) -> Step<Expr> {
        let first = self.and()?;
        let mut rest = Vec::new();
        while self.eat_keyword(Keyword::Or).is_some() {
            rest.push(self.and()?);
        }
        Ok(Expr::any(first, rest))
    }

    fn and(&mut self) -> Step<Expr> {
        let first = self.unary()?;
        let mut rest = Vec::new();
        while self.eat_keyword(Keyword::And).is_some() {
            rest.push(self.unary()?);
        }
        Ok(Expr::all(first, rest))
    }

    fn unary(&mut self) -> Step<Expr> {
        if let Some(not) = self.eat_keyword(Keyword::Not) {
            self.enter()?;
            let item = self.unary()?;
            self.depth -= 1;
            let span = not.to(item.span());
            return Ok(Expr::Not {
                item: Box::new(item),
                span,
            });
        }
        if let Some(open) = self.eat(&TokenKind::OpenBracket) {
            self.enter()?;
            self.hint = Situation::Condition;
            let inner = self.or()?;
            self.hint = Situation::Continue { closing: true };
            let Some(close) = self.eat(&TokenKind::CloseBracket) else {
                return self.fail(Situation::Continue { closing: true });
            };
            self.depth -= 1;
            self.hint = Situation::Continue {
                closing: self.depth > 0,
            };
            // The brackets are part of the expression's place in the text, not of the tree.
            return Ok(with_span(inner, open.to(close)));
        }
        let condition = self.condition()?;
        self.hint = Situation::Continue {
            closing: self.depth > 0,
        };
        Ok(condition)
    }

    fn condition(&mut self) -> Step<Expr> {
        let start = self
            .peek()
            .map_or(self.text.len(), |token| token.span.start);
        match self.peek_kind(0) {
            Some(TokenKind::Word(_)) => match self.keyword_at(0) {
                Some(Keyword::Under) => {
                    self.advance();
                    let id = self.readable_id()?;
                    Ok(Expr::Under {
                        span: Span::new(start, id.span.end),
                        id,
                    })
                }
                Some(Keyword::Blocks) => {
                    self.advance();
                    let id = self.readable_id()?;
                    Ok(Expr::Blocks {
                        span: Span::new(start, id.span.end),
                        id,
                    })
                }
                Some(Keyword::Blocked) => {
                    self.advance();
                    if self.eat_keyword(Keyword::By).is_none() {
                        return self.fail(Situation::BlockedBy);
                    }
                    let id = self.readable_id()?;
                    Ok(Expr::BlockedBy {
                        span: Span::new(start, id.span.end),
                        id,
                    })
                }
                Some(Keyword::Connected) => self.connected_to(start),
                Some(_) => self.fail(Situation::Condition),
                None => self.field_condition(),
            },
            Some(TokenKind::Quoted { closed: true, .. }) => self.field_condition(),
            _ => self.fail(Situation::Condition),
        }
    }

    fn connected_to(&mut self, start: usize) -> Step<Expr> {
        self.advance();
        if self.eat_keyword(Keyword::To).is_none() {
            return self.fail(Situation::ConnectedTo);
        }
        let id = self.readable_id()?;
        let mut steps = None;
        if self.keyword_at(0) == Some(Keyword::Within) {
            self.advance();
            let (count, count_span) = self.count()?;
            if !matches!(self.keyword_at(0), Some(Keyword::Step | Keyword::Steps)) {
                return self.fail(Situation::StepsWord);
            }
            let end = self.advance();
            steps = Some(Steps {
                count,
                span: count_span.to(end),
            });
        }
        let end = steps.as_ref().map_or(id.span.end, |steps| steps.span.end);
        Ok(Expr::ConnectedTo {
            span: Span::new(start, end),
            id,
            steps,
        })
    }

    fn readable_id(&mut self) -> Step<ReadableId> {
        if let Some(TokenKind::Id(text)) = self.peek_kind(0) {
            let text = text.clone();
            let span = self.advance();
            return Ok(ReadableId { text, span });
        }
        self.fail(Situation::ReadableId)
    }

    /// A field: a plain word or a quoted text.
    fn field_ref(&mut self, wanted: Situation) -> Step<FieldRef> {
        match self.peek_kind(0) {
            Some(TokenKind::Word(word)) => {
                if Keyword::of(word).is_some() {
                    let word = word.clone();
                    return self.reserved(&word);
                }
                let word = word.clone();
                let span = self.advance();
                Ok(FieldRef { word, span })
            }
            Some(TokenKind::Quoted { text, closed: true }) => {
                let word = text.clone();
                let span = self.advance();
                Ok(FieldRef { word, span })
            }
            _ => self.fail(wanted),
        }
    }

    fn field_condition(&mut self) -> Step<Expr> {
        let field = self.field_ref(Situation::Condition)?;
        let test = self.test(&field.word)?;
        Ok(Expr::Field {
            span: field.span.to(test_span(&test)),
            field,
            test,
        })
    }

    fn test(&mut self, field: &str) -> Step<Test> {
        let start = self
            .peek()
            .map_or(self.text.len(), |token| token.span.start);
        let named = || field.to_owned();
        let compare = |parser: &mut Self, operator| -> Step<Test> {
            parser.advance();
            let value = parser.value(Situation::Value { field: named() })?;
            Ok(Test::Compare {
                span: Span::new(start, value_span(&value).end),
                operator,
                value,
            })
        };
        match self.peek_kind(0) {
            Some(TokenKind::Less) => return compare(self, Comparison::Less),
            Some(TokenKind::Greater) => return compare(self, Comparison::Greater),
            Some(TokenKind::LessOrEqual) => return compare(self, Comparison::LessOrEqual),
            Some(TokenKind::GreaterOrEqual) => return compare(self, Comparison::GreaterOrEqual),
            _ => {}
        }
        match self.keyword_at(0) {
            Some(Keyword::Is) => {
                self.advance();
                self.is_test(field, start)
            }
            Some(Keyword::In) => {
                self.advance();
                self.in_test(field, start)
            }
            Some(Keyword::Contains) => {
                self.advance();
                let text = self.text_argument(Situation::Text)?;
                Ok(Test::Contains {
                    span: Span::new(start, text.span.end),
                    text,
                })
            }
            Some(Keyword::Member) => {
                self.advance();
                if self.eat_keyword(Keyword::Of).is_none() {
                    return self.fail(Situation::MemberOf);
                }
                let group = self.text_argument(Situation::Group)?;
                Ok(Test::MemberOf {
                    span: Span::new(start, group.span.end),
                    group,
                })
            }
            Some(Keyword::Within) => {
                self.advance();
                let project = self.short_name()?;
                Ok(Test::Within {
                    span: Span::new(start, project.span.end),
                    project,
                })
            }
            _ => self.fail(Situation::Operator { field: named() }),
        }
    }

    /// The rest of a test that starts with `is`.
    fn is_test(&mut self, field: &str, start: usize) -> Step<Test> {
        let named = field.to_owned();
        if self.eat_keyword(Keyword::Not).is_some() {
            if let Some(empty) = self.eat_keyword(Keyword::Empty) {
                return Ok(Test::IsNotEmpty {
                    span: Span::new(start, empty.end),
                });
            }
            let value = self.value(Situation::IsNotRest { field: named })?;
            return Ok(Test::IsNot {
                span: Span::new(start, value_span(&value).end),
                value,
            });
        }
        if let Some(empty) = self.eat_keyword(Keyword::Empty) {
            return Ok(Test::IsEmpty {
                span: Span::new(start, empty.end),
            });
        }
        let value = self.value(Situation::IsRest { field: named })?;
        Ok(Test::Is {
            span: Span::new(start, value_span(&value).end),
            value,
        })
    }

    /// The rest of a test that starts with `in`.
    fn in_test(&mut self, field: &str, start: usize) -> Step<Test> {
        if self.eat_keyword(Keyword::Last).is_some() {
            let (count, _) = self.count()?;
            let Some(unit) = self.keyword_at(0).and_then(unit_of) else {
                return self.fail(Situation::Unit);
            };
            let end = self.advance();
            return Ok(Test::InLast {
                count,
                unit,
                span: Span::new(start, end.end),
            });
        }
        if self.eat(&TokenKind::OpenBracket).is_none() {
            return self.fail(Situation::InRest);
        }
        let wanted = || Situation::Value {
            field: field.to_owned(),
        };
        let mut values = vec![self.value(wanted())?];
        while self.eat(&TokenKind::Comma).is_some() {
            values.push(self.value(wanted())?);
        }
        if self.eat(&TokenKind::CloseBracket).is_none() {
            return self.fail(Situation::ListNext);
        }
        Ok(Test::In {
            values,
            span: Span::new(start, self.last_end()),
        })
    }

    /// A whole number of at least one.
    fn count(&mut self) -> Step<(u32, Span)> {
        if let Some(TokenKind::Number(number)) = self.peek_kind(0)
            && number.scale == 0
            && let Ok(count) = u32::try_from(number.value)
            && count >= 1
        {
            let span = self.advance();
            return Ok((count, span));
        }
        self.fail(Situation::Count)
    }

    fn short_name(&mut self) -> Step<ShortName> {
        if let Some(TokenKind::Word(text)) = self.peek_kind(0)
            && is_short_name(text)
        {
            let text = text.clone();
            let span = self.advance();
            return Ok(ShortName { text, span });
        }
        self.fail(Situation::ShortName)
    }

    /// The text of `contains` or the group of `member of`: quoted, or one word, number, day or ID.
    fn text_argument(&mut self, wanted: Situation) -> Step<Text> {
        match self.peek_kind(0) {
            Some(TokenKind::Quoted { text, closed: true }) => {
                let text = text.clone();
                let span = self.advance();
                Ok(Text { text, span })
            }
            Some(TokenKind::Word(word))
                if Keyword::of(word).is_some_and(Keyword::reserved_as_value) =>
            {
                let word = word.clone();
                self.reserved(&word)
            }
            Some(
                TokenKind::Word(_) | TokenKind::Number(_) | TokenKind::Date(_) | TokenKind::Id(_),
            ) => {
                let span = self.advance();
                Ok(Text {
                    text: self.text[span.start..span.end].to_owned(),
                    span,
                })
            }
            _ => self.fail(wanted),
        }
    }

    fn value(&mut self, wanted: Situation) -> Step<Value> {
        let value = match self.peek_kind(0).cloned() {
            Some(TokenKind::Quoted { text, closed: true }) => {
                let span = self.advance();
                Value::Text { text, span }
            }
            Some(TokenKind::Number(number)) => Value::Number {
                number,
                span: self.advance(),
            },
            Some(TokenKind::Date(date)) => Value::Date {
                date,
                span: self.advance(),
            },
            Some(TokenKind::Id(text)) => {
                let span = self.advance();
                Value::Issue {
                    id: ReadableId { text, span },
                    span,
                }
            }
            Some(TokenKind::Word(word)) => return self.word_value(&word, wanted),
            _ => return self.fail(wanted),
        };
        Ok(value)
    }

    /// A value that is a word: `me`, a time word, or a plain word.
    fn word_value(&mut self, word: &str, wanted: Situation) -> Step<Value> {
        match Keyword::of(word) {
            None => {
                let span = self.advance();
                Ok(Value::Text {
                    text: word.to_owned(),
                    span,
                })
            }
            Some(Keyword::Me) => Ok(Value::Me {
                span: self.advance(),
            }),
            Some(keyword) if !keyword.reserved_as_value() => {
                let span = self.advance();
                Ok(Value::Text {
                    text: word.to_owned(),
                    span,
                })
            }
            Some(keyword) => {
                if let Some(value) = self.time_value() {
                    return Ok(value);
                }
                if matches!(keyword, Keyword::This | Keyword::Last) {
                    return self.fail(wanted);
                }
                self.reserved(word)
            }
        }
    }

    /// A time word at the next tokens.
    fn time_value(&mut self) -> Option<Value> {
        for word in TimeWord::ALL {
            let phrase = time_phrase(word);
            let words: Vec<&str> = (0..phrase.len())
                .map_while(|ahead| match self.peek_kind(ahead) {
                    Some(TokenKind::Word(text)) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            if words.len() == phrase.len() && starts_phrase(phrase, &words) {
                let first = self.advance();
                let mut span = first;
                for _ in 1..phrase.len() {
                    span = first.to(self.advance());
                }
                return Some(Value::Time { word, span });
            }
        }
        None
    }

    fn sort_clause(&mut self) -> Step<Vec<SortKey>> {
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

    /// Whether a direction begins `ahead` tokens on: its first word, and for a letter also `to`.
    fn direction_starts(&self, ahead: usize) -> bool {
        let Some(TokenKind::Word(word)) = self.peek_kind(ahead) else {
            return false;
        };
        Direction::ALL.iter().any(|direction| {
            let phrase = direction_phrase(*direction);
            phrase.first().is_some_and(|first| first.is(word))
                && (matches!(phrase.first(), Some(Part::Key(_)))
                    || self.keyword_at(ahead + 1) == Some(Keyword::To))
        })
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

/// The expression, with the span of its brackets.
fn with_span(expression: Expr, span: Span) -> Expr {
    match expression {
        Expr::Or { items, .. } => Expr::Or { items, span },
        Expr::And { items, .. } => Expr::And { items, span },
        Expr::Not { item, .. } => Expr::Not { item, span },
        Expr::Field { field, test, .. } => Expr::Field { field, test, span },
        Expr::Under { id, .. } => Expr::Under { id, span },
        Expr::Blocks { id, .. } => Expr::Blocks { id, span },
        Expr::BlockedBy { id, .. } => Expr::BlockedBy { id, span },
        Expr::ConnectedTo { id, steps, .. } => Expr::ConnectedTo { id, steps, span },
    }
}

fn test_span(test: &Test) -> Span {
    match test {
        Test::Is { span, .. }
        | Test::IsNot { span, .. }
        | Test::Compare { span, .. }
        | Test::In { span, .. }
        | Test::Contains { span, .. }
        | Test::IsEmpty { span }
        | Test::IsNotEmpty { span }
        | Test::InLast { span, .. }
        | Test::MemberOf { span, .. }
        | Test::Within { span, .. } => *span,
    }
}

/// Where a value stands.
pub(crate) fn value_span(value: &Value) -> Span {
    match value {
        Value::Text { span, .. }
        | Value::Number { span, .. }
        | Value::Date { span, .. }
        | Value::Issue { span, .. }
        | Value::Me { span }
        | Value::Time { span, .. } => *span,
    }
}
