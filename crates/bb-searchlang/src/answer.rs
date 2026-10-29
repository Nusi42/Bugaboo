//! What a text comes to: parsed, optionally checked, printed. The server and the browser both
//! answer a search bar with this one shape, so there is one place that decides what a text
//! comes to (QRY-03).

use serde::Serialize;

use crate::checker::check;
use crate::fields::ReadableFields;
use crate::lexer::tokens;
use crate::mistake::Mistake;
use crate::printer::print;
use crate::syntax::Search;

/// The answer to a text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Answer {
    /// The tree of a text that parses (also when the checker has mistakes); `null` otherwise.
    pub search: Option<Search>,
    /// The canonical text; `null` when the text has mistakes.
    pub canonical: Option<String>,
    /// The mistakes of the parser (one) or of the checker (all), in the order of the text.
    pub mistakes: Vec<Mistake>,
}

/// How many words and signs at the end of a text that does not parse are left out to read what comes
/// before the place where it stops (`prio is high and `).
const MOST_LEFT_OUT: usize = 6;

/// The mistakes of the part of a text before the place where it does not parse, in reading order:
/// a field that is not known in `prio is high and ` is the first thing wrong, before the `and` that
/// is not followed by anything (the mistake of the parser stays, last). Empty when the text up to
/// the last piece before the place reads whole and holds no mistake.
fn earlier_mistakes(text: &str, fields: &ReadableFields, stopped: &Mistake) -> Vec<Mistake> {
    let found = tokens(text);
    let before = found
        .iter()
        .filter(|token| token.span.end <= stopped.span.start);
    let starts: Vec<usize> = before.map(|token| token.span.start).collect();
    for left_out in 0..MOST_LEFT_OUT.min(starts.len()) {
        let end = starts[starts.len() - 1 - left_out];
        let Ok(search) = crate::parse_with(&text[..end], fields) else {
            continue;
        };
        return check(search, fields)
            .err()
            .unwrap_or_default()
            .into_iter()
            .filter(|mistake| mistake.span.end <= stopped.span.start)
            .collect();
    }
    Vec::new()
}

/// Parses `text`, checks it when the fields are given, and prints it. A text that does not parse has
/// the mistakes of what stands before the place where it stops, then the mistake of the parser.
#[must_use]
pub fn read(text: &str, readable: Option<&ReadableFields>) -> Answer {
    let parsed = match readable {
        Some(fields) => crate::parse_with(text, fields),
        None => crate::parse(text),
    };
    let search = match parsed {
        Ok(search) => search,
        Err(mistake) => {
            return Answer {
                search: None,
                canonical: None,
                mistakes: readable
                    .map_or_else(Vec::new, |fields| earlier_mistakes(text, fields, &mistake))
                    .into_iter()
                    .chain([mistake])
                    .collect(),
            };
        }
    };
    let mistakes = readable
        .and_then(|fields| check(search.clone(), fields).err())
        .unwrap_or_default();
    Answer {
        canonical: mistakes.is_empty().then(|| print(&search)),
        search: Some(search),
        mistakes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vectors::sample_fields;

    #[test]
    fn a_text_comes_to_its_tree_and_canonical_text_or_to_mistakes() {
        let good = read("STATUS is  Open", None);
        assert_eq!(good.canonical.as_deref(), Some("STATUS is Open"));
        assert!(good.mistakes.is_empty());
        let unchecked = read("nothing is Open", None);
        assert!(unchecked.mistakes.is_empty(), "no readable, no check");
        let readable = sample_fields();
        let wrong = read("nothing is Open", Some(&readable));
        assert!(wrong.search.is_some() && wrong.canonical.is_none());
        assert_eq!(wrong.mistakes.len(), 1);
        let broken = read("status is", Some(&readable));
        assert!(broken.search.is_none() && broken.mistakes.len() == 1);
    }
}
