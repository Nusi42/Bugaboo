//! The names of the members of a record in its events, defined once: the use cases that write the
//! events spell them with these, and the conflict answer (`editing/conflict.rs`) lists the ones it
//! may show with the same constants. The words are what the events already store, so no stored
//! event is renamed.

/// The members of a project.
pub(crate) mod project {
    pub(crate) const NAME: &str = "name";
    pub(crate) const SHORT_NAME: &str = "short_name";
}

/// The members of a group.
pub(crate) mod group {
    pub(crate) const NAME: &str = "name";
}

/// The members of a role.
pub(crate) mod role {
    pub(crate) const NAME: &str = "name";
    pub(crate) const PERMISSIONS: &str = "permissions";
    pub(crate) const TWO_STEP: &str = "two_step";
}

/// The members of the sign-up policy (`bb_core::settings::SIGN_UP_POLICY`).
pub(crate) mod sign_up_policy {
    pub(crate) const ENABLED: &str = "auth_sign_up_enabled";
    pub(crate) const DOMAINS: &str = "auth_sign_up_domains";
}

/// The members of a layout.
pub(crate) mod layout {
    pub(crate) const KIND: &str = "kind";
    pub(crate) const PROJECT: &str = "project";
    pub(crate) const ISSUE_TYPE: &str = "issue_type";
    pub(crate) const SECTIONS: &str = "sections";
    /// The fields taken off the form, each with the roles it keeps (M3-F107).
    pub(crate) const OFF: &str = "off";
}

/// The members of an own theme.
pub(crate) mod theme {
    pub(crate) const KIND: &str = "kind";
    pub(crate) const NAME: &str = "name";
    pub(crate) const EXTENDS: &str = "extends";
    pub(crate) const VALUES: &str = "values";
}

/// The members of a workflow.
pub(crate) mod workflow {
    pub(crate) const NAME: &str = "name";
    pub(crate) const STATUSES: &str = "statuses";
    pub(crate) const CHANGES: &str = "changes";
    /// The places the workflow is used at.
    pub(crate) const USES: &str = "uses";
    /// The list of the workflows the first start made.
    pub(crate) const STARTING: &str = "workflows";
}

/// The members of a kind of relation.
pub(crate) mod relation_kind {
    pub(crate) const FORWARD: &str = "forward";
    pub(crate) const BACKWARD: &str = "backward";
    pub(crate) const RETIRED: &str = "retired";
    /// What the kind does to dates.
    pub(crate) const DATE_EFFECT: &str = "date-effect";
    /// The list of the kinds the first start made.
    pub(crate) const STARTING: &str = "relation-kinds";
}

/// The members of a relation between two issues. History reads the two issues from `FROM` and
/// `TO`.
pub(crate) mod relation {
    pub(crate) const KIND: &str = "kind";
    pub(crate) const FROM: &str = "from";
    pub(crate) const TO: &str = "to";
}

/// The members of a comment. History reads the issue of a new comment from `ISSUE`, and so does
/// the check after a run of many people (`bb-load-check`).
pub mod comment {
    /// The member that names the issue of a comment.
    pub const ISSUE: &str = "issue";
    pub(crate) const TEXT: &str = "text";
    pub(crate) const DELETED_AT: &str = crate::issues::member::DELETED_AT;
}

/// The member of the event of someone following an issue or leaving it.
pub(crate) mod follower {
    pub(crate) const PERSON: &str = "follower";
}

/// The member of the event of the notes of an edit lock (*Ask … to finish*, *Edit anyway*), and
/// the words inside it. The notifications reader reads the person who is told from it.
pub(crate) mod edit_lock {
    pub(crate) const FIELD: &str = "edit_lock";
    /// The part somebody asked the holder of its lock to finish.
    pub(crate) const ASKED_TO_FINISH: &str = "asked_to_finish";
    /// The part whose editing somebody took over.
    pub(crate) const TAKEN_OVER: &str = "taken_over";
    /// The person who holds the lock and is told.
    pub(crate) const PERSON: &str = "person";
}

/// The member of the event of a run of many changes that says where the run is.
pub(crate) mod run {
    pub(crate) const STATUS: &str = "status";
}

/// The member of the event of a person changing their handle.
pub(crate) mod handle {
    pub(crate) const FIELD: &str = "handle";
}

/// The member of the event of a person reading their notifications.
pub(crate) mod notifications {
    pub(crate) const READ: &str = "notifications-read";
}

/// The members of a tag.
pub(crate) mod tag {
    pub(crate) const NAME: &str = "name";
    pub(crate) const COLOR: &str = "color";
    pub(crate) const PROJECTS: &str = "projects";
    /// The tag a tag was combined into, in the event of the tag that went away.
    pub(crate) const COMBINED_INTO: &str = "combined-into";
}

