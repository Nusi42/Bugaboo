//! What the saved-search use cases take and give back (QRY-06, ADR-0034).

use bb_core::id::{GroupId, ProjectId, SavedSearchId};
use bb_core::saved_search::{Presentation, SearchSet, ShareKind, ShareTarget};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::access::ProjectLabel;

/// Save a search: private, and the person's own.
pub struct SaveSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called.
    pub name: String,
    /// The search text.
    pub text: String,
    /// How its results are shown.
    pub presentation: Presentation,
}

/// Change a saved search the person made. What is `None` stays as it is.
pub struct ChangeSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
    /// The version the person looked at; a change made meanwhile stops this one.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new search text.
    pub text: Option<String>,
    /// A new way to show the results.
    pub presentation: Option<Presentation>,
}

/// Share a saved search with a group or a project, or stop.
pub struct ShareSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
    /// The group or the project.
    pub target: ShareTarget,
}

/// Stop sharing a saved search with a group or a project.
pub struct StopSharingSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
    /// The group or the project.
    pub target: ShareTarget,
}

/// Put a saved search into the trash.
pub struct DeleteSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
}

/// Take a saved search out of the trash (*Undo*).
pub struct RestoreSearch {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
}

/// *Save a copy*: a private saved search of the person with the text and the presentation of one
/// they see.
pub struct SaveCopy {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search to copy.
    pub search: SavedSearchId,
    /// What the copy is called; the name of the original when `None`.
    pub name: Option<String>,
}

/// A page of the saved searches of one scope.
pub struct SearchesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Which saved searches.
    pub scope: SearchSet,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many the page holds at most.
    pub limit: u32,
}

/// One saved search, opened.
pub struct SavedSearchQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
}

/// Which groups or projects the person may share a saved search with.
pub struct ShareChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Groups or projects.
    pub kind: ShareKind,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many the page holds at most.
    pub limit: u32,
}

/// Choose the saved search a project shows as its list, or none (the built-in default).
pub struct SetProjectList {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or short name as it stands in the address.
    pub project: String,
    /// The saved search, shared with the project; `None` for the built-in default.
    pub search: Option<SavedSearchId>,
}

/// The list a project shows.
pub struct ProjectListQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or short name as it stands in the address.
    pub project: String,
}

/// Whom a saved search is shared with, as its maker sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShareView {
    /// A group.
    Group {
        /// The group.
        id: GroupId,
        /// Its name.
        name: String,
    },
    /// A project, named only while the maker may know it.
    Project {
        /// The project.
        id: ProjectId,
        /// Its name, or *a project you can't see*.
        label: ProjectLabel,
    },
}

/// A saved search as one person sees it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedSearchView {
    /// The ID, in the address `/s/<id>`.
    pub id: SavedSearchId,
    /// What it is called.
    pub name: String,
    /// The search text.
    pub text: String,
    /// How its results are shown.
    pub presentation: Presentation,
    /// Who made it, as the viewer sees people who write.
    pub maker: PersonRef,
    /// Whether the viewer made it.
    pub mine: bool,
    /// Whom it is shared with; only its maker is told, for everybody else this is empty.
    pub shared_with: Vec<ShareView>,
    /// When it was last changed.
    pub updated_at: Timestamp,
    /// Counts up with every change, for the check that nobody changed it meanwhile.
    pub version: i64,
}

impl Masked for SavedSearchView {}

/// What taking a saved search out of the trash answers: the search, when it is one the person
/// may open. Somebody who may only take it out (the maker's right to *Undo* is theirs; a person who
/// may change organization settings may too) gets nothing of its name or text.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RestoredSearch {
    /// The saved search as the person sees it; none when it is not theirs to open.
    pub search: Option<SavedSearchView>,
}

impl Masked for RestoredSearch {}

/// A page of saved searches.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedSearchPage {
    /// The saved searches.
    pub items: Vec<SavedSearchView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for SavedSearchPage {}

/// A group or a project the person may share with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShareChoice {
    /// The group or project.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The short name of a project; none for a group.
    pub short_name: Option<String>,
}

/// A page of the groups or projects the person may share with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShareChoicePage {
    /// The choices, by name.
    pub items: Vec<ShareChoice>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for ShareChoicePage {}

/// The list a project shows: a saved search, or none when it shows the built-in default.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProjectList {
    /// The saved search the viewer may open, if the project has one.
    pub search: Option<SavedSearchView>,
}

impl Masked for ProjectList {}
