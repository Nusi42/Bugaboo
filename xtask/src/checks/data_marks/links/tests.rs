//! The good and bad examples of the links of a person in DATA-MARKS.

use super::*;
use crate::source::Source;

const VOCABULARY: &str = "words! { pub enum Category { Identity => \"identity\" } }
words! { pub enum Basis { Contract => \"contract\" } }
";

const LINKS: &str = "words! { pub enum Link { Author => \"author\", Subject => \"subject\" } }
names_a_person!(crate::id::UserId, crate::event::Actor);
refers_to_a_record!(crate::id::EntityId, crate::event::EntityRef);
carries_changes!(crate::event::FieldChange);
";

const MARK: &str = "#[personal(category = \"identity\", basis = \"contract\")]";

/// The findings for `entity` in `crates/bb-core/src/note.rs`, beside `more` in another file of
/// `bb-core` (aliases and newtypes).
fn findings_with(entity: &str, more: &str) -> String {
    let sources: Vec<Source> = vec![
        (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
        (privacy::LINKS_PATH.to_owned(), LINKS.to_owned()),
        ("crates/bb-core/src/note.rs".to_owned(), entity.to_owned()),
        ("crates/bb-core/src/owner.rs".to_owned(), more.to_owned()),
    ];
    evaluate_links(&privacy::read_model(&sources).unwrap())
        .into_iter()
        .map(|f| f.message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn findings(entity: &str) -> String {
    findings_with(entity, "")
}

fn note(fields: &str) -> String {
    format!("#[derive(Entity)]\npub struct Note {{\n{fields}}}\n")
}

#[test]
fn good_example_every_field_that_names_a_person_has_a_link() {
    let entity = note(&format!(
        "    {MARK}\n    #[person(author)]\n    pub author: UserId,\n    {MARK}\n    #[person(subject)]\n    pub about: Option<Vec<crate::id::UserId>>,\n    #[not_personal]\n    #[person_for(record, link = \"subject\")]\n    pub entity: EntityRef,\n    {MARK}\n    #[person_for(record, kind = \"entity_kind\", link = \"subject\")]\n    pub entity_id: EntityId,\n    {MARK}\n    #[person_for(field_type = \"person\", key = \"field\", link = \"subject\")]\n    #[person_for(field_type = \"people\", key = \"field\", link = \"subject\")]\n    pub value: Option<EntityId>,\n    {MARK}\n    #[person_in_changes(subject)]\n    pub changes: Vec<FieldChange>,\n    #[not_personal]\n    #[names_no_person]\n    pub record: EntityId,\n    {MARK}\n    #[person_email(subject)]\n    pub email: String,\n    {MARK}\n    pub text: String,\n"
    ));
    assert_eq!(findings(&entity), "");
}

#[test]
fn bad_example_a_field_that_names_a_person_without_a_link() {
    let entity = note(&format!(
        "    {MARK}\n    pub author: UserId,\n    {MARK}\n    pub actor: Actor,\n"
    ));
    let all = findings(&entity);
    assert_eq!(
        all.matches("names a person but has no link").count(),
        2,
        "{all}"
    );
}

#[test]
fn bad_example_a_reference_to_a_record_that_does_not_say_whether_it_names_a_person() {
    let entity = note(
        "    #[not_personal]\n    pub record: EntityId,\n    #[not_personal]\n    pub entity: Option<EntityRef>,\n",
    );
    let all = findings(&entity);
    assert_eq!(
        all.matches("refers to a record, which can be a person")
            .count(),
        2,
        "{all}"
    );
}

#[test]
fn bad_example_an_alias_or_a_newtype_of_a_person_without_a_link() {
    let entity = note(&format!(
        "    {MARK}\n    pub owner: Owner,\n    {MARK}\n    pub keeper: Keeper,\n"
    ));
    let all = findings_with(
        &entity,
        "pub type Owner = Option<UserId>;\npub struct Keeper(pub Owner);\n",
    );
    assert_eq!(
        all.matches("names a person but has no link").count(),
        2,
        "{all}"
    );
}

#[test]
fn bad_example_a_person_inside_a_type_no_link_can_be_put_on() {
    let entity = note(&format!(
        "    {MARK}\n    #[person(author)]\n    pub pair: Vec<(UserId, u8)>,\n    {MARK}\n    pub held: Holder<UserId>,\n"
    ));
    let all = findings(&entity);
    assert_eq!(
        all.matches("inside a type no link can be put on").count(),
        2,
        "{all}"
    );
}

#[test]
fn bad_example_a_link_with_an_unknown_word_or_on_a_field_that_does_not_fit() {
    let entity = note(&format!(
        "    {MARK}\n    #[person(writer)]\n    pub author: UserId,\n    {MARK}\n    #[person(author)]\n    pub text: String,\n    {MARK}\n    #[names_no_person]\n    pub name: String,\n    {MARK}\n    #[person_for(record)]\n    pub entity: EntityRef,\n    {MARK}\n    #[person_in_changes(subject)]\n    pub words: String,\n"
    ));
    let all = findings(&entity);
    assert!(all.contains("`writer` is not a link"), "{all}");
    assert!(
        all.contains("its type does not hold the ID of a person"),
        "{all}"
    );
    assert!(
        all.contains("its type is not a reference to a record"),
        "{all}"
    );
    assert!(all.contains("needs link"), "{all}");
    assert!(all.contains("its type does not hold changes"), "{all}");
}

#[test]
fn bad_example_the_words_of_the_links_are_missing() {
    let sources: Vec<Source> = vec![(privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned())];
    let found = evaluate_links(&privacy::read_model(&sources).unwrap());
    assert!(found[0].message.contains("`Link`"), "{}", found[0].message);
}

#[test]
fn bad_example_changes_without_the_link_of_their_people() {
    let entity = note(&format!("    {MARK}\n    pub changes: Vec<FieldChange>,\n"));
    let all = findings(&entity);
    assert!(
        all.contains("holds the changes of a record, whose values can name a person"),
        "{all}"
    );
}
