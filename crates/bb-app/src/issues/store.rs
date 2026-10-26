//! The port of the issues use cases: what they need from storage beyond the projects port.
//!
//! Like [`ProjectStore`], reads take `&self` and **writes** are associated functions that run
//! inside `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).
//! A read that is also needed inside a change is written once with `queries!`.

use std::future::Future;

use bb_core::event::DomainEvent;
use bb_core::id::{IssueId, IssueTypeId, ProjectId};
use bb_core::issue::{AssignedPerson, Issue, IssueAlias, IssueIcon, IssueType};
use bb_core::readable_id::IssueCode;
use bb_core::workflow::StatusKey;
use jiff::Timestamp;

use crate::projects::ProjectStore;
use crate::store_macros::{queries, reads, writes};

/// What counting the version of an issue up does with an issue that is in the trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InTrash {
    /// The issue is not there for this change.
    NotFound,
    /// The issue is counted up like any other.
    Counted,
}

/// The status an issue has after a move into another project, and the closed date that goes with
/// it.
#[derive(Clone, Debug)]
pub struct MovedStatus {
    /// The status.
    pub status: StatusKey,
    /// When the issue was closed, if it is in a status of the group *Done*.
    pub closed_at: Option<Timestamp>,
}

/// An issue found by a readable ID: the issue, and whether the code was an old one of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueFound {
    /// The issue (also one in the trash: the use case decides what to say about it).
    pub issue: Issue,
    /// The code named an old code of the issue.
    pub old_code: bool,
}

/// What the issues use cases need from storage.
pub trait IssueStore: ProjectStore + 'static {
    queries! {
        /// Every issue type, in the order they are offered in (then by ID).
        fn issue_types() -> Vec<IssueType> = issue_types_in;
        /// The issue with this ID, also one in the trash.
        fn issue(id: IssueId) -> Option<Issue> = issue_in;
        /// The issue that has this code in the project now or had it once.
        fn issue_at(project: ProjectId, code: IssueCode) -> Option<IssueFound> = issue_at_in;
        /// Whether a code is given out in the project: an issue has it, or had it once.
        fn issue_code_taken(project: ProjectId, code: IssueCode) -> bool = issue_code_taken_in;
        /// The people an issue is assigned to, in the order they were put there.
        fn assigned_people(issue: IssueId) -> Vec<AssignedPerson> = assigned_people_in;
        /// Every issue of a project, also those in the trash, with their IDs and codes.
        fn issue_codes(project: ProjectId) -> Vec<(IssueId, IssueCode)> = issue_codes_in;
    }

    reads! {
        /// The issue type with this ID.
        fn issue_type(id: IssueTypeId) -> Option<IssueType>;
        /// The people the issues are assigned to, for a page of issues at once.
        fn assigned_people_of(issues: &[IssueId]) -> Vec<AssignedPerson>;
    }

    writes! {
        /// Adds an issue type.
        fn insert_issue_type(issue_type: &IssueType) -> ();
        /// Changes the name, icon and retired date of an issue type, counts its version up, and
        /// says whether the type was still at `version`.
        fn update_issue_type(
            id: IssueTypeId,
            name: &str,
            icon: IssueIcon,
            retired_at: Option<Timestamp>,
            version: i64
        ) -> bool;
        /// Adds an issue with the people it is assigned to.
        fn insert_issue(issue: &Issue, assigned_people: &[AssignedPerson]) -> ();
        /// Writes the type, title, description, status, priority, update date and closed date of
        /// `issue`, counts its version up, and says whether it was still at `version`.
        fn update_issue(issue: &Issue, version: i64) -> bool;
        /// Puts an issue into another project with a code of that project and the status and
        /// closed date it has there, sets the date it changed, counts its version up, and says
        /// whether the issue was still at `version`. The old code is kept with
        /// `insert_issue_alias`.
        fn move_issue(
            id: IssueId,
            project: ProjectId,
            code: &IssueCode,
            moved: &MovedStatus,
            version: i64,
            now: Timestamp
        ) -> bool;
        /// Replaces the people an issue is assigned to.
        fn replace_assigned_people(issue: IssueId, assigned_people: &[AssignedPerson]) -> ();
        /// Puts an issue into the trash (a date) or takes it out (`None`), counts its version up
        /// and sets the date it changed (see [`Self::count_issue_up`]), and says whether the issue
        /// was there.
        fn set_issue_deleted(id: IssueId, deleted_at: Option<Timestamp>, now: Timestamp) -> bool;
        /// Gives out the next counting number of a project: the number after the last one given
        /// out, which is stored. Two writers wait for each other, so no number is skipped or
        /// given twice.
        fn next_counting_number(project: ProjectId) -> i64;
        /// Gives an issue a new code (the old one is kept with `insert_issue_alias`) and counts its
        /// version up, so a person who looked at the issue sees that it changed. It does not set the
        /// date the issue changed: a renewal of IDs is not an edit by a person, and must not move
        /// the issues up in *recently updated*.
        fn set_issue_code(id: IssueId, code: &IssueCode) -> ();
        /// Counts the version of an issue up without changing anything else of it: the version
        /// goes up by one and the date it changed is `now`, as in every other change of an issue
        /// that a person makes. With `only_at` the issue must still have that version (a change
        /// that took the version the person looked at); without, whatever the version is (a tag or
        /// a label, which take none). `in_trash` says what an issue in the trash is: a change a
        /// person asks for finds it not there, while a change of a value that the issue holds (a
        /// choice combined into another) counts it up too, so that taking it out of the trash shows
        /// the right version. Gives the version the issue has now; `None` when the issue is not
        /// there, is in the trash and `in_trash` says not found, or has another version than
        /// `only_at`.
        fn count_issue_up(
            issue: IssueId,
            only_at: Option<i64>,
            in_trash: InTrash,
            now: Timestamp
        ) -> Option<i64>;
        /// Keeps an old code with its issue.
        fn insert_issue_alias(alias: &IssueAlias) -> ();
        /// Waits for every other change that gives out or renews the codes of this project, until
        /// the end of this change: the mode and the codes of a project are judged and written by
        /// one change at a time.
        fn lock_issue_codes(project: ProjectId) -> ();
        /// Appends the event of something this change did besides what its own event says (an
        /// issue got a new code because the mode of its project changed), in the same
        /// transaction.
        fn append_follow_up_event(event: &DomainEvent) -> ();
    }
}
