//! What the routes of the timer and of time entries need (ADR-0035, decision 2): the rows of the
//! table of permissions for this area, kept beside it so the table does not grow.
//!
//! Starting the timer on an issue and booking, changing and deleting an entry need *Book time*
//! where the issue is. The timer is the person's own: reading it, stopping, switching, moving its
//! start and throwing it away are `own-account`, and the use case judges *Book time* on the issues
//! whose time it saves. One's own entries are read as one's own; the entries on an issue are read
//! by whoever sees the issue, with the people and the lengths as the use case allows, and so are
//! the time of an issue and the totals of a project.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

const BOOK_TIME: Declared = Declared::Permission(Permission::BookTime);

/// The routes of the timer and of time entries, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("getTimer", Declared::OwnAccount, Answer::NeverStored),
    route("startTimer", BOOK_TIME, Answer::Ordinary),
    route("stopTimer", Declared::OwnAccount, Answer::Ordinary),
    route("switchTimer", Declared::OwnAccount, Answer::Ordinary),
    route("changeTimerStart", Declared::OwnAccount, Answer::Ordinary),
    route("throwAwayTimer", Declared::OwnAccount, Answer::Ordinary),
    route(
        "listMyTimeEntries",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
    route("addTimeEntry", BOOK_TIME, Answer::Ordinary),
    route("changeTimeEntry", BOOK_TIME, Answer::Ordinary),
    route("deleteTimeEntry", BOOK_TIME, Answer::Ordinary),
    route(
        "listIssueTimeEntries",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    // What a new entry starts with is read by whoever may book on the issue.
    route("getTimeEntryDefaults", BOOK_TIME, Answer::NeverStored),
    // The time of an issue and the totals of a project (TIME-03) are read by whoever sees the issue
    // or the project; the totals count only the issues they see and the fields they may read.
    route(
        "getIssueTime",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "getProjectTime",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
];
