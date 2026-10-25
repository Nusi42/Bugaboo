//! What a text comes to: parsed, optionally checked, printed. The server and the browser both
//! answer a search bar with this one shape, so there is one place that decides what a text
//! comes to (QRY-03).

use serde::Serialize;

use crate::catalogue::FieldCatalogue;
use crate::checker::check;
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

/// Parses `text`, checks it when the fields are given, and prints it.
#[must_use]
pub fn read(text: &str, catalogue: Option<&FieldCatalogue>) -> Answer {
    let search = match crate::parse(text) {
        Ok(search) => search,
        Err(mistake) => {
            return Answer {
                search: None,
                canonical: None,
                mistakes: vec![mistake],
            };
        }
    };
    let mistakes = catalogue
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
    use crate::vectors::sample_catalogue;

    #[test]
    fn a_text_comes_to_its_tree_and_canonical_text_or_to_mistakes() {
        let good = read("STATUS is  Open", None);
        assert_eq!(good.canonical.as_deref(), Some("STATUS is Open"));
        assert!(good.mistakes.is_empty());
        let unchecked = read("nothing is Open", None);
        assert!(unchecked.mistakes.is_empty(), "no catalogue, no check");
        let catalogue = sample_catalogue();
        let wrong = read("nothing is Open", Some(&catalogue));
        assert!(wrong.search.is_some() && wrong.canonical.is_none());
        assert_eq!(wrong.mistakes.len(), 1);
        let broken = read("status is", Some(&catalogue));
        assert!(broken.search.is_none() && broken.mistakes.len() == 1);
    }
}
