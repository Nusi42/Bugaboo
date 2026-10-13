//! The book of a run: what each person kept, was refused and read. The people write it while they
//! work (the test of the server in memory, the driver of `cargo xtask load` as a JSON file); the
//! rules that judge it are in [`crate::judge`] and nowhere else.

use serde::{Deserialize, Serialize};

/// An issue the people work on.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Work {
    /// The ID of the issue.
    pub id: String,
    /// Its readable ID.
    pub key: String,
}

/// A change the server kept.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Kept {
    /// The readable ID of the issue.
    pub key: String,
    /// The version the issue had after the change.
    pub version: i64,
    /// The description, when the change was a save of the description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A version of an issue that a person read.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Seen {
    /// Who read it (the number of the person in the run).
    pub person: u32,
    /// The readable ID of the issue.
    pub key: String,
    /// The version they read.
    pub version: i64,
}

/// How the text of a draft came to an end.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftEnd {
    /// The save went through: the text is the description now.
    Saved,
    /// The save was refused (a lost lock, a take-over, a changed text): the text stays as the draft.
    Refused,
    /// The editor went away without letting go: the heartbeat draft stays.
    Vanished,
}

/// The text of a draft that somebody sent with a heartbeat, and what became of it.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Drafted {
    /// The ID of the person.
    pub holder: String,
    /// The readable ID of the issue.
    pub key: String,
    /// The text.
    pub text: String,
    /// What became of it.
    pub ending: DraftEnd,
    /// For a refused save: whether the person found the text as their draft at once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found_as_draft: Option<bool>,
}

/// Everything the people did that counts, in the order it happened to each of them.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Book {
    /// The issues of the run.
    pub issues: Vec<Work>,
    /// The changes the server kept.
    pub kept: Vec<Kept>,
    /// The readable ID of the issue of every comment that was written.
    pub comments: Vec<String>,
    /// The versions people read.
    pub seen: Vec<Seen>,
    /// Every text that was sent as a draft.
    pub drafted: Vec<Drafted>,
    /// What the people saw break a rule while they worked.
    pub broken: Vec<String>,
}

impl Book {
    /// The readable ID of the issue with this ID, or the ID itself.
    #[must_use]
    pub fn name_of<'a>(&'a self, id: &'a str) -> &'a str {
        self.issues
            .iter()
            .find(|work| work.id == id)
            .map_or(id, |work| work.key.as_str())
    }
}

/// The last draft each person sent for each issue, in the order the people sent them. What became
/// of it is what the database must still show, unless a later save went through.
#[must_use]
pub(crate) fn last_drafts(book: &Book) -> Vec<&Drafted> {
    let mut last: Vec<&Drafted> = Vec::new();
    for drafted in &book.drafted {
        match last
            .iter()
            .position(|each| each.holder == drafted.holder && each.key == drafted.key)
        {
            Some(at) => last[at] = drafted,
            None => last.push(drafted),
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file that the test of the driver reads too (`web/tooling/load/book.test.ts`): the two
    /// sides write the book in the same shape.
    const EXAMPLE: &str = include_str!("../../../tools/dev/load-book.example.json");

    #[test]
    fn test_concurrent_the_book_of_the_driver_reads_and_writes_the_same_shape() {
        let book: Book = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(book.kept.len(), 2);
        assert_eq!(book.drafted[1].ending, DraftEnd::Vanished);
        let written = serde_json::to_value(&book).unwrap();
        let example: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(written, example);
    }
}
