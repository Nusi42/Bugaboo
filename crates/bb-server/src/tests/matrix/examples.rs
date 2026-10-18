//! The table of example requests, one row for every route of the API description (SEC-MATRIX).
//!
//! A package that adds a route adds its row here in the same change: the matrix fails for a route
//! of the description that has no row, and for a row that names no route. A row says how the route
//! behaves at the edges that the description cannot say (below); [`request`] makes the request,
//! with an address that is filled in and a body that is valid, so that a person who may not do it
//! is refused for the reason the matrix expects and not because the body is wrong.
//!
//! The entry points of later milestones add their own tables next to this one (M3 issues, M4 e-mail,
//! M5 search, M8 assistant and AI apps, M9 exports); this one is the API.

use bb_app::access::Permission;
use bb_app::auth::CreateInvitation;
use bb_testing::Failure;
use serde_json::{Value, json};

use super::actors::{Actor, Project};
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::rig::{OTHER_PASSWORD, PASSWORD, Rig};

mod requests;
use requests::*;

/// A request: the method, the address and the body.
pub(super) type Request = (&'static str, String, Option<Value>);

/// What a route does at the edges the API description does not say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Quirk {
    /// The route answers a project that the actor cannot see with `resource.not_found` before it
    /// looks at the permission, also an actor who knows it exists (they hold *Decide who has which
    /// role* there): it is about what is inside the project.
    HidesUnseen,
    /// The route looks at the permission first, and then answers `resource.not_found` to somebody
    /// who holds it but does not see the project: changing what one cannot see is not possible.
    NeedsToSeeTheProject,
    /// The route lists what the actor may see: a person without the permission is not refused but
    /// gets what they see, which may be nothing (M2-F30). Only the routes named here may do that.
    FiltersWhatIsSeen,
    /// The route is about the target project through its query (`?project=`) and not through its
    /// address: it counts what the actor holds in that project, and a project the actor may not
    /// know about is `resource.not_found`, like one in the address.
    AtTheTargetProject,
    /// The call ends the session of whoever makes it, so they sign in again afterwards.
    EndsTheSession,
    /// The route names a thing that is not there (a passkey nobody added), so a person who is let
    /// in is told `resource.not_found`, which is not a refusal of the permission.
    NamesAMissingThing,
    /// The route finishes a sign-in that is half done (the second step): a session that is whole
    /// has nothing to finish, so it is told it is not signed in for this.
    FinishesAHalfDoneSignIn,
    /// The route may answer with this other refusal (a `401` or `403` with this code) to a person
    /// who is let in: the wrong password it asks for again, a code that is wrong. Any other
    /// refusal that the matrix does not expect fails, so a permitted actor cannot be refused for
    /// another reason without anybody noticing.
    AlsoAnswers(&'static str),
    /// The route names a record that belongs to the person who made it (an upload). The example
    /// names the actor's own where the actor may start one, so the route runs on a real record; for
    /// whoever may not, it names one that nobody made, and they are told `resource.not_found`, as
    /// for a record that is not there, whether the permission is theirs or not, because the record
    /// is looked up before the permission of the issue it belongs to (M4 WP-05).
    TheirOwn,
    /// The route is about an issue in the target project that somebody else reported (M3 WP-03): a
    /// person who does not see the project, or lacks *See issues* there, is told
    /// `resource.not_found`, as for an issue that is not there; a person who sees the issue and
    /// lacks the permission of the route is refused.
    AnIssueOfTheTarget,
    /// The route takes something out of the trash: a person who sees the issue and lacks the
    /// permission is told `resource.not_found`, as for something that is not in the trash, and
    /// not that they are refused (SEC-06).
    InTheTrash,
    /// The route lists the issues of the target project: anybody who sees the project gets a list
    /// (without *See issues*, of the issues they reported), and anybody else
    /// `resource.not_found`.
    ListsTheIssuesSeen,
    /// A route of *My settings*: the person's own choices, not their sign-in data, so an access
    /// key acts as the person: it reads them, and changes them only if it may change anything
    /// (WP-07, ADR-0025 decision 8).
    KeysActAsThePerson,
    /// The route lets in whoever is signed in and the use case judges the record it names; the
    /// example names the target project, so the person needs this permission there (and to see
    /// the project: one who does not is told it is not found, before the permission is looked at).
    OnTheTargetProject(Permission),
    /// The route declares one permission and its use case also needs this one on the record it
    /// names (*Edit anyway* takes over editing a record only for a person who may change it).
    AlsoNeeds(Permission),
    /// The route names a tag that the example puts on an issue of the target project: a person who
    /// holds the permission but does not read the issues there is told `resource.not_found`, as
    /// for a tag that is not there (SEC-06).
    NamesATagOfTheTarget,
    /// The route names an issue type or a group of labels that the example creates in the target
    /// project: a person who may not know that project is told `resource.not_found`, as for one
    /// that is not there (SEC-03, ADR-0027 amendment).
    NamesASubjectOfTheTarget,
    /// The route belongs to an open editor, a tab of a signed-in person: an access key is turned
    /// away from it, whatever the key may do (ADR-0019).
    SessionOnly,
    /// The route answers every signed-in actor and says in its answer what it did to each issue it
    /// was asked about (changing many issues, ISS-09): the issue of the example, in the target
    /// project, is left alone as *not found* when the actor does not see it, as *not allowed* when
    /// they see it and lack *Change issues*, and is changed otherwise. The matrix reads that from
    /// the answer, as a route that is let in says nothing else about the actor.
    SkipsWhatItMayNotChange,
}

/// One row of the table: a route and what is special about it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Example {
    /// The operation ID of the route in the API description.
    pub(super) operation: &'static str,
    /// What is special about the route.
    pub(super) quirks: &'static [Quirk],
}

