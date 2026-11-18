//! DATA-MARKS, the links of a person (ADR-0044, decision 1): every field whose type holds the ID
//! of a person says what the person is to the row, so the walk of a person's data finds the row.
//!
//! The derive refuses a link on a field that cannot name a person; this half finds the field that
//! names a person and has no link, which the compiler cannot see. The types that hold the ID of a
//! person are read from `names_a_person!` in `bb_core::personal`, the same list the derive checks
//! a link against.

use std::collections::BTreeSet;

use crate::checks::Finding;
use crate::generate::privacy::{self, Model};

pub(super) fn evaluate_links(model: &Model) -> Vec<Finding> {
    let mut findings = Vec::new();
    if model.vocabulary.links.is_empty() || model.person_types.is_empty() {
        findings.push(Finding::at(
            privacy::VOCABULARY_PATH,
            "The enum `Link` or the list `names_a_person!` is missing; a person field cannot say \
             what the person is to the row.",
        ));
        return findings;
    }
    let words: BTreeSet<&str> = model
        .vocabulary
        .links
        .iter()
        .map(|word| word.code.as_str())
        .collect();
    for entity in &model.entities {
        for field in &entity.fields {
            let place = format!("{}: {}.{}", entity.path, entity.name, field.name);
            let names_a_person = field
                .type_names
                .iter()
                .any(|name| model.person_types.contains(name));
            let mut linked = false;
            for link in &field.links {
                match link {
                    Err(problem) => {
                        findings.push(Finding::at(place.clone(), format!("The field {problem}.")));
                    }
                    Ok(link) => {
                        linked = true;
                        if !words.contains(link.word.as_str()) {
                            findings.push(Finding::at(
                                place.clone(),
                                format!(
                                    "`{}` is not a link; use one of {}.",
                                    link.word,
                                    words.iter().copied().collect::<Vec<_>>().join(", ")
                                ),
                            ));
                        }
                        if link.kind.is_none() && !names_a_person {
                            findings.push(Finding::at(
                                place.clone(),
                                "The field carries `#[person(…)]`, but its type does not hold \
                                 the ID of a person.",
                            ));
                        }
                    }
                }
            }
            if names_a_person && !linked {
                findings.push(Finding::at(
                    place,
                    format!(
                        "The field names a person but has no link. Add `#[person(…)]` with one of \
                         {} to say what the person is to the row (ADR-0044).",
                        words.iter().copied().collect::<Vec<_>>().join(", ")
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Source;

    const VOCABULARY: &str = "words! { pub enum Category { Identity => \"identity\" } }
words! { pub enum Basis { Contract => \"contract\" } }
words! { pub enum Link { Author => \"author\", Subject => \"subject\" } }
names_a_person!(crate::id::UserId, crate::event::Actor);
";

    fn findings(entity: &str) -> String {
        let sources: Vec<Source> = vec![
            (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            ("crates/bb-core/src/note.rs".to_owned(), entity.to_owned()),
        ];
        evaluate_links(&privacy::read_model(&sources).unwrap())
            .into_iter()
            .map(|f| f.message)
            .collect::<Vec<_>>()
            .join("\n")
    }

    const MARK: &str = "#[personal(category = \"identity\", basis = \"contract\")]";

    #[test]
    fn good_example_every_field_that_names_a_person_has_a_link() {
        let entity = format!(
            "#[derive(Entity)]\npub struct Note {{\n    {MARK}\n    #[person(author)]\n    pub author: UserId,\n    {MARK}\n    #[person(subject)]\n    pub about: Option<Vec<crate::id::UserId>>,\n    #[not_personal]\n    #[person_for(kind = \"person\", link = \"subject\")]\n    pub entity: EntityRef,\n    {MARK}\n    pub text: String,\n}}\n"
        );
        assert_eq!(findings(&entity), "");
    }

    #[test]
    fn bad_example_a_field_that_names_a_person_without_a_link() {
        let entity = format!(
            "#[derive(Entity)]\npub struct Note {{\n    {MARK}\n    pub author: UserId,\n    {MARK}\n    pub actor: Actor,\n}}\n"
        );
        let all = findings(&entity);
        assert_eq!(
            all.matches("names a person but has no link").count(),
            2,
            "{all}"
        );
    }

    #[test]
    fn bad_example_a_link_with_an_unknown_word_or_on_a_field_without_a_person() {
        let entity = format!(
            "#[derive(Entity)]\npub struct Note {{\n    {MARK}\n    #[person(writer)]\n    pub author: UserId,\n    {MARK}\n    #[person(author)]\n    pub text: String,\n    {MARK}\n    #[person_for(kind = \"person\")]\n    pub entity: EntityRef,\n}}\n"
        );
        let all = findings(&entity);
        assert!(all.contains("`writer` is not a link"), "{all}");
        assert!(
            all.contains("its type does not hold the ID of a person"),
            "{all}"
        );
        assert!(all.contains("needs both kind"), "{all}");
    }

    #[test]
    fn bad_example_the_words_of_the_links_are_missing() {
        let sources: Vec<Source> = vec![(
            privacy::VOCABULARY_PATH.to_owned(),
            "words! { pub enum Category { Identity => \"identity\" } }".to_owned(),
        )];
        let found = evaluate_links(&privacy::read_model(&sources).unwrap());
        assert!(found[0].message.contains("`Link`"), "{}", found[0].message);
    }
}
