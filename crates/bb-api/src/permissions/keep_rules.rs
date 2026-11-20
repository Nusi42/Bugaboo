//! What the routes of *Keep and delete rules* need (PRIV-05, ADR-0045): every person who signs in
//! reads the rules, as their own right to know how long their data is kept (not with an access
//! key); a count and *Delete them* need *Change organization settings*, which no access key holds
//! and no agent acts with. A count of old records is personal data and is never stored by a cache.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the keep rules, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listKeepRules", Declared::OwnAccount, Answer::Ordinary),
    route(
        "countWouldDelete",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "countPastKeepPeriod",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "deletePastKeepPeriod",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
];
