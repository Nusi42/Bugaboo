//! What the routes of working hours need (TIME-08, TIME-12, PRIV-09, ADR-0035): one's own days need
//! a signed-in person, another person's days the permissions of working hours, and the days older
//! than the keep period *Change organization settings*. Every read of working hours is personal data
//! and is never stored by a cache; the answer to a change is never kept anyway.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of working hours, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listMyWorkDays", Declared::SignedIn, Answer::NeverStored),
    route("startWorkDay", Declared::SignedIn, Answer::Ordinary),
    route("startBreak", Declared::SignedIn, Answer::Ordinary),
    route("endBreak", Declared::SignedIn, Answer::Ordinary),
    route("endWorkDay", Declared::SignedIn, Answer::Ordinary),
    route("changeMyWorkDay", Declared::SignedIn, Answer::Ordinary),
    route(
        "getMyWorkDayHistory",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route("getMyBookedWeek", Declared::SignedIn, Answer::NeverStored),
    route(
        "listPeopleOfWorkDays",
        Declared::Permission(Permission::SeeWorkingHoursOfOthers),
        Answer::NeverStored,
    ),
    route(
        "listWorkDaysOfPerson",
        Declared::Permission(Permission::SeeWorkingHoursOfOthers),
        Answer::NeverStored,
    ),
    route(
        "changeWorkDayOfPerson",
        Declared::Permission(Permission::ChangeWorkingHoursOfOthers),
        Answer::Ordinary,
    ),
    route(
        "getWorkDayHistoryOfPerson",
        Declared::Permission(Permission::SeeWorkingHoursOfOthers),
        Answer::NeverStored,
    ),
    route(
        "countOldWorkDays",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "deleteOldWorkDays",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
];
