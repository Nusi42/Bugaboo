//! Suggestions: given the text and the place of the cursor, what may come next. The parser says
//! what the grammar wants at that place (its `Situation`); this module turns that into words: the
//! readable fields, the operators a field allows, the choices it offers and the words of
//! the language. What only the server knows (people, groups, projects, issues) comes back as a
//! request of its kind, which the search bar sends to the server (M5 WP-07).

use serde::Serialize;

use crate::fields::{Field, FieldKind, ReadableFields};
use crate::lexer::{Token, TokenKind, tokens};
use crate::literal::{field_or_quoted, key_or_quoted, value_or_quoted};
use crate::parser::{excess, parse};
use crate::situation::Situation;
use crate::span::Span;
use crate::syntax::{Direction, TimeWord, Unit};
use crate::words::{Keyword, direction_phrase, starts_phrase, unit_word};

/// What to offer for the place of the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestions {
    /// The part of the text that a chosen word replaces: what is typed of the word, or the whole
    /// quoted text the cursor stands in. Empty (start equals end) when nothing is typed yet.
    pub replace: Span,
    /// What may come, in the order to show it.
    pub items: Vec<Suggestion>,
}

/// One offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Suggestion {
    /// A word to put in the text, written the way the printer writes it (in quotes when it needs
    /// them).
    Word {
        /// The text that replaces what is typed.
        text: String,
        /// What it is.
        kind: WordKind,
    },
    /// A kind of value that only the server knows: the search bar asks for the ones that start
    /// with what is typed.
    Ask {
        /// What to ask for.
        what: AskKind,
        /// What the names start with; empty for all.
        starting_with: String,
    },
}

/// What a suggested word is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WordKind {
    /// A field.
    Field,
    /// What to ask of a field (`is not`, `contains`).
    Operator,
    /// A value of a field.
    Value,
    /// A word of the language that joins, orders or ends.
    Keyword,
    /// A helper (`under`, `blocks`, `connected to`).
    Helper,
    /// A direction of `sort by`.
    Direction,
    /// A sign: a bracket or a comma.
    Sign,
}

/// What the server is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AskKind {
    /// People.
    People,
    /// Groups.
    Groups,
    /// Projects.
    Projects,
    /// Issues.
    Issues,
}

/// The suggestions for `text` with the cursor at byte `cursor`.
#[must_use]
pub fn suggest(text: &str, cursor: usize, readable: &ReadableFields) -> Suggestions {
    let mut cursor = cursor.min(text.len());
    if excess(text).is_some() {
        return Suggestions {
            replace: Span::new(cursor, cursor),
            items: Vec::new(),
        };
    }
    while !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let found = tokens(text);
    let quote = found.iter().find(|token| {
        matches!(&token.kind, TokenKind::Quoted { closed, .. }
            if token.span.start < cursor && (cursor < token.span.end || (!closed && cursor <= token.span.end)))
    });
    if let Some(quote) = quote {
        let typed = &text[quote.span.start + 1..cursor];
        return Suggestions {
            replace: quote.span,
            items: items(
                parse(&text[..quote.span.start]).next,
                typed,
                readable,
                Quoted::Yes,
            ),
        };
    }
    let partial = found.iter().find(|token| {
        token.kind.is_chunk() && token.span.start < cursor && cursor <= token.span.end
    });
    let (start, replace) = partial.map_or((cursor, Span::new(cursor, cursor)), |token| {
        (token.span.start, token.span)
    });
    let situation = parse(&text[..start]).next;
    let typed = &text[start..cursor];
    let offered = items(situation.clone(), typed, readable, Quoted::No);
    if offered.is_empty() && !typed.is_empty() {
        // The name of a field has several words (`status group`): what is typed may be the start of
        // its last words, after the words of a field that is a start of it already.
        if let Some(longer) = longer_field(text, &found, start, situation.as_ref(), typed, readable)
        {
            return Suggestions {
                replace: Span::new(longer.0, replace.end),
                items: longer.1,
            };
        }
    }
    Suggestions {
        replace,
        items: offered,
    }
}

