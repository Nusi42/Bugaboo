//! The table of example requests, one row for every route of the API description (SEC-MATRIX).
//!
//! A package that adds a route adds its row here in the same change: the matrix fails for a route
//! of the description that has no row, and for a row that names no route. A row says how the route
//! behaves at the edges that the description cannot say (below); [`request`] makes the request,
//! with an address that is filled in and a body that is valid, so that a person who may not do it
//! is refused for the reason the matrix expects and not because the body is wrong.
//!
//! The entry points of later milestones add their own tables next to this one (M3 issues, M4 e-mail,
//! M5 search, M7 assistant and AI apps, M9 exports); this one is the API.

use bb_app::access::Permission;
use bb_app::auth::CreateInvitation;
use bb_testing::Failure;
use serde_json::{Value, json};

use super::actors::{Actor, Project};
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::rig::{OTHER_PASSWORD, PASSWORD, Rig};

mod quirks;
mod requests;
pub(super) use quirks::Quirk;
pub(super) use requests::NOBODYS_RANDOM_ID;
use requests::*;

/// A request: the method, the address and the body.
pub(super) type Request = (&'static str, String, Option<Value>);

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

    /// The permission a person needs for a route that judges the record it names: on the target
    /// project, or in the whole organization, if the row says so.
    pub(super) fn permission_on_the_record(&self) -> Option<Permission> {
        self.record_permission().or_else(|| {
            self.quirks.iter().find_map(|quirk| match quirk {
                Quirk::AtTheOrganization(permission) => Some(*permission),
                _ => None,
            })
        })
    }

    /// The permission the person must hold in some project, if the row says so.
    pub(super) fn held_in_some_project(&self) -> Option<Permission> {
        self.quirks.iter().find_map(|quirk| match quirk {
            Quirk::HeldInSomeProject(permission) => Some(*permission),
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

/// What the routes that change a theme for everyone declare as `by-record`: the use case judges the
/// theme, and one for everyone needs *Change organization settings*; a theme is the look of a
/// person in a browser, so a key never reaches it.
const CHANGES_A_THEME_FOR_EVERYONE: &[Quirk] = &[
    Quirk::SessionOnly,
    Quirk::AtTheOrganization(Permission::ChangeOrganizationSettings),
];

/// Every route of the API description, in the order of the description.
pub(super) const EXAMPLES: &[Example] = &[
    Example::route("getServerStatus"),
    Example::route("acceptInvitation"),
    Example::route("lookUpInvitation"),
    Example::route("listInvitations"),
    Example::route("createInvitation"),
    Example::route("takeBackInvitation"),
    Example::route("getMe"),
    Example::route("signInWithPasskey"),
    Example::route("startPasskeySignIn"),
    Example::with(
        "addPasskey",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with(
        "startAddPasskey",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with(
        "removePasskey",
        &[
            Quirk::NamesAMissingThing,
            Quirk::AlsoAnswers("auth.wrong-credentials"),
        ],
    ),
    Example::with(
        "changePassword",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
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
        &[Quirk::AlsoAnswers("auth.wrong-setup-code")],
    ),
    Example::with("signIn", &[Quirk::AlsoAnswers("auth.wrong-credentials")]),
    Example::with("signOut", &[Quirk::EndsTheSession]),
    Example::with("signUp", &[Quirk::AlsoAnswers("auth.sign-up-refused")]),
    Example::route("getSignUpPolicy"),
    Example::route("changeSignUpPolicy"),
    Example::route("getAuthStatus"),
    Example::route("getTwoStep"),
    Example::with(
        "renewBackupCodes",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with(
        "startCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with(
        "confirmCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with(
        "removeCodeApp",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
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
    Example::route("changeGroup"),
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
    Example::with("resetRole", &[Quirk::AlsoAnswers("access.too-much")]),
    Example::route("listPermissions"),
    Example::route("listAssignments"),
    Example::route("createAssignment"),
    Example::route("listProjectAssignments"),
    Example::route("createProjectAssignment"),
    Example::route("removeAssignment"),
    Example::route("listAssignmentChoices"),
    Example::route("listProjectAssignmentChoices"),
    Example::with(
        "createAccessKey",
        &[Quirk::AlsoAnswers("auth.wrong-credentials")],
    ),
    Example::with("deleteAccessKey", &[Quirk::NamesAMissingThing]),
    Example::route("getAccessKeyRules"),
    Example::route("listMyAgents"),
    Example::with(
        "createAgent",
        &[
            Quirk::AlsoAnswers("name.taken"),
            Quirk::HeldInSomeProject(Permission::CreateAgents),
        ],
    ),
    Example::with("getAgent", &[Quirk::NamesAMissingThing]),
    Example::with("changeAgent", &[Quirk::NamesAMissingThing]),
    Example::with("handAgent", &[Quirk::NamesAMissingThing]),
    Example::with("acceptAgent", &[Quirk::NamesAMissingThing]),
    Example::with("takeBackAgentOffer", &[Quirk::NamesAMissingThing]),
    Example::with(
        "createAgentKey",
        &[
            Quirk::NamesAMissingThing,
            Quirk::AlsoAnswers("auth.wrong-credentials"),
        ],
    ),
    Example::with(
        "renewAgentKey",
        &[
            Quirk::NamesAMissingThing,
            Quirk::AlsoAnswers("auth.wrong-credentials"),
        ],
    ),
    Example::with("deleteAgentKey", &[Quirk::NamesAMissingThing]),
    Example::route("stopAllMyAgents"),
    Example::route("listMyAgentWork"),
    Example::route("listOrganizationAgents"),
    Example::with("handOver", &[Quirk::NamesAMissingThing]),
    Example::with("getIssueAgentWork", &[Quirk::NamesAMissingThing]),
    Example::with("answerAgentQuestion", &[Quirk::NamesAMissingThing]),
    Example::with("approveAgentPlan", &[Quirk::NamesAMissingThing]),
    Example::with("reviewAgentWork", &[Quirk::NamesAMissingThing]),
    Example::with("lookAtAgentWork", &[Quirk::NamesAMissingThing]),
    Example::with("createStandingRule", &[Quirk::NamesAMissingThing]),
    Example::with("changeStandingRule", &[Quirk::NamesAMissingThing]),
    Example::with("listStandingRules", &[Quirk::NamesAMissingThing]),
    Example::with("decideProposal", &[Quirk::NamesAMissingThing]),
    Example::route("listProjectChoices"),
    Example::route("getSecondStepGap"),
    Example::route("listOrganizationSettings"),
    Example::route("changeOrganizationSetting"),
    Example::route("resetOrganizationSetting"),
    Example::route("lockOrganizationSetting"),
    Example::route("unlockOrganizationSetting"),
    Example::route("listProjectSettings"),
    Example::route("changeProjectSetting"),
    Example::route("getProjectAi"),
    Example::route("changeProjectAi"),
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
    Example::with("getEditLock", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::with("renewEditLock", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::route("letGoOfEditLock"),
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
        "letGoOfStuckEditLock",
        &[
            Quirk::AtTheTargetProject,
            Quirk::HidesUnseen,
            Quirk::AlsoNeeds(Permission::ChangeProjectSettings),
            Quirk::NamesAMissingThing,
            Quirk::SessionOnly,
        ],
    ),
    Example::with("saveDraft", &[ON_THE_TARGET, Quirk::SessionOnly]),
    Example::with("getDraft", &[ON_THE_TARGET, Quirk::SessionOnly]),
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
    Example::with("listNeverSentToAi", &[Quirk::KeysActAsThePerson]),
    // The assistant is the panel of a signed-in person: no key, also not the key of an AI app.
    Example::with("getAiGuide", &[Quirk::KeysActAsThePerson]),
    Example::route("getAssistantStatus"),
    Example::route("askAssistant"),
    Example::route("applyAssistantProposal"),
    Example::route("setFieldNeverSentToAi"),
    Example::with("setProjectNeverSentToAi", &[Quirk::HidesUnseen]),
    Example::with(
        "getFormLayout",
        &[Quirk::AtTheTargetProject, Quirk::HidesUnseen],
    ),
    Example::with("getLayoutEditor", &[Quirk::AtTheTargetProject]),
    Example::with("createLayout", &[Quirk::AtTheTargetProject]),
    Example::with("saveLayout", &[Quirk::AtTheTargetProject]),
    Example::with("deleteLayout", &[Quirk::AtTheTargetProject]),
    Example::route("listThemes"),
    Example::with("createTheme", &[Quirk::SessionOnly]),
    Example::route("getTheme"),
    Example::with("saveTheme", CHANGES_A_THEME_FOR_EVERYONE),
    Example::with("deleteTheme", CHANGES_A_THEME_FOR_EVERYONE),
    Example::with("copyTheme", &[Quirk::SessionOnly]),
    Example::route("downloadTheme"),
    Example::with("uploadTheme", &[Quirk::SessionOnly]),
    Example::route("getThemeSheet"),
    Example::with(
        "getThemeDraftSheet",
        &[
            Quirk::SessionOnly,
            Quirk::AtTheOrganization(Permission::ChangeOrganizationSettings),
            Quirk::NamesAMissingThing,
        ],
    ),
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
    // A search lets in whoever is signed in and shows what they read: nothing for a person who
    // sees no project (SEC-MATRIX, the `search` entry point).
    Example::with("searchIssues", &[Quirk::KeysActAsThePerson]),
    Example::with("countSearchedIssues", &[Quirk::KeysActAsThePerson]),
    Example::with("groupSearchedIssues", &[Quirk::KeysActAsThePerson]),
    Example::with("listSearchFields", &[Quirk::KeysActAsThePerson]),
    Example::with("listSearchNames", &[Quirk::KeysActAsThePerson]),
    Example::with("listAssignablePeople", &[Quirk::HidesUnseen]),
    Example::with("createIssue", &[Quirk::NeedsToSeeTheProject]),
    Example::with("getIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("changeIssue", &[Quirk::AnIssueOfTheTarget]),
    Example::with("moveDates", &[Quirk::AnIssueOfTheTarget]),
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
    Example::with("listSavedSearches", &[Quirk::SessionOnly]),
    Example::with("listShareChoices", &[Quirk::SessionOnly]),
    Example::with("createSavedSearch", &[Quirk::SessionOnly]),
    Example::with("getSavedSearch", &[Quirk::SessionOnly]),
    Example::with("changeSavedSearch", &[Quirk::SessionOnly]),
    Example::with("deleteSavedSearch", &[Quirk::SessionOnly]),
    Example::with("restoreSavedSearch", &[Quirk::SessionOnly]),
    Example::with("copySavedSearch", &[Quirk::SessionOnly]),
    Example::with(
        "shareSavedSearch",
        &[
            Quirk::OnTheTargetProject(Permission::ShareSearches),
            Quirk::SessionOnly,
        ],
    ),
    Example::with("stopSharingSavedSearch", &[Quirk::SessionOnly]),
    Example::with(
        "getProjectList",
        &[
            Quirk::OnTheTargetProject(Permission::SeeProjects),
            Quirk::SessionOnly,
        ],
    ),
    Example::with(
        "setProjectList",
        &[Quirk::NeedsToSeeTheProject, Quirk::SessionOnly],
    ),
    Example::route("getMyHandle"),
    Example::route("changeMyHandle"),
    Example::route("listNotifications"),
    Example::route("getUnreadCount"),
    Example::route("markNotificationsRead"),
    Example::route("setMailPassword"),
    Example::route("getMailServer"),
    Example::route("sendTestEmail"),
    Example::route("getAiSetup"),
    Example::route("changeAiSetup"),
    Example::with("setAiKey", &[Quirk::AlsoAnswers("auth.wrong-credentials")]),
    Example::route("confirmAiAgreement"),
    Example::route("getWhatGoesWhere"),
    Example::route("tryAiProvider"),
    Example::route("getAuthorization"),
    Example::route("decideAuthorization"),
    Example::route("listConnectedApps"),
    Example::with("takeBackApp", &[Quirk::NamesAMissingThing]),
    Example::route("listAppAddresses"),
    Example::route("addAppAddress"),
    Example::with("removeAppAddress", &[Quirk::NamesAMissingThing]),
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
        "getRun",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "listRunChanges",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "previewUndoRun",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "undoRun",
        &[Quirk::KeysActAsThePerson, Quirk::NamesAMissingThing],
    ),
    Example::with(
        "saveIssueFieldText",
        &[
            Quirk::AnIssueOfTheTarget,
            Quirk::OnTheTargetProject(Permission::ChangeIssues),
            Quirk::SessionOnly,
        ],
    ),
    Example::with("getTimer", &[Quirk::KeysActAsThePerson]),
    Example::with("startTimer", &[Quirk::AnIssueOfTheTarget]),
    Example::with("stopTimer", &[Quirk::KeysActAsThePerson]),
    Example::with("switchTimer", &[Quirk::KeysActAsThePerson]),
    Example::with("changeTimerStart", &[Quirk::KeysActAsThePerson]),
    Example::with("throwAwayTimer", &[Quirk::KeysActAsThePerson]),
    Example::with("listMyTimeEntries", &[Quirk::KeysActAsThePerson]),
    Example::with("addTimeEntry", &[Quirk::AnIssueOfTheTarget]),
    Example::with(
        "changeTimeEntry",
        &[Quirk::AnIssueOfTheTarget, Quirk::TheirOwn],
    ),
    Example::with(
        "deleteTimeEntry",
        &[Quirk::AnIssueOfTheTarget, Quirk::TheirOwn],
    ),
    Example::with("listIssueTimeEntries", &[Quirk::AnIssueOfTheTarget]),
    Example::route("listMyWorkDays"),
    Example::route("startWorkDay"),
    Example::route("startBreak"),
    Example::route("endBreak"),
    Example::route("endWorkDay"),
    Example::route("changeMyWorkDay"),
    Example::with("getMyWorkDayHistory", &[Quirk::NamesAMissingThing]),
    Example::route("getMyBookedWeek"),
    Example::route("listPeopleOfWorkDays"),
    Example::with("listWorkDaysOfPerson", &[Quirk::HidesThePerson]),
    Example::with(
        "changeWorkDayOfPerson",
        &[
            Quirk::HidesThePerson,
            Quirk::AlsoNeeds(Permission::SeeWorkingHoursOfOthers),
        ],
    ),
    Example::with(
        "getWorkDayHistoryOfPerson",
        &[Quirk::HidesThePerson, Quirk::NamesAMissingThing],
    ),
    Example::route("countOldWorkDays"),
    Example::route("deleteOldWorkDays"),
    Example::with("getTimeEntryDefaults", &[Quirk::AnIssueOfTheTarget]),
    Example::with("getIssueTime", &[Quirk::AnIssueOfTheTarget]),
    Example::with("getProjectTime", &[Quirk::ListsTheIssuesSeen]),
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
    if let Some(found) = agent_request(operation) {
        return Ok(found);
    }
    if let Some(found) = super::examples_agent_work::agent_work_request(operation) {
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
    if let Some(found) = super::examples_ai::ai_request(fixture, operation).await? {
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
    if let Some(found) = super::examples_themes::theme_request(fixture, operation).await? {
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
    if let Some(found) =
        super::examples_searches::search_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) = super::examples_tags::tag_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_workflows::workflow_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_time::time_request(fixture, operation, browser).await? {
        return Ok(found);
    }
    if let Some(found) = super::examples_working_hours::working_hours_request(fixture, operation)? {
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
