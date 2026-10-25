//! The checker: a tree and the field catalogue go in; the tree, checked, or the mistakes come out.
//! It looks at the fields, the operators, the kind of each value, the fixed lists of choices and the
//! two limits. The server checks every search again and never relies on the browser's check
//! (ADR-0005, amendment M5, decision 4).
//!
//! A field that is not in the catalogue gives the same mistake whether it does not exist or the
//! person may not read it (QRY-05): the checker cannot tell the two apart, because the catalogue
//! only holds what the person may read.

use crate::catalogue::{Field, FieldCatalogue, FieldKind, Operator};
use crate::limits::{MAX_CONDITIONS, MAX_STEPS};
use crate::messages as key;
use crate::mistake::Mistake;
use crate::parser::value_span;
use crate::similar::closest;
use crate::span::Span;
use crate::syntax::{Comparison, Expr, FieldRef, Search, Test, Value};
use crate::words::Keyword;

/// A search that the checker has accepted, for a catalogue. The compiler of the server takes this
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
pub fn check(search: Search, catalogue: &FieldCatalogue) -> Result<CheckedSearch, Vec<Mistake>> {
    let mut checker = Checker {
        catalogue,
        conditions: 0,
        mistakes: Vec::new(),
    };
    if let Some(filter) = &search.filter {
        checker.expression(filter);
    }
    for key in &search.sort {
        checker.known_field(&key.field);
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
    catalogue: &'a FieldCatalogue,
    conditions: usize,
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
                self.count(*span);
                if let Some(found) = self.known_field(field) {
                    self.test(found, test);
                }
            }
            Expr::Under { span, .. } | Expr::Blocks { span, .. } | Expr::BlockedBy { span, .. } => {
                self.count(*span);
            }
            Expr::ConnectedTo { steps, span, .. } => {
                self.count(*span);
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

    /// Counts a condition; the first one over the limit is the mistake.
    fn count(&mut self, span: Span) {
        self.conditions += 1;
        if self.conditions == MAX_CONDITIONS + 1 {
            self.mistakes.push(Mistake::new(
                span,
                key::TOO_MANY_CONDITIONS,
                &[("limit", &MAX_CONDITIONS.to_string())],
            ));
        }
    }

    /// The field with this word, or the mistake of a field that is not known.
    fn known_field(&mut self, field: &FieldRef) -> Option<&'a Field> {
        let found = self.catalogue.field(&field.word);
        if found.is_none() {
            let suggestions = closest(
                &field.word,
                self.catalogue
                    .fields
                    .iter()
                    .map(|field| field.word.as_str()),
            );
            self.mistakes.push(unknown_field(field, suggestions));
        }
        found
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
        let values: &[Value] = match test {
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

/// The mistake of a field that the catalogue does not have. Nothing in it tells a field that does
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
        Test::Compare { operator, span, .. } => (
            match operator {
                Comparison::Less => Operator::Less,
                Comparison::Greater => Operator::Greater,
                Comparison::LessOrEqual => Operator::LessOrEqual,
                Comparison::GreaterOrEqual => Operator::GreaterOrEqual,
            },
            *span,
        ),
        Test::In { span, .. } => (Operator::In, *span),
        Test::Contains { span, .. } => (Operator::Contains, *span),
        Test::IsEmpty { span } => (Operator::IsEmpty, *span),
        Test::IsNotEmpty { span } => (Operator::IsNotEmpty, *span),
        Test::InLast { span, .. } => (Operator::InLast, *span),
        Test::MemberOf { span, .. } => (Operator::MemberOf, *span),
        Test::Within { span, .. } => (Operator::Within, *span),
    }
}

/// The words of a value as the choices of a field spell them.
fn spelled(value: &Value) -> String {
    match value {
        Value::Text { text, .. } => text.clone(),
        Value::Number { number, .. } => number.text(),
        Value::Date { date, .. } => date.text(),
        Value::Issue { id, .. } => id.text.clone(),
        Value::Me { .. } => Keyword::Me.text().to_owned(),
        Value::Time { word, .. } => word.text(),
    }
}

/// What is wrong with a value for a field, if anything.
fn value_mistake(field: &Field, value: &Value) -> Option<Mistake> {
    let span = value_span(value);
    let wrong = |message: &str| Some(Mistake::new(span, message, &[("field", &field.word)]));
    match (field.kind, value) {
        (FieldKind::Text | FieldKind::Project, Value::Text { .. })
        | (FieldKind::Number, Value::Number { .. })
        | (FieldKind::Date, Value::Date { .. } | Value::Time { .. })
        | (FieldKind::Person, Value::Text { .. } | Value::Me { .. })
        | (FieldKind::Issue, Value::Issue { .. }) => None,
        (FieldKind::Choice, _) => unknown_choice(field, value, span),
        (FieldKind::Text | FieldKind::Project, _) => wrong(key::WRONG_TEXT),
        (FieldKind::Number, _) => wrong(key::WRONG_NUMBER),
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
fn unknown_choice(field: &Field, value: &Value, span: Span) -> Option<Mistake> {
    let wanted = spelled(value);
    if field.choices.is_empty()
        || field
            .choices
            .iter()
            .any(|choice| choice.eq_ignore_ascii_case(&wanted))
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
