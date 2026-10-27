//! The checker: a tree and the readable fields go in; the tree, checked, or the mistakes come out.
//! It looks at the fields, the operators, the kind of each value, the fixed lists of choices and the
//! two limits. The server checks every search again and never relies on the browser's check
//! (ADR-0005, amendment M5, decision 4).
//!
//! A field that is not among the readable fields gives the same mistake whether it does not exist or the
//! person may not read it (QRY-05): the checker cannot tell the two apart, because readable fields
//! only holds what the person may read.

use bb_core::names::same_name;
use bb_core::text_words::search_words;

use crate::fields::{Field, FieldKind, Operator, ReadableFields};
use crate::limits::{MAX_CONTAINS_WORDS, MAX_FILTERS, MAX_SORT_KEYS, MAX_STEPS, MIN_PREFIX_CHARS};
use crate::messages as key;
use crate::mistake::Mistake;
use crate::parser::value_span;
use crate::similar::closest;
use crate::span::Span;
use crate::syntax::{Expr, FieldRef, Operand, Search, Test, Text};
use crate::words::{Keyword, duration_words};

/// A search that the checker has accepted, for readable fields. The compiler of the server takes this
/// type, so a search that was not checked cannot be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSearch(Search);

impl CheckedSearch {
    /// The search.
    #[must_use]
    pub const fn search(&self) -> &Search {
        &self.0
    }

    /// The search, given up.
    #[must_use]
    pub fn into_search(self) -> Search {
        self.0
    }
}

/// Checks `search` against the fields the person may read. All mistakes are returned, in the order
/// of the text.
///
/// # Errors
/// The mistakes, when there is at least one.
pub fn check(search: Search, readable: &ReadableFields) -> Result<CheckedSearch, Vec<Mistake>> {
    let mut checker = Checker {
        readable,
        filters: 0,
        mistakes: Vec::new(),
    };
    if let Some(filter) = &search.filter {
        checker.expression(filter);
    }
    for key in search.sort.iter().take(MAX_SORT_KEYS) {
        checker.known_field(&key.field);
    }
    if let Some(over) = search.sort.get(MAX_SORT_KEYS) {
        checker.mistakes.push(Mistake::new(
            over.span,
            key::TOO_MANY_SORT_KEYS,
            &[("limit", &MAX_SORT_KEYS.to_string())],
        ));
    }
    let mut mistakes = checker.mistakes;
    mistakes.sort_by_key(|mistake| mistake.span.start);
    if mistakes.is_empty() {
        Ok(CheckedSearch(search))
    } else {
        Err(mistakes)
    }
}

struct Checker<'a> {
    readable: &'a ReadableFields,
    filters: usize,
    mistakes: Vec<Mistake>,
}

impl<'a> Checker<'a> {
    fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Or { items, .. } | Expr::And { items, .. } => {
                for item in items {
                    self.expression(item);
                }
            }
            Expr::Not { item, .. } => self.expression(item),
            Expr::Field { field, test, span } => {
                if !self.count(*span, test_values(test)) {
                    return;
                }
                if let Some(found) = self.known_field(field) {
                    self.test(found, test);
                }
            }
            Expr::Contains { text, span } => {
                if !self.count(*span, 1) {
                    return;
                }
                // A `contains` on its own looks in the title and in the description among other
                // texts: a person who may read only one of them has no such search, told as a
                // field that is not there (nothing tells a hidden field from a missing one).
                if self.readable.allows_bare_contains() {
                    self.words_mistakes(text);
                } else {
                    self.mistakes.push(Mistake::new(
                        Span::new(span.start, span.start + "contains".len()),
                        key::UNKNOWN_FIELD,
                        &[("field", "contains")],
                    ));
                }
            }
            Expr::Under { span, .. } | Expr::Blocks { span, .. } | Expr::BlockedBy { span, .. } => {
                self.count(*span, 1);
            }
            Expr::ConnectedTo { steps, span, .. } => {
                if !self.count(*span, 1) {
                    return;
                }
                if let Some(steps) = steps.as_ref().filter(|steps| steps.count > MAX_STEPS) {
                    self.mistakes.push(Mistake::new(
                        steps.span,
                        key::TOO_MANY_STEPS,
                        &[("limit", &MAX_STEPS.to_string())],
                    ));
                }
            }
        }
    }

    /// Counts a filter of `values` comparisons; the first one over the limit is the mistake.
    /// Whether the filter is within the limit: nothing is looked at beyond it, so a hostile search
    /// costs no more than the limit allows (no search for close words for the thousandth filter).
    fn count(&mut self, span: Span, values: usize) -> bool {
        let before = self.filters;
        self.filters = self.filters.saturating_add(values);
        if self.filters <= MAX_FILTERS {
            return true;
        }
        if before <= MAX_FILTERS {
            self.mistakes.push(Mistake::new(
                span,
                key::TOO_MANY_FILTERS,
                &[("limit", &MAX_FILTERS.to_string())],
            ));
        }
        false
    }

    /// The field with this word, or the mistake of a field that is not known.
    fn known_field(&mut self, field: &FieldRef) -> Option<&'a Field> {
        let found = self.readable.field(&field.word);
        if found.is_none() {
            let suggestions = closest(
                &field.word,
                self.readable.fields.iter().map(|field| field.word.as_str()),
            );
            self.mistakes.push(unknown_field(field, suggestions));
        }
        found
    }

    /// The mistakes of the words of a `contains`: too many, or a start with too few letters.
    fn words_mistakes(&mut self, text: &Text) {
        let words = search_words(&text.text);
        if words.len() > MAX_CONTAINS_WORDS {
            self.mistakes.push(Mistake::new(
                text.span,
                key::TOO_MANY_WORDS,
                &[("limit", &MAX_CONTAINS_WORDS.to_string())],
            ));
        }
        if words
            .iter()
            .any(|word| word.is_prefix() && !word.matches_by_start())
        {
            self.mistakes.push(Mistake::new(
                text.span,
                key::PREFIX_TOO_SHORT,
                &[("limit", &MIN_PREFIX_CHARS.to_string())],
            ));
        }
    }

    fn test(&mut self, field: &Field, test: &Test) {
        let (operator, span) = operator_of(test);
        if !field.operators.contains(&operator) {
            self.mistakes.push(Mistake::new(
                span,
                key::OPERATOR_NOT_ALLOWED,
                &[("field", &field.word), ("operator", &operator.text())],
            ));
            return;
        }
        if let Test::Contains { text, .. } = test {
            self.words_mistakes(text);
        }
        let values: &[Operand] = match test {
            Test::Is { value, .. } | Test::IsNot { value, .. } | Test::Compare { value, .. } => {
                std::slice::from_ref(value)
            }
            Test::In { values, .. } => values,
            _ => &[],
        };
        for value in values {
            if let Some(mistake) = value_mistake(field, value) {
                self.mistakes.push(mistake);
            }
        }
    }
}

