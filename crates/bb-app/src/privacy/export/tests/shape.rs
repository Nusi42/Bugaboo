//! What of a row goes into the file (ADR-0044, decision 3).

use std::collections::BTreeMap;

use bb_core::access_key::AccessKey;
use bb_core::comment::Comment;
use bb_core::id::{SequentialIds, UserId};
use bb_core::personal::{Entity as _, Link};
use bb_policy::PersonRef;

use super::*;
use crate::privacy::walk::{Cell, FoundRow, Value};

fn cell(column: &str, field: &'static str, value: Value) -> Cell {
    Cell {
        column: column.to_owned(),
        field,
        value,
    }
}

fn comment_by(author: UserId, links: Vec<Link>) -> FoundRow {
    FoundRow {
        links,
        cells: vec![
            cell("author", "author", Value::Person(author)),
            cell("text", "text", Value::Text("Ask @someone".to_owned())),
        ],
    }
}

fn fields(row: &ShapedRow) -> Vec<&str> {
    row.cells.iter().map(|cell| cell.field).collect()
}

#[test]
fn priv_03_a_text_the_person_wrote_is_kept_and_a_text_of_others_is_not() {
    let mut ids = SequentialIds::default();
    let (person, other) = (UserId::generate(&mut ids), UserId::generate(&mut ids));
    let seen = BTreeMap::from([(
        other,
        PersonRef::Role {
            role: "Team member".to_owned(),
        },
    )]);
    let own = shape_row(
        comment_by(person, vec![Link::Author]),
        &Comment::MARKS,
        person,
        &seen,
    );
    assert_eq!(fields(&own), ["author", "text"]);
    assert_eq!(
        own.cells[1].value,
        Shown::Text {
            text: "Ask @someone".to_owned(),
            written: true
        }
    );
    let theirs = shape_row(
        comment_by(other, vec![Link::Recipient]),
        &Comment::MARKS,
        person,
        &seen,
    );
    assert_eq!(
        fields(&theirs),
        ["author"],
        "the text of another is not copied"
    );
    assert_eq!(
        theirs.cells[0].value,
        Shown::Person(PersonRef::Role {
            role: "Team member".to_owned()
        }),
        "another person as the person sees them now"
    );
}

#[test]
fn priv_03_somebody_the_person_may_not_see_is_someone() {
    let mut ids = SequentialIds::default();
    let (person, hidden) = (UserId::generate(&mut ids), UserId::generate(&mut ids));
    let shaped = shape_row(
        comment_by(hidden, vec![Link::Recipient]),
        &Comment::MARKS,
        person,
        &BTreeMap::new(),
    );
    assert_eq!(shaped.cells[0].value, Shown::Person(PersonRef::Someone));
}

#[test]
fn priv_03_no_credential_goes_into_the_file() {
    let mut ids = SequentialIds::default();
    let person = UserId::generate(&mut ids);
    let row = FoundRow {
        links: vec![Link::Holder],
        cells: vec![
            cell("owner", "owner", Value::Person(person)),
            cell(
                "secret_hash",
                "secret_hash",
                Value::Bytes("ab12".to_owned()),
            ),
            cell("name", "name", Value::Text("Laptop".to_owned())),
        ],
    };
    let shaped = shape_row(row, &AccessKey::MARKS, person, &BTreeMap::new());
    assert!(!fields(&shaped).contains(&"secret_hash"));
    assert!(fields(&shaped).contains(&"name"));
}
