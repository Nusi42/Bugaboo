//! The printer: one canonical text for every tree. One spelling for each word, one way to quote,
//! one space between words, brackets only where the meaning needs them, and the order of `sort by`
//! kept. The address, the API and a saved search carry this text (ADR-0005, amendment M5).

use crate::literal::{quoted, value_or_quoted, word_or_quoted};
use crate::syntax::{Expr, Operand, Search, SortKey, Test, Text};
use crate::words::{Keyword, steps_word, unit_word};

/// Whether this build checks that a canonical text reads back as the same search.
pub const ROUND_TRIP_GUARD: bool = cfg!(feature = "round-trip-guard");

/// The canonical text of a search, given only when the language reads it back as the same search
/// (ADR-0005 amendment M5, decision 8). The check is the feature `round-trip-guard`: the server and
/// the tests build with it, so a defect of the printer can never be saved or run. The module of the
/// browser builds without it: there the text is the one `print`
/// writes, and nothing in the browser refuses a text because of the check. Whatever the browser
/// prints is sent to the server, which parses and checks it again and makes its own canonical text.
#[must_use]
pub fn canonical_text(search: &Search) -> Option<String> {
    let text = print(search);
    #[cfg(feature = "round-trip-guard")]
    {
        match crate::parse(&text) {
            Ok(back) if back.same_words(search) => Some(text),
            _ => None,
        }
    }
    #[cfg(not(feature = "round-trip-guard"))]
    {
        Some(text)
    }
}

/// The canonical text of a search.
#[must_use]
pub fn print(search: &Search) -> String {
    let mut parts = Vec::new();
    if let Some(filter) = &search.filter {
        parts.push(expression(filter, Level::Or));
    }
    if !search.sort.is_empty() {
        let keys: Vec<String> = search.sort.iter().map(sort_key).collect();
        parts.push(format!(
            "{} {} {}",
            Keyword::Sort.text(),
            Keyword::By.text(),
            keys.join(", ")
        ));
    }
    parts.join(" ")
}

/// How tightly an expression binds. A part that binds less than its place asks for is put in
/// brackets.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Level {
    Or,
    And,
    Single,
}

fn level_of(expression: &Expr) -> Level {
    match expression {
        Expr::Or { .. } => Level::Or,
        Expr::And { .. } => Level::And,
        _ => Level::Single,
    }
}

/// The text of an expression where parts that bind less than `needed` need brackets. The items of
/// an `and` and of an `or` need a tighter binding than their own, so a same kind inside the same
/// kind (which a tree built by hand may hold) keeps its brackets and its meaning.
fn expression(node: &Expr, needed: Level) -> String {
    let own = level_of(node);
    let text = match node {
        Expr::Or { items, .. } => joined(items, Keyword::Or, Level::And),
        Expr::And { items, .. } => joined(items, Keyword::And, Level::Single),
        Expr::Not { item, .. } => format!(
            "{} {}",
            Keyword::Not.text(),
            expression(item, Level::Single)
        ),
        Expr::Field { field, test, .. } => {
            format!("{} {}", word_or_quoted(&field.word), test_text(test))
        }
        Expr::Contains { text, .. } => format!("{} {}", Keyword::Contains.text(), text_of(text)),
        Expr::Under { id, .. } => format!("{} {}", Keyword::Under.text(), id.text),
        Expr::Blocks { id, .. } => format!("{} {}", Keyword::Blocks.text(), id.text),
        Expr::BlockedBy { id, .. } => format!(
            "{} {} {}",
            Keyword::Blocked.text(),
            Keyword::By.text(),
            id.text
        ),
        Expr::ConnectedTo { id, steps, .. } => {
            let steps = steps.as_ref().map_or_else(String::new, |steps| {
                format!(
                    " {} {} {}",
                    Keyword::Within.text(),
                    steps.count,
                    steps_word(steps.count).text()
                )
            });
            format!(
                "{} {} {}{steps}",
                Keyword::Connected.text(),
                Keyword::To.text(),
                id.text
            )
        }
    };
    if own < needed {
        format!("({text})")
    } else {
        text
    }
}

fn joined(items: &[Expr], keyword: Keyword, needed: Level) -> String {
    let separator = format!(" {} ", keyword.text());
    items
        .iter()
        .map(|item| expression(item, needed))
        .collect::<Vec<_>>()
        .join(&separator)
}

fn test_text(test: &Test) -> String {
    match test {
        Test::Is { value, .. } => format!("{} {}", Keyword::Is.text(), value_text(value)),
        Test::IsNot { value, .. } => format!(
            "{} {} {}",
            Keyword::Is.text(),
            Keyword::Not.text(),
            value_text(value)
        ),
        Test::Compare {
            operator, value, ..
        } => format!("{} {}", operator.sign(), value_text(value)),
        Test::In { values, .. } => {
            let values: Vec<String> = values.iter().map(value_text).collect();
            format!("{} ({})", Keyword::In.text(), values.join(", "))
        }
        Test::Contains { text, .. } => format!("{} {}", Keyword::Contains.text(), text_of(text)),
        Test::IsEmpty { .. } => format!("{} {}", Keyword::Is.text(), Keyword::Empty.text()),
        Test::IsNotEmpty { .. } => format!(
            "{} {} {}",
            Keyword::Is.text(),
            Keyword::Not.text(),
            Keyword::Empty.text()
        ),
        Test::InLast { count, unit, .. } => format!(
            "{} {} {} {}",
            Keyword::In.text(),
            Keyword::Last.text(),
            count,
            unit_word(*unit, *count).text()
        ),
        Test::MemberOf { group, .. } => format!(
            "{} {} {}",
            Keyword::Member.text(),
            Keyword::Of.text(),
            text_of(group)
        ),
        Test::Within { project, .. } => {
            format!("{} {}", Keyword::Within.text(), project.text)
        }
    }
}

/// A text that always stands in quotes.
fn text_of(text: &Text) -> String {
    quoted(&text.text)
}

fn value_text(value: &Operand) -> String {
    match value {
        Operand::Text { text, .. } => value_or_quoted(text),
        other => other.spelling(),
    }
}

fn sort_key(key: &SortKey) -> String {
    let field = word_or_quoted(&key.field.word);
    match key.direction {
        Some(direction) => format!("{field}, {}", direction.text()),
        None => field,
    }
}
