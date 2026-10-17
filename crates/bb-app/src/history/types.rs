//! What the history takes and gives back (ISS-06, SEC-06).
//!
//! A row of the history is shaped for one reader: the changes to fields the reader may not see are
//! not in it (they are left out, not shown as hidden), the people are shown as the kinds *Authors*,
//! *Assigned to* and *Reported by* say, and an issue another row names is a line only when the
//! reader may see it.

use bb_core::history::HistoryTab;
use bb_core::id::EventId;
use bb_core::own_fields::FieldType;
use bb_core::relation::Direction;
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use crate::comments::CommentView;
use crate::projects::ProjectRef;
use crate::relations::IssueLine;

/// The history of an issue.
pub struct HistoryQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// Which rows.
    pub tab: HistoryTab,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many rows a page holds at most.
    pub limit: u32,
}

/// A value in a change, as the reader may see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "shown", rename_all = "snake_case")]
pub enum ChangeValue {
    /// A word, a name or a number.
    Text {
        /// The text.
        text: String,
    },
    /// The names of the choices of a pick field; the screen joins them.
    Names {
        /// The names, in the order of the choices.
        names: Vec<String>,
    },
    /// People, each as the kind of the field says.
    People {
        /// The people.
        people: Vec<PersonRef>,
    },
    /// A value that a screen words itself, as the kind of its own field says: yes or no, a
    /// length of time, a number, a day (the JSON the issue page gets).
    Typed {
        /// The kind of the field.
        kind: FieldType,
        /// The value.
        value: Value,
    },
    /// An issue the reader may see.
    Issue {
        /// The issue.
        issue: IssueLine,
    },
    /// A project the reader may see.
    Project {
        /// The project.
        project: ProjectRef,
    },
}

/// One field that changed, with the values the reader may see. A long text (the description) holds
/// no values: the row says that it changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldChangeView {
    /// The key of the field (`status`, `assigned-to`, `parent`).
    pub field: String,
    /// The name of an own field; `None` for a field of the catalogue and the other members.
    pub name: Option<String>,
    /// What it was; `None` when it had no value, or for a long text.
    pub before: Option<ChangeValue>,
    /// What it is; `None` when it has no value now, or for a long text.
    pub after: Option<ChangeValue>,
}

/// What happened to a file of an issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEvent {
    /// The file was added.
    Added,
    /// The file was put into the trash.
    Deleted,
    /// The file was brought back from the trash (*Undo*).
    Restored,
}

impl FileEvent {
    /// The word the API says it with.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Restored => "restored",
        }
    }
}

/// What a row of the history says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryWhat {
    /// A comment, as the reader sees it now.
    Comment {
        /// The comment.
        comment: CommentView,
    },
    /// Fields of the issue that changed, only those the reader may see.
    Changes {
        /// The fields.
        fields: Vec<FieldChangeView>,
    },
    /// A file of the issue was added, put into the trash or brought back.
    File {
        /// What happened.
        event: FileEvent,
        /// The name of the file.
        name: String,
    },
    /// A relation to another issue was added or taken away.
    Relation {
        /// Whether it was added (otherwise taken away).
        added: bool,
        /// Which end of the relation this issue is.
        direction: Direction,
        /// What this end reads (*blocks*, *is blocked by*).
        name: String,
        /// The issue at the other end.
        issue: IssueLine,
    },
}

/// One row of the history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HistoryItem {
    /// The event the row was made from.
    pub event: EventId,
    /// When it happened.
    pub at: Timestamp,
    /// Who did it, as the kind *Authors* says for this reader; nobody for Bugaboo itself.
    pub by: Option<PersonRef>,
    /// What happened.
    pub what: HistoryWhat,
}

/// A page of the history, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HistoryPage {
    /// The rows.
    pub items: Vec<HistoryItem>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for HistoryPage {}
