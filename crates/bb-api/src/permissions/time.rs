//! What the routes of the timer and of time entries need (ADR-0035, decision 2): the rows of the
//! table of permissions for this area, kept beside it so the table does not grow.
//!
//! Starting the timer on an issue and booking, changing and deleting an entry need *Book time*
//! where the issue is. The timer is the person's own: reading it, stopping, switching, moving its
//! start and throwing it away are `own-account`, and the use case judges *Book time* on the issues
//! whose time it saves. One's own entries are read as one's own; the entries on an issue are read
//! by whoever sees the issue, with the people and the lengths as the use case allows.

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
];