/// The fields whose words start with the words of a field typed before `start` and what is typed
/// since, and where the field begins; `None` when the place is not after the first words of a
/// field, or no field starts so.
fn longer_field(
    text: &str,
    found: &[Token],
    start: usize,
    situation: Option<&Situation>,
    typed: &str,
    readable: &ReadableFields,
) -> Option<(usize, Vec<Suggestion>)> {
    let before: Vec<&Token> = found
        .iter()
        .filter(|token| token.span.end <= start)
        .collect();
    let (words, written): (usize, fn(&str) -> String) = match situation? {
        Situation::Operator { field } => (field.split(' ').count(), field_or_quoted),
        Situation::SortAfterField => (
            before
                .iter()
                .rev()
                .take_while(|token| match &token.kind {
                    TokenKind::Word(word) => Keyword::of(word) != Some(Keyword::By),
                    _ => false,
                })
                .count(),
            key_or_quoted,
        ),
        _ => return None,
    };
    let first = before
        .len()
        .checked_sub(words)
        .and_then(|at| before.get(at))?;
    if words == 0
        || !before[before.len() - words..]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Word(_)))
    {
        return None;
    }
    let begun = format!("{} {typed}", text[first.span.start..start].trim_end());
    let items: Vec<Suggestion> = readable
        .fields
        .iter()
        .filter(|field| starts_with(&field.word, &begun))
        .map(|field| word(written(&field.word), WordKind::Field))
        .collect();
    (!items.is_empty()).then_some((first.span.start, items))
}

/// Whether the cursor stands inside quotes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quoted {
    Yes,
    No,
}

/// Whether `word` starts with `typed`, in any case.
fn starts_with(word: &str, typed: &str) -> bool {
    word.to_lowercase().starts_with(&typed.to_lowercase())
}

fn word(text: String, kind: WordKind) -> Suggestion {
    Suggestion::Word { text, kind }
}

