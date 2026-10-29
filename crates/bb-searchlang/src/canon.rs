//! The words of a search as the person is shown them (ADR-0005 amendment M5, decision 11): once a
//! search is read with the fields it is for, every field is written by its word (`status group`,
//! also where the text said `Status Group` or `statusGroup`, the form of the old language) and
//! every value of a fixed list by the name the list gives (`High`, also where the text said
//! `high`). The words of the text before this are only what was typed; the tree after it is the one
//! the printer writes, so a saved search of the old language reads and is shown in the new words.

use bb_core::fields::word_key;
use bb_core::names::same_name;

use crate::fields::{Field, ReadableFields};
use crate::syntax::{Expr, FieldRef, Operand, Search, Test};

/// Puts the words of the fields and of their lists in place of what was typed. A field that the
/// fields do not have, or a value that is not in the list, stays as it was typed: the checker tells
/// the person.
pub(crate) fn put_in_words(search: &mut Search, readable: &ReadableFields) {
    if let Some(filter) = &mut search.filter {
        expression(filter, readable);
    }
    for key in &mut search.sort {
        field_word(&mut key.field, readable);
    }
}

fn field_word<'a>(field: &mut FieldRef, readable: &'a ReadableFields) -> Option<&'a Field> {
    let found = readable.field(&field.word)?;
    field.word.clone_from(&found.word);
    Some(found)
}

fn expression(node: &mut Expr, readable: &ReadableFields) {
    match node {
        Expr::Or { items, .. } | Expr::And { items, .. } => {
            for item in items {
                expression(item, readable);
            }
        }
        Expr::Not { item, .. } => expression(item, readable),
        Expr::Field { field, test, .. } => {
            if let Some(found) = field_word(field, readable) {
                values(test, found);
            }
        }
        Expr::Contains { .. }
        | Expr::Under { .. }
        | Expr::Blocks { .. }
        | Expr::BlockedBy { .. }
        | Expr::ConnectedTo { .. } => {}
    }
}

fn values(test: &mut Test, field: &Field) {
    if field.choices.is_empty() {
        return;
    }
    match test {
        Test::Is { value, .. } | Test::IsNot { value, .. } | Test::Compare { value, .. } => {
            choice_word(value, field);
        }
        Test::In { values, .. } => {
            for value in values {
                choice_word(value, field);
            }
        }
        _ => {}
    }
}

/// The name the list gives for the value: the same name (case and spaces do not count) first, else
/// the one name that has the same letters run together (`inProgress`).
fn choice_word(value: &mut Operand, field: &Field) {
    let Operand::Text { text, .. } = value else {
        return;
    };
    let exact = field.choices.iter().find(|choice| same_name(choice, text));
    let run_together = || {
        if !field.reads_run_together() {
            return None;
        }
        let wanted = word_key(text);
        let mut found = field
            .choices
            .iter()
            .filter(|choice| word_key(choice) == wanted);
        found.next().filter(|_| found.next().is_none())
    };
    if let Some(choice) = exact.or_else(run_together) {
        text.clone_from(choice);
    }
}
