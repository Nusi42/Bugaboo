//! What the routes of work times need (PLAN-11, PLAN-15, PRIV-09, ADR-0030): one's own work times a
//! signed-in person, another person's usual work days and away times the permission *Enter work
//! times for others*, a share *Decide who has which role* in the project, and what depends on the
//! record (another person's work times by *Who sees people*, an away time by whose it is) is
//! judged by the use case, behind the route of a signed-in person: no access key reaches it. Every
//! read of work times is personal data and is never stored by a cache; the answer to a change is
//! never kept anyway.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of work times, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("getMyWorkTimes", Declared::SignedIn, Answer::NeverStored),
    route(
        "getWorkTimesOfPerson",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route("changeMyUsualDays", Declared::SignedIn, Answer::Ordinary),
    route(
        "changeUsualDaysOfPerson",
        Declared::Permission(Permission::EnterWorkTimesForOthers),
        Answer::Ordinary,
    ),
    route("enterMyAwayTime", Declared::SignedIn, Answer::Ordinary),
    route(
        "enterAwayTimeForPerson",
        Declared::Permission(Permission::EnterWorkTimesForOthers),
        Answer::Ordinary,
    ),
    route("changeAwayTime", Declared::SignedIn, Answer::Ordinary),
    route("removeAwayTime", Declared::SignedIn, Answer::Ordinary),
    route(
        "listProjectShares",
        Declared::Permission(Permission::SeeProjects),
        Answer::NeverStored,
    ),
    route(
        "setShare",
        Declared::Permission(Permission::DecideRoles),
        Answer::Ordinary,
    ),
];