impl Example {
    const fn route(operation: &'static str) -> Self {
        Self {
            operation,
            quirks: &[],
        }
    }

    const fn with(operation: &'static str, quirks: &'static [Quirk]) -> Self {
        Self { operation, quirks }
    }

    /// Whether the route has this quirk.
    pub(super) fn has(&self, quirk: Quirk) -> bool {
        self.quirks.contains(&quirk)
    }

    /// The permission a person needs on the target project for a route that judges the record
    /// it names, if the row says so.
    pub(super) fn record_permission(&self) -> Option<Permission> {
        self.quirks.iter().find_map(|quirk| match quirk {
            Quirk::OnTheTargetProject(permission) => Some(*permission),
            _ => None,
        })
    }

    /// The second permission the use case needs, if the row says so.
    pub(super) fn also_needs(&self) -> Option<Permission> {
        self.quirks.iter().find_map(|quirk| match quirk {
            Quirk::AlsoNeeds(permission) => Some(*permission),
            _ => None,
        })
    }

    /// Whether the row says that the route may answer a person who is let in with this code.
    pub(super) fn answers(&self, code: &str) -> bool {
        self.quirks
            .iter()
            .any(|quirk| matches!(quirk, Quirk::AlsoAnswers(known) if *known == code))
    }
}

/// What the routes of an upload have in common: the example names the actor's own upload, or one nobody started.
const UPLOAD: [Quirk; 4] = [
    CHANGES_THE_FILES,
    Quirk::AtTheTargetProject,
    Quirk::NamesAMissingThing,
    Quirk::TheirOwn,
];

/// What the routes that add or take away files declare as `by-record`: the use case judges the
/// record (the issue, or the comment the file belongs to) and lets in a person on an issue they
/// reported or a comment they wrote; the example is an issue of somebody else, which needs
/// *Change issues*.
const CHANGES_THE_FILES: Quirk = Quirk::OnTheTargetProject(Permission::ChangeIssues);

