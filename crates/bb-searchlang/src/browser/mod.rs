//! What the WebAssembly module of the browser reads: the tree that Simple search edited and the
//! readable fields, as JSON. Only the module turns the feature `browser` on; the server never reads
//! a tree from outside, it parses a text and checks it again (ADR-0005, amendment M5, decision 4).
//!
//! The reader is written from the types of the tree (`cargo xtask gen search-reader`), so its shape
//! stands in one place (`syntax.rs`, `fields.rs`); deriving `Deserialize` makes the module bigger
//! than its budget. The types cannot keep the rules of a tree that came from the parser, so a tree
//! that is read is held to them here, and the canonical text is only given when the language reads
//! it back as the very same tree. How deep a tree may be is the parser's rule alone (`MAX_DEPTH`
//! counts brackets and `not` in a text): a tree that is too deep has no text that reads back, and
//! the reader's own recursion is bounded by the depth that `serde_json` reads.

use serde_json::Value as Json;

#[path = "reader.generated.rs"]
mod reader;

use crate::fields::ReadableFields;
use crate::literal::{is_short_name, issue_id};
use crate::syntax::{Date, Decimal, Expr, IssueRef, Operand, ProjectRef, Search, Test};

/// What is wrong with the JSON: in plain words, for the developer of the browser (a defect, never
/// something a person caused).
pub type Problem = String;

/// Reads the readable fields.
///
/// # Errors
/// The JSON is not what the readable fields are.
pub fn readable_fields_from_json(json: &str) -> Result<ReadableFields, Problem> {
    reader::read_readable_fields(&parsed(json)?)
}

fn parsed(json: &str) -> Result<Json, Problem> {
    serde_json::from_str(json).map_err(|error| error.to_string())
}

/// Reads a tree, and holds it to the rules of a tree.
///
/// # Errors
/// The JSON is not a tree, or the tree breaks a rule.
pub fn search_from_json(json: &str) -> Result<Search, Problem> {
    let search = reader::read_search(&parsed(json)?)?;
    if let Some(filter) = &search.filter {
        expression(filter)?;
    }
    Ok(search)
}

/// The canonical text of a tree in JSON. The tree must keep the rules, and the language must read
/// the text back as the same tree (`parse(print(tree))` has the words of `tree`), or the text would
/// not be the search that Simple search shows.
///
/// # Errors
/// The tree cannot be read, breaks a rule, or has no text of the language.
pub fn print_from_json(json: &str) -> Result<String, Problem> {
    let search = search_from_json(json)?;
    let text = crate::print(&search);
    match crate::parse(&text) {
        Ok(back) if back.same_words(&search) => Ok(text),
        _ => Err("the tree has no text that the language reads back as the same tree".to_owned()),
    }
}

fn expression(node: &Expr) -> Result<(), Problem> {
    match node {
        Expr::Or { items, .. } | Expr::And { items, .. } => {
            if items.len() < 2 {
                return Err("an `and` or an `or` holds at least two items".to_owned());
            }
            let same_kind = |item: &Expr| {
                matches!(
                    (node, item),
                    (Expr::And { .. }, Expr::And { .. }) | (Expr::Or { .. }, Expr::Or { .. })
                )
            };
            if items.iter().any(same_kind) {
                return Err("an `and` or an `or` holds none of its own kind".to_owned());
            }
            items.iter().try_for_each(expression)
        }
        Expr::Not { item, .. } => expression(item),
        Expr::Field { test, .. } => check_test(test),
        Expr::Under { id, .. } | Expr::Blocks { id, .. } | Expr::BlockedBy { id, .. } => issue(id),
        Expr::ConnectedTo { id, steps, .. } => {
            issue(id)?;
            if steps.as_ref().is_some_and(|steps| steps.count == 0) {
                return Err("a chain of relations has at least one step".to_owned());
            }
            Ok(())
        }
    }
}

fn check_test(test: &Test) -> Result<(), Problem> {
    match test {
        Test::Is { value, .. } | Test::IsNot { value, .. } | Test::Compare { value, .. } => {
            operand(value)
        }
        Test::In { values, .. } => {
            if values.is_empty() {
                return Err("`in` holds at least one value".to_owned());
            }
            values.iter().try_for_each(operand)
        }
        Test::InLast { count: 0, .. } => Err("`in last` counts at least one".to_owned()),
        Test::Within { project, .. } => short_name(project),
        Test::Contains { .. }
        | Test::IsEmpty { .. }
        | Test::IsNotEmpty { .. }
        | Test::InLast { .. }
        | Test::MemberOf { .. } => Ok(()),
    }
}

fn operand(value: &Operand) -> Result<(), Problem> {
    match value {
        Operand::Number { number, .. } => decimal(*number),
        Operand::Date { date, .. } => {
            if Date::new(date.year, date.month, date.day) == Some(*date) {
                Ok(())
            } else {
                Err("a day is not in the calendar".to_owned())
            }
        }
        Operand::Issue { id, .. } => issue(id),
        Operand::Text { .. } | Operand::Me { .. } | Operand::Time { .. } => Ok(()),
    }
}

/// A number is held to its one form: the shortest, with no more digits than the language reads.
fn decimal(number: Decimal) -> Result<(), Problem> {
    if Decimal::parse(&number.text()) == Some(number) {
        Ok(())
    } else {
        Err("a number is not in its shortest form, or has too many digits".to_owned())
    }
}

fn issue(id: &IssueRef) -> Result<(), Problem> {
    if issue_id(&id.text).as_deref() == Some(id.text.as_str()) {
        Ok(())
    } else {
        Err(format!("`{}` is no Issue ID in capitals", id.text))
    }
}

fn short_name(name: &ProjectRef) -> Result<(), Problem> {
    if is_short_name(&name.text) {
        Ok(())
    } else {
        Err(format!("`{}` is no short name", name.text))
    }
}
