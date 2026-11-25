//! What the release use cases take and give back (ARCH-EVENTS: inputs that change something are
//! named after what they do; a read ends in `Query`).

use bb_core::given::SetTo;
use bb_core::id::ReleaseId;
use bb_core::project_release::{Release, ReleaseState};
use bb_policy::Masked;
use jiff::civil::Date;
use serde::Serialize;

use crate::projects::ProjectRef;

/// Make a release in a project.
pub struct CreateRelease {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it is made in, by its ID or short name.
    pub project: String,
    /// What it is called.
    pub name: String,
    /// Its *Release date*; none when left out.
    pub release_date: Option<Date>,
}

/// Change a release: what is `Some` is changed.
pub struct ChangeRelease {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it was made in, by its ID or short name.
    pub project: String,
    /// The release.
    pub release: ReleaseId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new *Release date*; `SetTo::Nothing` takes it away.
    pub release_date: Option<SetTo<Date>>,
    /// A new state: *Released* keeps the day as *Released on*, *Retired* stops offering it.
    pub state: Option<ReleaseState>,
    /// Why a release goes back from *Released* to *Planned*: needed then, kept in the history.
    pub reason: Option<String>,
}

/// Put a release that no issue names into the trash, or take it out again (*Undo*).
pub struct DeleteRelease {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it was made in, by its ID or short name.
    pub project: String,
    /// The release.
    pub release: ReleaseId,
}

/// Ask for the releases offered in a project: those made in it and in the projects above it.
pub struct ReleasesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or short name.
    pub project: String,
    /// Where the page starts.
    pub cursor: Option<String>,
    /// How many to give at most.
    pub limit: u32,
}

/// A release as a person who sees its project sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct ReleaseView {
    /// The ID.
    #[masked(open)]
    pub id: ReleaseId,
    /// What it is called.
    #[masked(open)]
    pub name: String,
    /// The project it was made in.
    #[masked(nested)]
    pub project: ProjectRef,
    /// *Release date*.
    #[masked(open)]
    pub release_date: Option<Date>,
    /// Its state.
    #[masked(open)]
    pub state: ReleaseState,
    /// *Released on*.
    #[masked(open)]
    pub released_on: Option<Date>,
    /// Whether the person who asks may make, change, mark and delete it (*Manage releases* in its
    /// project).
    #[masked(open)]
    pub may_manage: bool,
    /// Counts up with every change.
    #[masked(open)]
    pub version: i64,
}

impl ReleaseView {
    /// The view of a release made in `project`.
    pub(crate) fn of(release: &Release, project: ProjectRef, may_manage: bool) -> Self {
        Self {
            id: release.id,
            name: release.name.clone(),
            project,
            release_date: release.release_date,
            state: release.state,
            released_on: release.released_on,
            may_manage,
            version: release.version,
        }
    }
}

/// A page of the releases offered in a project, by *Release date* (none last), then by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct ReleasePage {
    /// The releases.
    #[masked(nested)]
    pub items: Vec<ReleaseView>,
    /// The cursor of the next page, if there is one.
    #[masked(open)]
    pub next_cursor: Option<String>,
}

/// A release an issue names, as the viewer of the issue sees it: only a release of a project the
/// viewer sees is shown (ADR-0049, decision 7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct ReleaseRef {
    /// The ID.
    #[masked(open)]
    pub id: ReleaseId,
    /// What it is called.
    #[masked(open)]
    pub name: String,
    /// Its state.
    #[masked(open)]
    pub state: ReleaseState,
    /// *Release date*.
    #[masked(open)]
    pub release_date: Option<Date>,
    /// Whether it is offered where the issue is now: `false` for an issue that moved out of the
    /// tree of the release's project (ADR-0049, decision 1).
    #[masked(open)]
    pub offered: bool,
}

impl ReleaseRef {
    /// The release as an issue in the project of `chain` names it.
    #[must_use]
    pub fn of(release: &Release, chain: &[bb_core::id::ProjectId]) -> Self {
        Self {
            id: release.id,
            name: release.name.clone(),
            state: release.state,
            release_date: release.release_date,
            offered: release.offered_in(chain),
        }
    }
}