/// Every route of the API description, in the order of the description.
pub(super) const EXAMPLES: &[Example] = &[
    Example::route("getServerStatus"),
    Example::route("acceptInvitation"),
    Example::route("lookUpInvitation"),
    Example::route("listInvitations"),
    Example::route("createInvitation"),
    Example::route("revokeInvitation"),
    Example::route("getMe"),
    Example::route("signInWithPasskey"),
    Example::route("startPasskeySignIn"),
    Example::with(
        "addPasskey",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with(
        "startAddPasskey",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with(
        "removePasskey",
        &[
            Quirk::NamesAMissingThing,
            Quirk::AlsoAnswers("auth.wrong_credentials"),
        ],
    ),
    Example::with(
        "changePassword",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with("finishSignInWithCode", &[Quirk::FinishesAHalfDoneSignIn]),
    Example::with("finishSignInWithPasskey", &[Quirk::FinishesAHalfDoneSignIn]),
    Example::with(
        "startSecondStepWithPasskey",
        &[Quirk::FinishesAHalfDoneSignIn],
    ),
    Example::route("listSessions"),
    Example::with("signOutEverywhere", &[Quirk::EndsTheSession]),
    Example::with("endSession", &[Quirk::EndsTheSession]),
    Example::with(
        "completeSetup",
        &[Quirk::AlsoAnswers("auth.wrong_setup_code")],
    ),
    Example::with("signIn", &[Quirk::AlsoAnswers("auth.wrong_credentials")]),
    Example::with("signOut", &[Quirk::EndsTheSession]),
    Example::with("signUp", &[Quirk::AlsoAnswers("auth.sign_up_refused")]),
    Example::route("getSignUpPolicy"),
    Example::route("changeSignUpPolicy"),
    Example::route("getAuthStatus"),
    Example::route("getTwoStep"),
    Example::with(
        "renewBackupCodes",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with(
        "startCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with(
        "confirmCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with(
        "removeCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::route("getOpenApi"),
    Example::with("listProjects", &[Quirk::FiltersWhatIsSeen]),
    Example::route("createProject"),
    Example::with("getProject", &[Quirk::HidesUnseen]),
    Example::with("changeProject", &[Quirk::NeedsToSeeTheProject]),
    Example::with("moveProject", &[Quirk::HidesUnseen]),
    Example::with("previewProjectMove", &[Quirk::HidesUnseen]),
    Example::with("createSubProject", &[Quirk::NeedsToSeeTheProject]),
    Example::route("getStatus"),
    Example::route("listAccessKeys"),
    Example::route("listPeople"),
    Example::route("getPerson"),
    Example::route("listGroups"),
    Example::route("createGroup"),
    Example::route("getGroup"),
    Example::route("renameGroup"),
    Example::route("deleteGroup"),
    Example::route("listGroupMembers"),
    Example::route("addGroupMember"),
    Example::route("removeGroupMember"),
    Example::route("listMemberChoices"),
    Example::route("listRoles"),
    Example::route("createRole"),
    Example::route("getRole"),
    Example::route("changeRole"),
    Example::route("deleteRole"),
    Example::route("copyRole"),
    Example::with("resetRole", &[Quirk::AlsoAnswers("access.too_much")]),
    Example::route("listPermissions"),
    Example::route("listAssignments"),
    Example::route("assignRole"),
    Example::route("listProjectAssignments"),
    Example::route("assignProjectRole"),
    Example::route("removeAssignment"),
    Example::route("listAssignmentChoices"),
    Example::route("listProjectAssignmentChoices"),
    Example::with(
        "createAccessKey",
        &[Quirk::AlsoAnswers("auth.wrong_credentials")],
    ),
    Example::with("deleteAccessKey", &[Quirk::NamesAMissingThing]),
    Example::route("getAccessKeyRules"),
    Example::route("listProjectChoices"),
    Example::route("getSecondStepGap"),
    Example::route("listOrganizationSettings"),
    Example::route("changeOrganizationSetting"),
    Example::route("resetOrganizationSetting"),
    Example::route("lockOrganizationSetting"),
    Example::route("unlockOrganizationSetting"),
    Example::route("listProjectSettings"),
    Example::route("changeProjectSetting"),
    Example::route("resetProjectSetting"),
    Example::route("checkOwnAccessChange"),
    Example::with("listMySettings", &[Quirk::KeysActAsThePerson]),
    Example::with("getImageHosts", &[Quirk::KeysActAsThePerson]),
    Example::with("changeMySetting", &[Quirk::KeysActAsThePerson]),
    Example::with("resetMySetting", &[Quirk::KeysActAsThePerson]),
    Example::with("getFieldPermissions", &[Quirk::AtTheTargetProject]),
    Example::with("changeFieldPermission", &[Quirk::AtTheTargetProject]),
    Example::with("getPeopleVisibility", &[Quirk::AtTheTargetProject]),
    Example::with("changePeopleVisibility", &[Quirk::AtTheTargetProject]),
    Example::with("checkAccess", &[Quirk::AtTheTargetProject]),
    Example::with("listPeopleToCheck", &[Quirk::AtTheTargetProject]),
    Example::with("takeEditLock", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::with("readEditLock", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::with("heartbeatEditLock", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::route("releaseEditLock"),
    Example::with(
        "askToFinish",
        &[ON_THE_TARGET, Quirk::NamesAMissingThing, Quirk::SessionOnly],
    ),
    Example::with(
        "editAnyway",
        &[
            Quirk::AtTheTargetProject,
            Quirk::HidesUnseen,
            Quirk::AlsoNeeds(Permission::ChangeProjectSettings),
            Quirk::SessionOnly,
        ],
    ),
    Example::with(
        "releaseStuckEditLock",
        &[
            Quirk::AtTheTargetProject,
            Quirk::HidesUnseen,
            Quirk::AlsoNeeds(Permission::ChangeProjectSettings),
            Quirk::NamesAMissingThing,
            Quirk::SessionOnly,
        ],
    ),
    Example::with("saveDraft", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::with("readDraft", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::route("throwAwayDraft"),
    Example::route("checkVersions"),
    Example::with("listIssueTypes", &[Quirk::KeysActAsThePerson]),
    Example::with("createIssueType", &[Quirk::AtTheTargetProject]),
    Example::with("changeIssueType", &[Quirk::AtTheTargetProject]),
    Example::with(
        "listIssueTypeOffers",
        &[Quirk::KeysActAsThePerson, Quirk::NamesASubjectOfTheTarget],
    ),
    Example::with("changeIssueTypeOffers", &[Quirk::AtTheTargetProject]),
    Example::with(
        "listLabelGroupOffers",
        &[Quirk::KeysActAsThePerson, Quirk::NamesASubjectOfTheTarget],
    ),
    Example::with("changeLabelGroupOffers", &[Quirk::AtTheTargetProject]),
    Example::with(
        "getFormLayout",
        &[Quirk::AtTheTargetProject, Quirk::HidesUnseen],
    ),
    Example::with("getLayoutEditor", &[Quirk::AtTheTargetProject]),
    Example::with("createLayout", &[Quirk::AtTheTargetProject]),
    Example::with("saveLayout", &[Quirk::AtTheTargetProject]),
    Example::with("deleteLayout", &[Quirk::AtTheTargetProject]),
    Example::with("getProjectStatuses", &[Quirk::HidesUnseen]),
    Example::with("getWhoMayChange", &[Quirk::HidesUnseen]),
    Example::with("listWorkflows", &[Quirk::AtTheTargetProject]),
    Example::with("getWorkflowEditor", &[Quirk::AtTheTargetProject]),
    Example::with("createWorkflow", &[Quirk::AtTheTargetProject]),
    Example::with("useWorkflow", &[Quirk::AtTheTargetProject]),
    Example::with("publishWorkflow", &[Quirk::AtTheTargetProject]),
    Example::with("previewPublishWorkflow", &[Quirk::AtTheTargetProject]),
    Example::with("deleteWorkflow", &[Quirk::AtTheTargetProject]),
    Example::with(
        "tryWorkflow",
        &[
            Quirk::AnIssueOfTheTarget,
            Quirk::AlsoNeeds(Permission::ManageWorkflows),
        ],
    ),
    Example::with("listIssues", &[Quirk::ListsTheIssuesSeen]),
    Example::with("listAssignablePeople", &[Quirk::HidesUnseen]),
    Example::with("createIssue", &[Quirk::NeedsToSeeTheProject]),
    Example::with("getIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("changeIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("saveIssueDescription", &[Quirk::AnIssueOfTheTarget]),
    Example::with("deleteIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "restoreIssue",
        &[Quirk::AnIssueOfTheTarget, Quirk::InTheTrash],
    ),
    Example::with("listRelationKinds", &[Quirk::KeysActAsThePerson]),
    Example::route("createRelationKind"),
    Example::route("changeRelationKind"),
    Example::with("getIssueRelations", &[Quirk::AnIssueOfTheTarget]),
    Example::with("listSubIssues", &[Quirk::AnIssueOfTheTarget]),
    Example::with("listRelatedIssues", &[Quirk::AnIssueOfTheTarget]),
    Example::with("setIssueParent", &[Quirk::AnIssueOfTheTarget]),
    Example::with("addRelation", &[Quirk::AnIssueOfTheTarget]),
    Example::with("removeRelation", &[Quirk::AnIssueOfTheTarget]),
    Example::with("addComment", &[Quirk::AnIssueOfTheTarget]),
    Example::with("saveComment", &[Quirk::AnIssueOfTheTarget]),
    Example::with("deleteComment", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "restoreComment",
        &[Quirk::AnIssueOfTheTarget, Quirk::InTheTrash],
    ),
    Example::with("listCommentVersions", &[Quirk::AnIssueOfTheTarget]),
    Example::with("listMentionChoices", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "startUpload",
        &[CHANGES_THE_FILES, Quirk::AnIssueOfTheTarget],
    ),
    Example::with("getUpload", &UPLOAD),
    Example::with("sendUploadPart", &UPLOAD),
    Example::with("cancelUpload", &UPLOAD),
    Example::with("finishUpload", &UPLOAD),
    Example::with("listIssueFiles", &[Quirk::AnIssueOfTheTarget]),
    Example::with("getFileRules", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "deleteAttachment",
        &[CHANGES_THE_FILES, Quirk::AnIssueOfTheTarget],
    ),
    Example::with(
        "restoreAttachment",
        &[
            CHANGES_THE_FILES,
            Quirk::AnIssueOfTheTarget,
            Quirk::InTheTrash,
        ],
    ),
    Example::with("downloadFile", &[Quirk::AnIssueOfTheTarget]),
    Example::route("getMyHandle"),
    Example::route("changeMyHandle"),
    Example::route("listNotifications"),
    Example::route("getUnreadNotifications"),
    Example::route("readNotifications"),
    Example::route("setMailPassword"),
    Example::route("getMailServer"),
    Example::route("sendTestEmail"),
    Example::with("listIssueHistory", &[Quirk::AnIssueOfTheTarget]),
    Example::with("listFollowers", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "getFollowing",
        &[
            Quirk::AnIssueOfTheTarget,
            SEES_THE_ISSUE,
            Quirk::SessionOnly,
        ],
    ),
    Example::with(
        "followIssue",
        &[
            Quirk::AnIssueOfTheTarget,
            SEES_THE_ISSUE,
            Quirk::SessionOnly,
        ],
    ),
    Example::with(
        "unfollowIssue",
        &[
            Quirk::AnIssueOfTheTarget,
            SEES_THE_ISSUE,
            Quirk::SessionOnly,
        ],
    ),
    Example::with("listFieldTypes", &[Quirk::KeysActAsThePerson]),
    Example::with("listFields", &[Quirk::KeysActAsThePerson]),
    Example::with(
        "getField",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with("getFieldUsedBy", &[Quirk::NamesAMissingThing]),
    Example::route("createField"),
    Example::route("changeField"),
    Example::route("addChoice"),
    Example::route("changeChoice"),
    Example::route("reorderChoices"),
    Example::route("combineChoices"),
    Example::with("getIssueLabelsAndTags", &[Quirk::AnIssueOfTheTarget]),
    Example::with("addTag", &[Quirk::AnIssueOfTheTarget]),
    Example::with("removeTag", &[Quirk::AnIssueOfTheTarget]),
    Example::with("addLabel", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "removeLabel",
        &[Quirk::AnIssueOfTheTarget, Quirk::NamesAMissingThing],
    ),
    Example::with("listTagSuggestions", &[Quirk::HidesUnseen]),
    Example::route("listTags"),
    Example::with("previewTagChange", &[Quirk::NamesATagOfTheTarget]),
    Example::with("changeTag", &[Quirk::NamesATagOfTheTarget]),
    Example::with("combineTags", &[Quirk::NamesATagOfTheTarget]),
    Example::with("listLabels", &[Quirk::KeysActAsThePerson]),
    Example::with("listLabelGroups", &[Quirk::KeysActAsThePerson]),
    Example::with("createLabelGroup", &[Quirk::AtTheTargetProject]),
    Example::with("changeLabelGroup", &[Quirk::AtTheTargetProject]),
    Example::with("createLabel", &[Quirk::AtTheTargetProject]),
    Example::with("previewLabelChange", &[Quirk::AtTheTargetProject]),
    Example::with("changeLabel", &[Quirk::AtTheTargetProject]),
    Example::with("combineLabels", &[Quirk::AtTheTargetProject]),
    Example::with("moveIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("previewManyChanges", &[Quirk::SkipsWhatItMayNotChange]),
    Example::with("applyManyChanges", &[Quirk::SkipsWhatItMayNotChange]),
    Example::with(
        "getManyChangeRun",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "stopManyChangeRun",
        &[Quirk::SessionOnly, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "saveIssueFieldText",
        &[
            Quirk::AnIssueOfTheTarget,
            Quirk::OnTheTargetProject(Permission::ChangeIssues),
            Quirk::SessionOnly,
        ],
    ),
];

/// The request of an example: the method, the address with its parts filled in, and a body that
/// is valid for the route. Changes that need a thing to change make it first (a new invitation to
/// take back, the session of the actor to end).
///
/// # Errors
/// A failure of the set-up, or an operation that has a row but no request here.
pub(super) async fn request(
    fixture: &Fixture<'_>,
    example: &Example,
    actor: Actor,
    browser: &mut Browser,
) -> Result<Request, Failure> {
    let operation = example.operation;
    if let Some(found) = open_request(operation)? {
        return Ok(found);
    }
    if let Some(found) = own_request(operation, actor, browser).await? {
        return Ok(found);
    }
    if let Some(found) = organization_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = project_request(fixture, operation)? {
        return Ok(found);
    }
    if let Some(found) = super::examples_people::people_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_editing::editing_request(fixture, operation) {
        return Ok(found);
    }
    if let Some(found) = super::examples_fields::field_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_issues::issue_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_layouts::layout_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) =
        super::examples_relations::relation_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) = super::examples_files::file_request(fixture, operation, browser).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_tags::tag_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_workflows::workflow_request(fixture, operation).await? {
        return Ok(found);
    }
    Err(format!("{operation} has a row in the table but no request").into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_route_has_one_row_only() {
        let operations: BTreeSet<&str> = EXAMPLES.iter().map(|example| example.operation).collect();
        assert_eq!(operations.len(), EXAMPLES.len());
    }

    #[test]
    fn only_the_routes_that_list_what_is_seen_may_answer_a_person_without_the_permission() {
        let filtering: Vec<&str> = EXAMPLES
            .iter()
            .filter(|example| example.has(Quirk::FiltersWhatIsSeen))
            .map(|example| example.operation)
            .collect();
        assert_eq!(
            filtering,
            ["listProjects"],
            "M2-F30: one route, with its reason"
        );
    }
}
