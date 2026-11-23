//! Where the text of a change or of JSON names a person by ID (ADR-0044, decision 1, amendment
//! *the index of the walk*): one row for each person and each row of a table that holds changes
//! (`LinkScope::Changes`) or JSON (`LinkScope::Json`) whose text holds the person's ID. The walk of
//! a person's data finds those rows by this index instead of reading the whole table, and still
//! decides by the text, so it finds exactly what it found before. The store writes a row of the
//! index in the change that writes the row it points to, and the row of the index goes with that
//! row.

use crate::id::{EntityId, EventId, IssueId, UserId};
use crate::personal::{Entity, EntityMarks};

/// A row of a table keyed by one ID whose changes or JSON hold the ID of a person.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RowNamingPerson {
    /// The key of the row.
    #[not_personal]
    #[names_no_person]
    pub row: EntityId,
    /// The person whose ID the text of the row holds.
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(subject)]
    pub person: UserId,
}

/// A row of the history of an issue, keyed by its event and its issue, whose changes hold the ID
/// of a person.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct HistoryRowNamingPerson {
    /// The event of the row.
    #[not_personal]
    pub event: EventId,
    /// The issue of the row.
    #[not_personal]
    pub issue: IssueId,
    /// The person whose ID the changes of the row hold.
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(subject)]
    pub person: UserId,
}

/// Whether `marks` are those of the index of the walk: a copy of what the rows it points to say,
/// which the walk reads to find them and never hands over as data of its own (like the full-text
/// index, M5-F52).
#[must_use]
pub fn is_index_of_the_walk(marks: &EntityMarks) -> bool {
    [
        RowNamingPerson::MARKS.name(),
        HistoryRowNamingPerson::MARKS.name(),
    ]
    .contains(&marks.name())
}

/// The characters of an ID as text (`0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c`).
const ID_TEXT_LENGTH: usize = 36;

/// Where the hyphens of an ID written as text stand.
const ID_HYPHENS: [usize; 4] = [8, 13, 18, 23];

/// Every ID written as text inside `text`, wherever it stands (also inside a longer word), with
/// capitals not counting: the IDs a search for the text of an ID anywhere in `text` (`LIKE
/// '%<id>%'`) could find. Each as its 16 bytes.
#[must_use]
pub fn ids_in(text: &str) -> Vec<[u8; 16]> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    if bytes.len() < ID_TEXT_LENGTH {
        return found;
    }
    for start in 0..=bytes.len() - ID_TEXT_LENGTH {
        if let Some(id) = bytes.get(start..start + ID_TEXT_LENGTH).and_then(id_at) {
            found.push(id);
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// The ID that `window` (36 bytes) writes, if it writes one.
fn id_at(window: &[u8]) -> Option<[u8; 16]> {
    let mut id = [0_u8; 16];
    let mut digits = 0_usize;
    for (at, byte) in window.iter().enumerate() {
        if ID_HYPHENS.contains(&at) {
            if *byte != b'-' {
                return None;
            }
            continue;
        }
        let value = char::from(*byte).to_digit(16)?;
        let slot = id.get_mut(digits / 2)?;
        *slot = (*slot << 4) | u8::try_from(value).ok()?;
        digits += 1;
    }
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn priv_03_every_id_in_a_text_is_found_wherever_it_stands() {
        let mut ids = SequentialIds::default();
        let lea = UserId::generate(&mut ids);
        let ada = UserId::generate(&mut ids);
        let text = format!(
            r#"[{{"field":"assigned-to","after":["{lea}"]}},{{"note":"x{}y"}}]"#,
            ada.to_string().to_uppercase()
        );
        assert_eq!(ids_in(&text), {
            let mut both = vec![lea.to_bytes(), ada.to_bytes()];
            both.sort_unstable();
            both
        });
        assert!(ids_in("0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4").is_empty());
        assert!(ids_in("0190f4c2_7a3e-7c1d-9b52-3d6e8f1a2b4c").is_empty());
        assert!(ids_in("").is_empty());
        assert_eq!(ids_in(&format!("{lea}{lea}")), vec![lea.to_bytes()]);
    }

    #[test]
    fn priv_03_the_index_of_the_walk_is_its_two_kinds_and_no_other() {
        assert!(is_index_of_the_walk(&RowNamingPerson::MARKS));
        assert!(is_index_of_the_walk(&HistoryRowNamingPerson::MARKS));
        assert!(!is_index_of_the_walk(&crate::job::Job::MARKS));
    }
}
