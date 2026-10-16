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
    Example::route("createIssueType"),
    Example::route("changeIssueType"),
    Example::with(
        "getFormLayout",
        &[Quirk::AtTheTargetProject, Quirk::HidesUnseen],
    ),
    Example::with("getLayoutEditor", &[Quirk::AtTheTargetProject]),
    Example::with("createLayout", &[Quirk::AtTheTargetProject]),
    Example::with("saveLayout", &[Quirk::AtTheTargetProject]),
    Example::with("deleteLayout", &[Quirk::AtTheTargetProject]),
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
    Example::route("listMentions"),
    Example::route("readMentions"),
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
    Example::with("getIssueSorting", &[Quirk::AnIssueOfTheTarget]),
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
    Example::route("createLabelGroup"),
    Example::route("changeLabelGroup"),
    Example::route("createLabel"),
    Example::route("previewLabelChange"),
    Example::route("changeLabel"),
    Example::route("combineLabels"),
    Example::with("moveIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("previewBulkChange", &[Quirk::SkipsWhatItMayNotChange]),
    Example::with("applyBulkChange", &[Quirk::SkipsWhatItMayNotChange]),
    Example::with(
        "getBulkRun",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "cancelBulkRun",
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

/// A route that judges the record it names, as the example names the target project.
const ON_THE_TARGET: Quirk = Quirk::OnTheTargetProject(Permission::ChangeProjectSettings);

/// A route of the person's own state about an issue: the issue must be one they may see.
const SEES_THE_ISSUE: Quirk = Quirk::OnTheTargetProject(Permission::SeeIssues);

/// An ID that no invitation, session, passkey or access key has (the random kind: a time-ordered
/// one would not be an ID of these, and the route would say "not found" before it looked at
/// anything).
const NOBODYS_RANDOM_ID: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// A short name that no other call of the matrix used, for a project to make.
fn unique_short_name(fixture: &Fixture<'_>) -> String {
    let mut number = fixture.next_number();
    let mut letters = String::from("Z");
    loop {
        // `A` to `J` for the digits: a short name is capital letters and digits.
        letters.push(char::from(b'A' + u8::try_from(number % 10).unwrap_or(0)));
        number /= 10;
        if number == 0 {
            return letters;
        }
    }
}

/// A request: the method, the address and the body.
pub(super) type Request = (&'static str, String, Option<Value>);

/// The routes of everybody and of whoever is signed in.
fn open_request(operation: &str) -> Result<Option<Request>, Failure> {
    let wrong = "not the password";
    Ok(Some(match operation {
        // Open to everybody.
        "getStatus" => ("GET", "/api/v1/status".into(), None),
        "getOpenApi" => ("GET", "/api/v1/openapi.json".into(), None),
        "getAuthStatus" => ("GET", "/api/v1/auth/status".into(), None),
        "completeSetup" => (
            "POST",
            "/api/v1/auth/setup".into(),
            Some(
                json!({"code": "not the code", "display_name": "Eve", "email": "eve@example.test", "password": PASSWORD}),
            ),
        ),
        "signIn" => (
            "POST",
            "/api/v1/auth/sign-in".into(),
            Some(json!({"email": "nobody@example.test", "password": wrong})),
        ),
        "signUp" => (
            "POST",
            "/api/v1/auth/sign-up".into(),
            Some(
                json!({"email": "newcomer@example.test", "display_name": "Newcomer", "password": PASSWORD}),
            ),
        ),
        "startPasskeySignIn" => ("POST", "/api/v1/auth/passkey/sign-in/options".into(), None),
        "signInWithPasskey" => (
            "POST",
            "/api/v1/auth/passkey/sign-in".into(),
            Some(json!({"challenge_id": "none", "credential": {}})),
        ),
        "lookUpInvitation" => (
            "POST",
            "/api/v1/auth/invitation/lookup".into(),
            Some(json!({"secret": "not a secret"})),
        ),
        "acceptInvitation" => (
            "POST",
            "/api/v1/auth/invitation/accept".into(),
            Some(json!({"secret": "not a secret", "display_name": "Bob", "password": PASSWORD})),
        ),
        // Whoever is signed in, and the choices a person makes for themselves.
        "listMySettings" => ("GET", "/api/v1/me/settings".into(), None),
        "getImageHosts" => ("GET", "/api/v1/content/image-hosts".into(), None),
        "changeMySetting" => (
            "PUT",
            "/api/v1/me/settings/appearance.mode".into(),
            Some(json!({"value": "dark"})),
        ),
        "resetMySetting" => ("DELETE", "/api/v1/me/settings/appearance.mode".into(), None),
        "getMe" => ("GET", "/api/v1/auth/me".into(), None),
        "signOut" => ("POST", "/api/v1/auth/sign-out".into(), None),
        _ => return Ok(None),
    }))
}

/// A person's own sign-in data and access keys.
async fn own_request(
    operation: &str,
    actor: Actor,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    let wrong = "not the password";
    Ok(Some(match operation {
        // A person's own sign-in data.
        "listSessions" => ("GET", "/api/v1/auth/sessions".into(), None),
        "signOutEverywhere" => ("DELETE", "/api/v1/auth/sessions".into(), None),
        "endSession" => {
            // The session of the actor, so ending it is what it would be; nobody has one.
            let id = if actor.signed_in() {
                let sessions = browser.send("GET", "/api/v1/auth/sessions", None).await?;
                sessions.body["sessions"]
                    .as_array()
                    .and_then(|list| list.iter().find(|session| session["current"] == true))
                    .and_then(|session| session["id"].as_str())
                    .unwrap_or(NOBODYS_RANDOM_ID)
                    .to_owned()
            } else {
                NOBODYS_RANDOM_ID.to_owned()
            };
            ("DELETE", format!("/api/v1/auth/sessions/{id}"), None)
        }
        "changePassword" => (
            "POST",
            "/api/v1/auth/password".into(),
            Some(json!({"current_password": wrong, "new_password": OTHER_PASSWORD})),
        ),
        "getTwoStep" => ("GET", "/api/v1/auth/two-step".into(), None),
        "startCodeApp" => (
            "POST",
            "/api/v1/auth/two-step/code-app".into(),
            Some(json!({"password": wrong})),
        ),
        "confirmCodeApp" => (
            "POST",
            "/api/v1/auth/two-step/code-app/confirm".into(),
            Some(json!({"code": "000000", "password": wrong})),
        ),
        "removeCodeApp" => (
            "POST",
            "/api/v1/auth/two-step/code-app/remove".into(),
            Some(json!({"password": wrong})),
        ),
        "renewBackupCodes" => (
            "POST",
            "/api/v1/auth/two-step/backup-codes".into(),
            Some(json!({"password": wrong})),
        ),
        "startAddPasskey" => (
            "POST",
            "/api/v1/auth/passkeys/options".into(),
            Some(json!({"password": wrong})),
        ),
        "addPasskey" => (
            "POST",
            "/api/v1/auth/passkeys".into(),
            Some(
                json!({"challenge_id": "none", "credential": {}, "label": "Key", "password": wrong}),
            ),
        ),
        "removePasskey" => (
            "POST",
            format!("/api/v1/auth/passkeys/{NOBODYS_RANDOM_ID}/remove"),
            Some(json!({"password": wrong})),
        ),
        "finishSignInWithCode" => (
            "POST",
            "/api/v1/auth/second-step".into(),
            Some(json!({"code": "000000"})),
        ),
        "startSecondStepWithPasskey" => (
            "POST",
            "/api/v1/auth/second-step/passkey/options".into(),
            None,
        ),
        "finishSignInWithPasskey" => (
            "POST",
            "/api/v1/auth/second-step/passkey".into(),
            Some(json!({"challenge_id": "none", "credential": {}})),
        ),
        // The access keys of a person: their own sign-in data, which a key never reaches.
        "listAccessKeys" => ("GET", "/api/v1/access-keys".into(), None),
        "createAccessKey" => (
            "POST",
            "/api/v1/access-keys".into(),
            Some(json!({"password": wrong, "name": "Made by the matrix", "choice": "see-only"})),
        ),
        "deleteAccessKey" => (
            "DELETE",
            format!("/api/v1/access-keys/{NOBODYS_RANDOM_ID}"),
            None,
        ),
        "getAccessKeyRules" => ("GET", "/api/v1/access-keys/rules".into(), None),
        "listProjectChoices" => ("GET", "/api/v1/access-keys/projects".into(), None),
        _ => return Ok(None),
    }))
}

/// The organization.
async fn organization_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        // The organization.
        "listInvitations" => ("GET", "/api/v1/auth/invitations".into(), None),
        "createInvitation" => (
            "POST",
            "/api/v1/auth/invitations".into(),
            Some(json!({"email": format!("invited{}@example.test", fixture.next_number())})),
        ),
        "revokeInvitation" => {
            // An invitation that is there, so a person who is let in takes a real one back.
            let made = fixture
                .rig()
                .api()
                .create_invitation(
                    Rig::info(),
                    CreateInvitation {
                        session_token: fixture.admin_session().to_owned(),
                        email: format!("to-revoke{}@example.test", fixture.next_number()),
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            (
                "DELETE",
                format!("/api/v1/auth/invitations/{}", made.invitation.id),
                None,
            )
        }
        "listOrganizationSettings" => ("GET", "/api/v1/settings".into(), None),
        "changeOrganizationSetting" => (
            "PUT",
            "/api/v1/settings/security.session-idle-minutes".into(),
            Some(json!({"value": 30})),
        ),
        "resetOrganizationSetting" => (
            "DELETE",
            "/api/v1/settings/security.session-idle-minutes".into(),
            None,
        ),
        "lockOrganizationSetting" => ("PUT", "/api/v1/settings/appearance.mode/lock".into(), None),
        "unlockOrganizationSetting" => (
            "DELETE",
            "/api/v1/settings/appearance.mode/lock".into(),
            None,
        ),
        "getSignUpPolicy" => ("GET", "/api/v1/auth/sign-up-policy".into(), None),
        "changeSignUpPolicy" => (
            "PUT",
            "/api/v1/auth/sign-up-policy".into(),
            Some(json!({"enabled": false, "domains": [], "version": 0})),
        ),
        "getServerStatus" => ("GET", "/api/v1/admin/status".into(), None),
        // How many people with a role still have to set up a second step.
        "getSecondStepGap" => (
            "GET",
            format!("/api/v1/roles/{}/second-step-gap", fixture.a_role()?),
            None,
        ),
        _ => return Ok(None),
    }))
}

/// The projects.
fn project_request(fixture: &Fixture<'_>, operation: &str) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    let sibling = fixture.project(Project::Sibling).to_string();
    Ok(Some(match operation {
        // Projects.
        "listProjects" => ("GET", "/api/v1/projects".into(), None),
        "createProject" => (
            "POST",
            "/api/v1/projects".into(),
            Some(json!({"short_name": unique_short_name(fixture), "name": "Made by the matrix"})),
        ),
        "listProjectSettings" => ("GET", format!("/api/v1/projects/{target}/settings"), None),
        "changeProjectSetting" => (
            "PUT",
            format!("/api/v1/projects/{target}/settings/test.project-note"),
            Some(json!({"value": "short"})),
        ),
        "resetProjectSetting" => (
            "DELETE",
            format!("/api/v1/projects/{target}/settings/test.project-note"),
            None,
        ),
        "getProject" => ("GET", format!("/api/v1/projects/{target}"), None),
        "changeProject" => (
            "PATCH",
            format!("/api/v1/projects/{target}"),
            Some(json!({"version": 1, "name": "Renamed by the matrix"})),
        ),
        "createSubProject" => (
            "POST",
            format!("/api/v1/projects/{target}/sub-projects"),
            Some(json!({"short_name": unique_short_name(fixture), "name": "Made by the matrix"})),
        ),
        "previewProjectMove" => (
            "POST",
            format!("/api/v1/projects/{target}/move-preview"),
            Some(json!({"to": sibling})),
        ),
        "moveProject" => (
            "POST",
            format!("/api/v1/projects/{target}/move"),
            Some(json!({"to": sibling, "preview": "not a preview"})),
        ),
        // What roles see, and what a person can do and why: in the target project.
        "getFieldPermissions" => (
            "GET",
            format!("/api/v1/field-permissions?project={target}"),
            None,
        ),
        "changeFieldPermission" => (
            "PUT",
            format!(
                "/api/v1/field-permissions/{}/cost-center?project={target}",
                fixture.a_role()?
            ),
            Some(json!({"level": "hidden"})),
        ),
        "getPeopleVisibility" => (
            "GET",
            format!("/api/v1/people-visibility?project={target}"),
            None,
        ),
        "changePeopleVisibility" => (
            "PUT",
            format!(
                "/api/v1/people-visibility/{}/followers?project={target}",
                fixture.a_role()?
            ),
            Some(json!({"level": "hidden"})),
        ),
        "checkAccess" => (
            "GET",
            format!(
                "/api/v1/check-access/people/{}?project={target}",
                fixture.a_person()
            ),
            None,
        ),
        "listPeopleToCheck" => (
            "GET",
            format!("/api/v1/check-access/people?project={target}"),
            None,
        ),
        _ => return Ok(None),
    }))
}

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
    if let Some(found) = super::examples_tags::tag_request(fixture, operation).await? {
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
