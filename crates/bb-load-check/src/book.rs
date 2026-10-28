//! The book of a run: what each person kept, was refused and read. The people write it while they
//! work (the test of the server in memory, the driver of `cargo xtask load` as a JSON file); the
//! rules that judge it are in [`crate::judge`] and nowhere else.

use serde::{Deserialize, Serialize};

/// An issue the people work on.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Work {
    /// The ID of the issue.
    pub id: String,
    /// Its readable ID.
    pub key: String,
    /// The ID of the Guest who reported it; none for an issue the team made. A Guest hears only
    /// of the issues they reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporter: Option<String>,
}

/// A change the server kept.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Kept {
    /// The readable ID of the issue.
    pub key: String,
    /// The version the issue had after the change.
    pub version: i64,
    /// The description, when the change was a save of the description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The title, when the change was a new title (M3-F93).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The key of the status, when the change was a status change under the workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
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

/// A search a person made while others changed data, and what they were told (architecture §11,
/// "Searches while others change data").
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Searched {
    /// The ID of the person.
    pub holder: String,
    /// The text of the search.
    pub query: String,
    /// Whether the search asks for every issue the person reads, which no change of the run alters.
    pub everything: bool,
    /// Whether the search reads the own field that Guests may not see.
    pub reads_hidden: bool,
    /// The readable IDs of the issues the first page held.
    pub found: Vec<String>,
    /// Whether there was a page after the first.
    pub more: bool,
    /// What the count line said, for a search that asked for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// The code of the refusal, when the search was refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
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
    /// The readable ID of the issue of every file that was added.
    #[serde(default)]
    pub files: Vec<String>,
    /// The IDs of the Guests of the run: what they are told is judged by what a Guest may see.
    #[serde(default)]
    pub guests: Vec<String>,
    /// The keys of the own fields the run hid from Guests.
    #[serde(default)]
    pub hidden_from_guests: Vec<String>,
    /// The searches people made.
    #[serde(default)]
    pub searched: Vec<Searched>,
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

    /// A file of the book in the shape the driver writes (its types are generated from the ones
    /// above, `cargo xtask gen load-book`): read, and written back unchanged.
    #[test]
    fn test_concurrent_the_book_of_the_driver_is_read_and_written_back_unchanged() {
        let example = serde_json::json!({
            "issues": [{"id": "00000000-0000-4000-8000-000000000001", "key": "LOAD-1", "reporter": "00000000-0000-4000-8000-000000000004"}],
            "kept": [
                {"key": "LOAD-1", "version": 2},
                {"key": "LOAD-1", "version": 3, "text": "A text that was kept"},
                {"key": "LOAD-1", "version": 4, "title": "A title", "status": "in-progress"}
            ],
            "comments": ["LOAD-1"],
            "seen": [{"person": 1, "key": "LOAD-1", "version": 2}],
            "drafted": [
                {
                    "holder": "00000000-0000-4000-8000-000000000002",
                    "key": "LOAD-1",
                    "text": "A text that was refused",
                    "ending": "refused",
                    "found_as_draft": true
                },
                {
                    "holder": "00000000-0000-4000-8000-000000000003",
                    "key": "LOAD-1",
                    "text": "A text that was left behind",
                    "ending": "vanished"
                }
            ],
            "broken": [],
            "files": ["LOAD-1"],
            "guests": ["00000000-0000-4000-8000-000000000004"],
            "hidden_from_guests": ["00000000-0000-4000-8000-000000000005"],
            "searched": [
                {
                    "holder": "00000000-0000-4000-8000-000000000002",
                    "query": "",
                    "everything": true,
                    "reads_hidden": false,
                    "found": ["LOAD-1"],
                    "more": false,
                    "count": 1
                },
                {
                    "holder": "00000000-0000-4000-8000-000000000004",
                    "query": "internalNote is x",
                    "everything": false,
                    "reads_hidden": true,
                    "found": [],
                    "more": false,
                    "refused": "search.invalid"
                }
            ]
        });
        let book: Book = serde_json::from_value(example.clone()).unwrap();
        assert_eq!(book.kept.len(), 3);
        assert_eq!(book.searched.len(), 2);
        assert_eq!(book.drafted[1].ending, DraftEnd::Vanished);
        assert_eq!(serde_json::to_value(&book).unwrap(), example);
    }
}