/// How many comparisons a test makes: one for each value of `in (…)`, one for any other.
fn test_values(test: &Test) -> usize {
    match test {
        Test::In { values, .. } => values.len().max(1),
        _ => 1,
    }
}

/// The mistake of a field that the readable fields do not have. Nothing in it tells a field that does
/// not exist from one that is hidden.
fn unknown_field(field: &FieldRef, suggestions: Vec<String>) -> Mistake {
    let args = [("field", field.word.as_str())];
    match suggestions.first() {
        Some(first) => Mistake::new(
            field.span,
            key::UNKNOWN_FIELD_DID_YOU_MEAN,
            &[args[0], ("suggestion", first)],
        ),
        None => Mistake::new(field.span, key::UNKNOWN_FIELD, &args),
    }
    .meaning(suggestions)
}

/// The operator a test uses, and where its words stand.
fn operator_of(test: &Test) -> (Operator, Span) {
    match test {
        Test::Is { span, .. } => (Operator::Is, *span),
        Test::IsNot { span, .. } => (Operator::IsNot, *span),
        Test::Compare { operator, span, .. } => (Operator::from(*operator), *span),
        Test::In { span, .. } => (Operator::In, *span),
        Test::Contains { span, .. } => (Operator::Contains, *span),
        Test::IsEmpty { span } => (Operator::IsEmpty, *span),
        Test::IsNotEmpty { span } => (Operator::IsNotEmpty, *span),
        Test::InLast { span, .. } => (Operator::InLast, *span),
        Test::MemberOf { span, .. } => (Operator::MemberOf, *span),
        Test::Within { span, .. } => (Operator::Within, *span),
    }
}

/// What is wrong with a value for a field, if anything.
fn value_mistake(field: &Field, value: &Operand) -> Option<Mistake> {
    let span = value_span(value);
    let wrong = |message: &str| Some(Mistake::new(span, message, &[("field", &field.word)]));
    match (field.kind, value) {
        (FieldKind::Text | FieldKind::Project, Operand::Text { .. })
        | (FieldKind::Number, Operand::Number { .. })
        | (FieldKind::Duration, Operand::Duration { .. })
        | (FieldKind::Date, Operand::Date { .. } | Operand::Time { .. })
        | (FieldKind::Person, Operand::Text { .. } | Operand::Me { .. })
        | (FieldKind::Issue, Operand::Issue { .. }) => None,
        (FieldKind::Choice, _) => unknown_choice(field, value, span),
        (FieldKind::Text | FieldKind::Project, _) => wrong(key::WRONG_TEXT),
        (FieldKind::Number, _) => wrong(key::WRONG_NUMBER),
        (FieldKind::Duration, _) => Some(Mistake::new(
            span,
            key::WRONG_DURATION,
            &[("field", field.word.as_str())]
                .into_iter()
                .chain(duration_words())
                .collect::<Vec<_>>(),
        )),
        (FieldKind::Date, _) => wrong(key::WRONG_DATE),
        (FieldKind::Person, _) => Some(Mistake::new(
            span,
            key::WRONG_PERSON,
            &[("field", &field.word), ("me", Keyword::Me.text())],
        )),
        (FieldKind::Issue, _) => wrong(key::WRONG_ISSUE),
    }
}

/// The mistake of a value that is not among the choices of a field with a fixed list.
fn unknown_choice(field: &Field, value: &Operand, span: Span) -> Option<Mistake> {
    let wanted = value.spelling();
    // A length of time is no choice, even when a choice is spelled like it.
    let is_duration = matches!(value, Operand::Duration { .. });
    if !is_duration
        && (field.choices.is_empty()
            || field
                .choices
                .iter()
                .any(|choice| same_name(choice, &wanted)))
    {
        return None;
    }
    let suggestions = closest(&wanted, field.choices.iter().map(String::as_str));
    let mistake = match suggestions.first() {
        Some(first) => Mistake::new(
            span,
            key::UNKNOWN_CHOICE_DID_YOU_MEAN,
            &[
                ("field", &field.word),
                ("value", &wanted),
                ("suggestion", first),
            ],
        ),
        None => Mistake::new(
            span,
            key::UNKNOWN_CHOICE,
            &[("field", &field.word), ("value", &wanted)],
        ),
    };
    Some(mistake.meaning(suggestions))
}
