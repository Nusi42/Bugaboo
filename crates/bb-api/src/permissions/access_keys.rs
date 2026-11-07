//! What the routes of a person's own access keys need (AUTH-07): the rows of the table of
//! permissions for this area, kept beside it so the table does not grow. A person's keys are their
//! own account, so a session makes and lists them and a key never does.

use super::{Answer, Declared, Route, route};

/// The routes of access keys, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listAccessKeys", Declared::OwnAccount, Answer::NeverStored),
    route("createAccessKey", Declared::OwnAccount, Answer::Secret),
    route("deleteAccessKey", Declared::OwnAccount, Answer::Ordinary),
    route(
        "getAccessKeyRules",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
    route(
        "listProjectChoices",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
];
