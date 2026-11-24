//! What the routes of relations, tags and labels need (ISS-03, TAG-01): the kinds of relation,
//! the relations and the parent of an issue, the tags and the labels with their groups. Kept beside
//! the table so it does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route, route_reading};

/// The routes of relations, tags and labels, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listRelationKinds", Declared::SignedIn, Answer::Ordinary),
    route(
        "createRelationKind",
        Declared::Permission(Permission::ManageIssueTypes),
        Answer::Ordinary,
    ),
    route(
        "changeRelationKind",
        Declared::Permission(Permission::ManageIssueTypes),
        Answer::Ordinary,
    ),
    route(
        "getIssueRelations",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "listSubIssues",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "listRelatedIssues",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    // The relation map reads like a search: of an issue the person sees, and of a search, which
    // lets in whoever is signed in and shows what they read.
    route(
        "getIssueMap",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route_reading("getSearchMap", Declared::SignedIn, Answer::Ordinary),
    route(
        "setIssueParent",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "addRelation",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "removeRelation",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "getIssueLabelsAndTags",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "addTag",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "removeTag",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "addLabel",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "removeLabel",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "listTagSuggestions",
        Declared::Permission(Permission::SeeProjects),
        Answer::NeverStored,
    ),
    route(
        "listTags",
        Declared::Permission(Permission::ManageLabels),
        Answer::NeverStored,
    ),
    route(
        "previewTagChange",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "changeTag",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "combineTags",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route("listLabels", Declared::SignedIn, Answer::Ordinary),
    route("listLabelGroups", Declared::SignedIn, Answer::Ordinary),
    route(
        "createLabelGroup",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "changeLabelGroup",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "createLabel",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "previewLabelChange",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "changeLabel",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
    route(
        "combineLabels",
        Declared::Permission(Permission::ManageLabels),
        Answer::Ordinary,
    ),
];
