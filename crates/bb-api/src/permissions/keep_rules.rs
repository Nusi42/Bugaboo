//! What the routes of *Keep and delete rules* need (PRIV-05, ADR-0045, decision 6): *Change
//! organization settings*, which no access key holds and no agent acts with. A count of old records
//! is personal data and is never stored by a cache.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the keep rules, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "listKeepRules",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
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