/// The offers for a situation. `typed` is what is typed of the word already.
fn items(
    situation: Option<Situation>,
    typed: &str,
    readable: &ReadableFields,
    quoted: Quoted,
) -> Vec<Suggestion> {
    let Some(situation) = situation else {
        return Vec::new();
    };
    let keyword = |keyword: Keyword, kind| word(keyword.text().to_owned(), kind);
    let sign = |text: &str| word(text.to_owned(), WordKind::Sign);
    // A field as its place writes it: in a filter, or as a key of `sort by`.
    let fields = |written: fn(&str) -> String| {
        readable
            .fields
            .iter()
            .map(move |field| word(written(&field.word), WordKind::Field))
    };
    let helpers = || {
        // A `contains` on its own is offered to a person who has such a search.
        let contains = readable
            .allows_bare_contains()
            .then(|| Keyword::Contains.text().to_owned());
        contains
            .into_iter()
            .chain([
                Keyword::Under.text().to_owned(),
                Keyword::Blocks.text().to_owned(),
                format!("{} {}", Keyword::Blocked.text(), Keyword::By.text()),
                format!("{} {}", Keyword::Connected.text(), Keyword::To.text()),
            ])
            .map(|text| word(text, WordKind::Helper))
    };
    let sort = || {
        word(
            format!("{} {}", Keyword::Sort.text(), Keyword::By.text()),
            WordKind::Keyword,
        )
    };
    let offered: Vec<Suggestion> = match situation {
        Situation::Start => fields(field_or_quoted)
            .chain([keyword(Keyword::Not, WordKind::Keyword), sign("(")])
            .chain(helpers())
            .chain([sort()])
            .collect(),
        Situation::Filter => fields(field_or_quoted)
            .chain([keyword(Keyword::Not, WordKind::Keyword), sign("(")])
            .chain(helpers())
            .collect(),
        Situation::Operator { field } => readable.field(&field).map_or_else(Vec::new, |field| {
            field
                .operators
                .iter()
                .map(|operator| word(operator.text(), WordKind::Operator))
                .collect()
        }),
        Situation::IsRest { field } => [
            keyword(Keyword::Not, WordKind::Operator),
            keyword(Keyword::Empty, WordKind::Operator),
        ]
        .into_iter()
        .chain(values(readable.field(&field), typed))
        .collect(),
        Situation::IsNotRest { field } => [keyword(Keyword::Empty, WordKind::Operator)]
            .into_iter()
            .chain(values(readable.field(&field), typed))
            .collect(),
        Situation::Value { field } => values(readable.field(&field), typed),
        Situation::InRest => vec![keyword(Keyword::Last, WordKind::Operator), sign("(")],
        Situation::ListNext => vec![sign(","), sign(")")],
        Situation::Unit => [Unit::Day, Unit::Week, Unit::Month]
            .map(|unit| keyword(unit_word(unit, 2), WordKind::Keyword))
            .into(),
        Situation::StepsWord => vec![keyword(Keyword::Steps, WordKind::Keyword)],
        Situation::MemberOf => vec![keyword(Keyword::Of, WordKind::Keyword)],
        Situation::BlockedBy | Situation::SortBy => vec![keyword(Keyword::By, WordKind::Keyword)],
        Situation::ConnectedTo => vec![keyword(Keyword::To, WordKind::Keyword)],
        Situation::ProjectRef => vec![ask(AskKind::Projects, typed)],
        Situation::IssueRef => vec![ask(AskKind::Issues, typed)],
        Situation::Group => vec![ask(AskKind::Groups, typed)],
        Situation::Continue { closing } => {
            let mut next = vec![
                keyword(Keyword::And, WordKind::Keyword),
                keyword(Keyword::Or, WordKind::Keyword),
            ];
            next.push(if closing { sign(")") } else { sort() });
            next
        }
        Situation::SortField => fields(key_or_quoted).collect(),
        Situation::SortAfterField => Direction::ALL
            .map(|direction| word(direction.text(), WordKind::Direction))
            .into_iter()
            .chain([sign(",")])
            .collect(),
        Situation::SortAfterKey => vec![sign(",")],
        Situation::Direction { typed: words } => next_direction_words(&words),
        Situation::Count | Situation::Duration | Situation::Text => Vec::new(),
    };
    offered
        .into_iter()
        .filter(|offer| match offer {
            // Inside quotes only the values fit; their quotes belong to what replaces the text.
            Suggestion::Word { text, kind } => {
                (quoted == Quoted::No || *kind == WordKind::Value)
                    && starts_with(text.trim_matches('"'), typed)
            }
            Suggestion::Ask { .. } => true,
        })
        .collect()
}

fn ask(what: AskKind, typed: &str) -> Suggestion {
    Suggestion::Ask {
        what,
        starting_with: typed.to_owned(),
    }
}

/// The values a field offers: the choices of its list, `me`, the time words, or a request of the
/// server.
fn values(field: Option<&Field>, typed: &str) -> Vec<Suggestion> {
    let Some(field) = field else {
        return Vec::new();
    };
    let value = |text: String| word(text, WordKind::Value);
    match field.kind {
        FieldKind::Choice => field
            .choices
            .iter()
            .map(|choice| value(value_or_quoted(choice)))
            .collect(),
        FieldKind::Person => vec![
            value(Keyword::Me.text().to_owned()),
            ask(AskKind::People, typed),
        ],
        FieldKind::Project => vec![ask(AskKind::Projects, typed)],
        FieldKind::Issue => vec![ask(AskKind::Issues, typed)],
        FieldKind::Date => TimeWord::ALL.map(|word| value(word.text())).into(),
        FieldKind::Text | FieldKind::Number | FieldKind::Duration => Vec::new(),
    }
}

/// The next word of each direction that starts with the words typed so far.
fn next_direction_words(typed: &[String]) -> Vec<Suggestion> {
    let typed: Vec<&str> = typed.iter().map(String::as_str).collect();
    Direction::ALL
        .iter()
        .filter_map(|direction| {
            let phrase = direction_phrase(*direction);
            (starts_phrase(phrase, &typed) && phrase.len() > typed.len())
                .then(|| word(phrase[typed.len()].text(), WordKind::Direction))
        })
        .collect()
}
