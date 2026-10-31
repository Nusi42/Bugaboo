//! What the routes of comments and of what a writer may name after `@` need (ISS-05): the rows of
//! the table of permissions for this area, kept beside it so the table does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

const COMMENT: Declared = Declared::Permission(Permission::Comment);

/// The routes of comments, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("addComment", COMMENT, Answer::Ordinary),
    route("saveComment", COMMENT, Answer::Ordinary),
    route("deleteComment", COMMENT, Answer::Ordinary),
    route("restoreComment", COMMENT, Answer::Ordinary),
    route(
        "listCommentVersions",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route("listMentionChoices", COMMENT, Answer::NeverStored),
];
