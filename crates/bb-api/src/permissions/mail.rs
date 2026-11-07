//! What the routes of the mail server need (NTF-02, PRIV-08): the password and the test e-mail need
//! *Change organization settings*. Kept beside the table so it does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the mail server, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "setMailPassword",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "getMailServer",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "sendTestEmail",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
];
