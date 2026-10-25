//! A mistake in a search: where it is, a message key with the words for its placeholders, and what
//! the person may have meant. The parser and the checker both make them, and the browser shows
//! them the way the server would (QRY-03).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::messages as key;
use crate::situation::{ParseError, Reason, Situation};
use crate::span::Span;
use crate::syntax::{Direction, Unit};
use crate::words::Keyword;

/// Something wrong in the text of a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mistake {
    /// The bytes of the text it is about.
    pub span: Span,
    /// The key of the message in the catalog.
    pub key: String,
    /// The words for the placeholders of the message, by name.
    pub args: BTreeMap<String, String>,
    /// What the person may have meant, closest first; empty when nothing is close.
    pub suggestions: Vec<String>,
}

impl Mistake {
    /// A mistake with its message and the words for its placeholders.
    pub(crate) fn new(span: Span, key: &str, args: &[(&str, &str)]) -> Self {
        Self {
            span,
            key: key.to_owned(),
            args: args
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            suggestions: Vec::new(),
        }
    }

    /// The mistake, with `suggestions` of what may have been meant.
    pub(crate) fn meaning(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

impl From<&ParseError> for Mistake {
    fn from(error: &ParseError) -> Self {
        let span = error.span;
        match &error.reason {
            Reason::Expected(situation) => expected(span, situation),
            Reason::ReservedWord(word) => Self::new(span, key::RESERVED_WORD, &[("word", word)]),
            Reason::UnclosedQuote => Self::new(span, key::UNCLOSED_QUOTE, &[]),
            Reason::NotADate(text) => Self::new(span, key::NOT_A_DATE, &[("text", text)]),
            Reason::Unreadable(text) => Self::new(span, key::UNREADABLE, &[("text", text)]),
            Reason::TooDeep => Self::new(span, key::TOO_DEEP, &[]),
            Reason::TooLong => Self::new(span, key::TOO_LONG, &[]),
        }
    }
}

/// The words of a list, with commas and a last "or".
fn listed(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{}, {}", init.join(", "), last),
    }
}

/// The mistake for what the grammar wanted at `span`.
fn expected(span: Span, situation: &Situation) -> Mistake {
    let word = |keyword: Keyword| keyword.text();
    let with = |key: &str, args: &[(&str, &str)]| Mistake::new(span, key, args);
    let operator = |field: &String| {
        with(
            key::EXPECTED_OPERATOR,
            &[
                ("field", field),
                ("is", word(Keyword::Is)),
                ("contains", word(Keyword::Contains)),
                ("in", word(Keyword::In)),
            ],
        )
    };
    match situation {
        Situation::Start | Situation::Filter => with(key::EXPECTED_FILTER, &[]),
        Situation::Operator { field } => operator(field),
        Situation::Value { field }
        | Situation::IsRest { field }
        | Situation::IsNotRest { field } => with(key::EXPECTED_VALUE, &[("field", field)]),
        Situation::InRest => with(
            key::EXPECTED_IN,
            &[("in", word(Keyword::In)), ("last", word(Keyword::Last))],
        ),
        Situation::ListNext => with(key::EXPECTED_LIST_NEXT, &[]),
        Situation::Count => with(key::EXPECTED_COUNT, &[]),
        Situation::Unit => {
            let units = listed(&[Unit::Day, Unit::Week, Unit::Month].map(unit_text));
            with(key::EXPECTED_UNIT, &[("units", &units)])
        }
        Situation::StepsWord => with(
            key::EXPECTED_STEPS_WORD,
            &[
                ("step", word(Keyword::Step)),
                ("steps", word(Keyword::Steps)),
            ],
        ),
        Situation::Text | Situation::Group => with(key::EXPECTED_TEXT, &[]),
        Situation::MemberOf => with(
            key::EXPECTED_MEMBER_OF,
            &[("of", word(Keyword::Of)), ("member", word(Keyword::Member))],
        ),
        Situation::ShortName => with(key::EXPECTED_SHORT_NAME, &[]),
        Situation::ReadableId => with(key::EXPECTED_ISSUE_ID, &[]),
        Situation::BlockedBy => with(
            key::EXPECTED_BLOCKED_BY,
            &[
                ("by", word(Keyword::By)),
                ("blocked", word(Keyword::Blocked)),
            ],
        ),
        Situation::ConnectedTo => with(
            key::EXPECTED_CONNECTED_TO,
            &[
                ("to", word(Keyword::To)),
                ("connected", word(Keyword::Connected)),
            ],
        ),
        Situation::Continue { closing } => with(
            if *closing {
                key::EXPECTED_CONTINUE_CLOSING
            } else {
                key::EXPECTED_CONTINUE
            },
            &[
                ("and", word(Keyword::And)),
                ("or", word(Keyword::Or)),
                ("sort", word(Keyword::Sort)),
                ("by", word(Keyword::By)),
            ],
        ),
        Situation::SortBy => with(
            key::EXPECTED_SORT_BY,
            &[("by", word(Keyword::By)), ("sort", word(Keyword::Sort))],
        ),
        Situation::SortField => with(key::EXPECTED_SORT_FIELD, &[]),
        Situation::SortAfterField | Situation::SortAfterKey | Situation::Direction { .. } => {
            let directions = listed(&Direction::ALL.map(Direction::text));
            with(key::EXPECTED_DIRECTION, &[("directions", &directions)])
        }
    }
}

/// The text of a unit in the plural, as the message lists them.
fn unit_text(unit: Unit) -> String {
    crate::words::unit_word(unit, 2).text().to_owned()
}
