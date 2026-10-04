//! DATA-MARKS: every stored field is marked as personal data or not (ADR-0015, PRIV-02).
//!
//! The derive macro `#[derive(Entity)]` already stops the build for an unmarked field. This check
//! reads the same marks from the source, so the mistake shows up right after the edit, and it
//! checks what the compiler cannot: the words of the marks, and that every entity is named in the
//! `entities!` list that the personal-data map is built from. The counts of marked fields are on
//! the generated page `docs/privacy/personal-data.md` (`cargo xtask gen privacy`).

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::generate::privacy::{self, Mark, Model};
use crate::rust_source::read_product_sources;

pub(crate) struct DataMarks;

impl Check for DataMarks {
    fn id(&self) -> &'static str {
        "DATA-MARKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = read_product_sources(context.repo)?;
        Ok(evaluate(&privacy::read_model(&sources)?))
    }
}

fn evaluate(model: &Model) -> Vec<Finding> {
    let mut findings = Vec::new();
    if privacy::require_vocabulary(model).is_err() {
        findings.push(Finding::at(
            privacy::VOCABULARY_PATH,
            "The enums `Category` and `Basis` are missing; the marks have no words to use.",
        ));
        return findings;
    }
    let categories: BTreeSet<&str> = model
        .vocabulary
        .categories
        .iter()
        .map(|w| w.code.as_str())
        .collect();
    let bases: BTreeSet<&str> = model
        .vocabulary
        .bases
        .iter()
        .map(|w| w.code.as_str())
        .collect();
    let listed: BTreeSet<&str> = model.listed.iter().map(String::as_str).collect();
    let mut names = BTreeSet::new();
    for entity in &model.entities {
        if !names.insert(entity.name.as_str()) {
            findings.push(Finding::at(
                &entity.path,
                format!(
                    "Another entity is also called `{}`; give each entity its own name.",
                    entity.name
                ),
            ));
        }
        if !listed.contains(entity.name.as_str()) {
            findings.push(Finding::at(
                &entity.path,
                format!(
                    "`{}` is not in the `entities!` list in {}, so the personal-data map would miss it.",
                    entity.name,
                    privacy::LIST_PATH
                ),
            ));
        }
        for field in &entity.fields {
            let place = format!("{}: {}.{}", entity.path, entity.name, field.name);
            match &field.mark {
                Err(problem) => findings.push(Finding::at(place, format!("The field {problem}."))),
                Ok(Mark::NotPersonal) => {}
                Ok(Mark::Personal { category, basis }) => {
                    if !categories.contains(category.as_str()) {
                        findings.push(Finding::at(
                            place.clone(),
                            format!(
                                "`{category}` is not a category; use one of {}.",
                                join(&categories)
                            ),
                        ));
                    }
                    if !bases.contains(basis.as_str()) {
                        findings.push(Finding::at(
                            place,
                            format!(
                                "`{basis}` is not a legal basis; use one of {}.",
                                join(&bases)
                            ),
                        ));
                    }
                }
            }
        }
    }
    let known: BTreeSet<&str> = model.entities.iter().map(|e| e.name.as_str()).collect();
    for name in listed.difference(&known) {
        findings.push(Finding::at(
            privacy::LIST_PATH,
            format!(
                "`{name}` is in the `entities!` list but no struct with that name derives `Entity`."
            ),
        ));
    }
    findings
}

fn join(words: &BTreeSet<&str>) -> String {
    words.iter().copied().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust_source::Source;

    const VOCABULARY: &str = "pub enum Category { Identity, EmployeeTime }\npub enum Basis { Contract, LegalObligation }\n";
    const LIST: &str = "crate::entities! { crate::person::Person }";

    fn findings(person: &str, list: &str) -> Vec<Finding> {
        let sources: Vec<Source> = vec![
            (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            (privacy::LIST_PATH.to_owned(), list.to_owned()),
            ("crates/bb-core/src/person.rs".to_owned(), person.to_owned()),
        ];
        evaluate(&privacy::read_model(&sources).unwrap())
    }

    #[test]
    fn good_example() {
        let person = "#[derive(Entity)]\npub struct Person {\n    #[not_personal]\n    pub id: u32,\n    #[personal(category = \"identity\", basis = \"contract\")]\n    pub email: String,\n}\n";
        assert!(findings(person, LIST).is_empty());
    }

    #[test]
    fn bad_example() {
        let person = "#[derive(Entity)]\npub struct Person {\n    pub id: u32,\n    #[personal(category = \"secrets\", basis = \"whim\")]\n    pub email: String,\n}\n#[derive(Entity)]\npub struct Forgotten {\n    #[not_personal]\n    pub id: u32,\n}\n";
        let messages: Vec<String> = findings(
            person,
            "crate::entities! { crate::person::Person, crate::x::Ghost }",
        )
        .into_iter()
        .map(|f| f.message)
        .collect();
        let all = messages.join("\n");
        assert!(all.contains("has no mark"), "{all}");
        assert!(all.contains("`secrets` is not a category"), "{all}");
        assert!(all.contains("`whim` is not a legal basis"), "{all}");
        assert!(
            all.contains("`Forgotten` is not in the `entities!` list"),
            "{all}"
        );
        assert!(all.contains("`Ghost` is in the `entities!` list"), "{all}");
    }

    #[test]
    fn the_entities_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        let findings = evaluate(&privacy::read_model(&sources).unwrap());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