/// The members of a label.
pub(crate) mod label {
    pub(crate) const NAME: &str = "name";
    pub(crate) const COLOR: &str = "color";
    pub(crate) const GROUP: &str = "group";
    pub(crate) const RETIRED: &str = "retired";
    /// The label a label was combined into, in the event of the label that went away.
    pub(crate) const COMBINED_INTO: &str = "combined-into";
}

/// The members of a group of labels.
pub(crate) mod label_group {
    pub(crate) const NAME: &str = "name";
    pub(crate) const PICK_ONE: &str = "pick-one";
    pub(crate) const RETIRED: &str = "retired";
    /// The project the group was made in, in the event of the group.
    pub(crate) const PROJECT: &str = "project";
    /// Where the group is offered, in the event of the group.
    pub(crate) const OFFERED: &str = "offered";
}

/// The members of a saved search (QRY-06).
/// The members of a work day (ADR-0035, decision 7): the times before and after hold the old
/// value of every change; the reason is the reason of the event.
pub(crate) mod work_day {
    /// Whose day it is.
    pub(crate) const PERSON: &str = "person";
    /// Its date.
    pub(crate) const DATE: &str = "date";
    /// Its periods and breaks.
    pub(crate) const TIMES: &str = "times";
    /// The reason, in a notification of a change by somebody else (the event holds it as its
    /// reason).
    pub(crate) const REASON: &str = "reason";
    /// How many old work days of a person were deleted (an event about the person).
    pub(crate) const DELETED: &str = "work_days_deleted";
    /// The days with a date before this one were older than the keep period.
    pub(crate) const BEFORE: &str = "work_days_before";
}

/// The members of the events of work times (ADR-0030, amendment M8). An event never holds the note
/// of an away time: it is read by the notifications and the log of changes, and only the person
/// reads the note.
pub(crate) mod work_times {
    /// Whose work times they are.
    pub(crate) const PERSON: &str = "person";
    /// An away time: its first and last day and the half days, before and after.
    pub(crate) const AWAY: &str = "away";
    /// The usual work days and work hours, before and after.
    pub(crate) const USUAL: &str = "usual_days";
    /// The project of a share.
    pub(crate) const PROJECT: &str = "project";
    /// The share in percent, before and after.
    pub(crate) const SHARE: &str = "share";
    /// Marks the event of a person who left: all their work times were deleted.
    pub(crate) const LEFT: &str = "work_times_left";
    /// How many old away times were deleted (an event about the person).
    pub(crate) const DELETED: &str = "away_times_deleted";
    /// The away times with a last day before this one were older than they are kept.
    pub(crate) const BEFORE: &str = "away_times_before";
}

pub(crate) mod saved_search {
    pub(crate) const NAME: &str = "name";
    pub(crate) const TEXT: &str = "text";
    pub(crate) const VIEW: &str = "view";
    pub(crate) const COLUMNS: &str = "columns";
    pub(crate) const GROUP_BY: &str = "group_by";
    /// The group or project it is shared with.
    pub(crate) const SHARED_WITH: &str = "shared_with";
    pub(crate) const DELETED_AT: &str = crate::issues::member::DELETED_AT;
    /// The saved search a project shows as its list (an event about the project).
    pub(crate) const DEFAULT_LIST: &str = "default_list";
}

/// The members of a time entry (TIME-01, TIME-02). The history of the issue reads the issue from
/// `ISSUE`.
pub(crate) mod time_entry {
    pub(crate) const ISSUE: &str = "issue";
    pub(crate) const DATE: &str = "date";
    pub(crate) const MINUTES: &str = "minutes";
    pub(crate) const NOTE: &str = "note";
    pub(crate) const BILLABLE: &str = "billable";
    pub(crate) const STARTED_AT: &str = "started_at";
    /// The timer that saved the entry: the issue it ran on (and the one a switch goes on with),
    /// its new start after a switch, and its version.
    pub(crate) const TIMER_ISSUE: &str = "timer_issue";
    pub(crate) const TIMER_STARTED_AT: &str = "timer_started_at";
    pub(crate) const TIMER_VERSION: &str = "timer_version";
}

/// The members of the timer of a person (TIME-01).
pub(crate) mod timer {
    /// The issue it runs on; none once it stands still.
    pub(crate) const ISSUE: &str = "issue";
    pub(crate) const STARTED_AT: &str = "started_at";
    /// The moment the person was told that it runs long.
    pub(crate) const WARNED_AT: &str = "warned_at";
}
