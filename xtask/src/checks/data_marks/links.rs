//! DATA-MARKS, the links of a person (ADR-0044, decision 1): every field that can name a person
//! says when it does and what the person is to the row, so the walk of a person's data finds the
//! row.
//!
//! The derive refuses a link that does not fit the type of its field; this half finds the field
//! that has none, which the compiler cannot see: a field whose type holds the ID of a person needs
//! `#[person(…)]`, and a field whose type refers to a record of any kind needs `#[person_for(…)]`
//! or `#[names_no_person]`. The types are read from `names_a_person!` and `refers_to_a_record!` in
//! `bb_core::personal`, the same lists the derive checks a link against, with their aliases and
//! newtypes in `bb-core`.

use std::collections::BTreeSet;

use crate::checks::Finding;
use crate::generate::privacy::links::{CONTAINERS, Scope, TUPLE};
use crate::generate::privacy::{self, Field, Model};

pub(super) fn evaluate_links(model: &Model) -> Vec<Finding> {
    if model.vocabulary.links.is_empty() || model.person_types.is_empty() {
        return vec![Finding::at(
            privacy::LINKS_PATH,
            "The enum `Link` or the list `names_a_person!` is missing; a person field cannot say \
             what the person is to the row.",
        )];
    }
    let words: BTreeSet<&str> = model
        .vocabulary
        .links
        .iter()
        .map(|word| word.code.as_str())
        .collect();
    let all_words = words.iter().copied().collect::<Vec<_>>().join(", ");
    let mut findings = Vec::new();
    for entity in &model.entities {
        for field in &entity.fields {
            let place = format!("{}: {}.{}", entity.path, entity.name, field.name);
            findings.extend(
                field_problems(model, field, &words, &all_words)
                    .into_iter()
                    .map(|problem| Finding::at(place.clone(), problem)),
            );
        }
    }
    findings
}

/// What is wrong with the links of one field.
fn field_problems(
    model: &Model,
    field: &Field,
    words: &BTreeSet<&str>,
    all_words: &str,
) -> Vec<String> {
    let names = |list: &[String]| field.type_names.iter().any(|name| list.contains(name));
    let names_a_person = names(&model.person_types);
    let refers_to_a_record = names(&model.record_types);
    let carries_changes = names(&model.change_types);
    let holds_json = names(&model.json_types);
    let owned_id = names(&model.owned_id_types);
    let mut problems = Vec::new();
    let mut scopes = Vec::new();
    for link in &field.links {
        match link {
            Err(problem) => problems.push(format!("The field {problem}.")),
            Ok(link) => {
                let has_a_word = !matches!(link.scope, Scope::Never | Scope::Changes | Scope::Part);
                if has_a_word && !words.contains(link.word.as_str()) {
                    problems.push(format!(
                        "`{}` is not a link; use one of {all_words}.",
                        link.word
                    ));
                }
                scopes.push(&link.scope);
            }
        }
    }
    let has = |test: fn(&Scope) -> bool| scopes.iter().any(|scope| test(scope));
    let every = has(|scope| *scope == Scope::Every);
    let changes = has(|scope| *scope == Scope::Changes);
    let json = has(|scope| *scope == Scope::Json);
    let rows = has(|scope| matches!(scope, Scope::Rows(_)));
    let never = has(|scope| *scope == Scope::Never);
    let part = has(|scope| *scope == Scope::Part);
    problems.extend(part_problem(field, owned_id, part, never));
    if carries_changes && !changes {
        problems.push(
            "The field holds the changes of a record, whose values can name a person. Add \
             `#[person_in_changes]` and declare the members that name a person in \
             `bb_core::whose::IN_CHANGES` (ADR-0044)."
                .to_owned(),
        );
    }
    if changes && !carries_changes {
        problems.push(
            "The field carries `#[person_in_changes]`, but its type does not hold changes."
                .to_owned(),
        );
    }
    if holds_json && !json && !never {
        problems.push(
            "The field holds JSON, which can name a person. Add `#[person_in_json(…)]`, or \
             `#[names_no_person]` when it never does (ADR-0044)."
                .to_owned(),
        );
    }
    if json && !holds_json {
        problems.push(
            "The field carries `#[person_in_json(…)]`, but its type does not hold JSON.".to_owned(),
        );
    }
    if every && !names_a_person {
        problems.push(
            "The field carries `#[person(…)]`, but its type does not hold the ID of a person."
                .to_owned(),
        );
    }
    if rows && !refers_to_a_record {
        problems.push(
            "The field carries `#[person_for(…)]`, but its type is not a reference to a record."
                .to_owned(),
        );
    }
    if never && !refers_to_a_record && !holds_json && !owned_id {
        problems.push(
            "The field carries `#[names_no_person]`, but its type is neither a reference to a \
             record nor JSON."
                .to_owned(),
        );
    }
    if names_a_person && !fits_a_link(&field.type_names, &model.person_types) {
        problems.push(
            "The field holds the ID of a person inside a type no link can be put on (a tuple, or \
             a type other than Option, Vec, Box, BTreeSet and BTreeMap around it). Give the \
             person a field of its own (ADR-0044)."
                .to_owned(),
        );
    } else if names_a_person && !every {
        problems.push(format!(
            "The field names a person but has no link. Add `#[person(…)]` with one of {all_words} \
             to say what the person is to the row (ADR-0044)."
        ));
    }
    if refers_to_a_record && !names_a_person && !rows && !never {
        problems.push(
            "The field refers to a record, which can be a person. Add `#[person_for(…)]` for the \
             rows in which it names a person, or `#[names_no_person]` when it never does \
             (ADR-0044)."
                .to_owned(),
        );
    }
    problems
}

/// What is wrong with a field of the ID of a record that belongs to a person, or with
/// `#[person_for(part_of)]` (`part`) on a field of another type; `never` is `#[names_no_person]`.
fn part_problem(field: &Field, owned_id: bool, part: bool, never: bool) -> Option<String> {
    if part && !owned_id {
        return Some(
            "The field carries `#[person_for(part_of)]`, but its type is not the ID of a record \
             that belongs to a person (`ids_of_owned_records!` in `bb_core::personal`)."
                .to_owned(),
        );
    }
    // The record's own ID is no reference to it; every other field of such an ID says whether the
    // row is a part of the person's record.
    (owned_id && field.name != OWN_ID && !part && !never).then(|| {
        "The field holds the ID of a record that belongs to a person. Add \
         `#[person_for(part_of)]` when the row is a part of that record (it is then the \
         person's), or `#[names_no_person]` when it is not (ADR-0044)."
            .to_owned()
    })
}

/// The name of the field that holds a record's own ID.
const OWN_ID: &str = "id";

/// Whether the type is a person type, or one inside the containers a link sees through
/// (`bb_core::personal::NamesAPerson` is implemented for them): no tuple anywhere, and the
/// outermost type is a container or the person type itself.
fn fits_a_link(names: &[String], person_types: &[String]) -> bool {
    !names.iter().any(|name| name == TUPLE)
        && names.first().is_some_and(|outer| {
            person_types.contains(outer) || CONTAINERS.contains(&outer.as_str())
        })
}

#[cfg(test)]
mod tests;
