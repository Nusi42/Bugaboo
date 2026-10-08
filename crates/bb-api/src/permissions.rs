//! What each route needs and what kind of answer it gives: declared in one place, written into the
//! API description by `crate::openapi` (M1-F32, ADR-0025 decision 1, architecture §7 *The
//! permission catalogue*, *Cache rules in one place*).
//!
//! A route declares either a key of the permission catalogue (`manage-invitations`) or one of three
//! words for the routes that need no catalogue permission: `public` (anybody, also nobody signed
//! in), `signed-in` (a signed-in person, nothing more) and `own-account` (a person's own sign-in
//! data: password, passkeys, second step, sessions). The use case behind the route enforces what
//! it declares; ARCH-PERMS reads the description and fails a route without a declaration, and
//! SEC-MATRIX runs every route as every kind of actor.
//!
//! The same row says what kind of answer the route gives ([`Answer`]): `crate::etag` decides the
//! cache rules from it. The marks `x-permission`, `x-never-stored` and `x-secret` are written from
//! this table only; no handler writes one by hand. Whether a change needs the token of the
//! session (CSRF) follows from the row too: only the routes that are `public` and change
//! something are open without one ([`csrf_exempt`]).

use std::sync::OnceLock;

use axum::http::Method;
use bb_api_words::needs;
use bb_policy::Permission;

/// What a route needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declared {
    /// Anybody, also nobody signed in.
    Public,
    /// A signed-in person, nothing more.
    SignedIn,
    /// A person's own sign-in data.
    OwnAccount,
    /// A permission of the catalogue.
    Permission(Permission),
}

impl Declared {
    /// The value of `x-permission`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Public => needs::PUBLIC,
            Self::SignedIn => needs::SIGNED_IN,
            Self::OwnAccount => needs::OWN_ACCOUNT,
            Self::Permission(permission) => permission.code(),
        }
    }
}

/// What a route's answer is, which decides how it may be kept (PERF-03, `crate::etag`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// A read without personal data: a browser may keep it privately and asks again every time
    /// (`private, no-cache`, with an `ETag`).
    Kept,
    /// A read with personal data (`x-never-stored`): `no-store`, no tag.
    NeverStored,
    /// An answer that hands out a secret (`x-secret`): `no-store`, no tag, never compressed.
    Secret,
    /// A change: its answer is never kept.
    Change,
}

/// One row of the table: the route with this operation ID needs `needs` and gives an answer of kind `kind`.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    /// The operation ID of the route.
    pub id: &'static str,
    /// What the route needs.
    pub needs: Declared,
    /// What kind of answer it gives.
    pub kind: Answer,
}

const fn route(id: &'static str, needs: Declared, kind: Answer) -> Route {
    Route { id, needs, kind }
}

/// Every route by its operation ID. A new route is added here in the same change that adds it: the
/// test below fails for an entry that names no route, and ARCH-PERMS fails for a route that has no
/// entry.
pub const ROUTES: &[Route] = &[
    route("getStatus", Declared::Public, Answer::Kept),
    route("getOpenApi", Declared::Public, Answer::Kept),
    route("getAuthStatus", Declared::Public, Answer::Kept),
    route("completeSetup", Declared::Public, Answer::Change),
    route("signIn", Declared::Public, Answer::Change),
    route("signUp", Declared::Public, Answer::Change),
    route("signInWithPasskey", Declared::Public, Answer::Change),
    route("startPasskeySignIn", Declared::Public, Answer::Change),
    route("lookUpInvitation", Declared::Public, Answer::Change),
    route("acceptInvitation", Declared::Public, Answer::Change),
    route("getMe", Declared::SignedIn, Answer::NeverStored),
    route("signOut", Declared::SignedIn, Answer::Change),
    route("signOutEverywhere", Declared::OwnAccount, Answer::Change),
    route("listSessions", Declared::OwnAccount, Answer::NeverStored),
    route("endSession", Declared::OwnAccount, Answer::Change),
    route("changePassword", Declared::OwnAccount, Answer::Change),
    route("getTwoStep", Declared::OwnAccount, Answer::NeverStored),
    route("startCodeApp", Declared::OwnAccount, Answer::Secret),
    route("confirmCodeApp", Declared::OwnAccount, Answer::Secret),
    route("removeCodeApp", Declared::OwnAccount, Answer::Change),
    route("renewBackupCodes", Declared::OwnAccount, Answer::Secret),
    route("startAddPasskey", Declared::OwnAccount, Answer::Change),
    route("addPasskey", Declared::OwnAccount, Answer::Change),
    route("removePasskey", Declared::OwnAccount, Answer::Change),
    route("finishSignInWithCode", Declared::OwnAccount, Answer::Change),
    route(
        "startSecondStepWithPasskey",
        Declared::OwnAccount,
        Answer::Change,
    ),
    route(
        "finishSignInWithPasskey",
        Declared::OwnAccount,
        Answer::Change,
    ),
    route("listAccessKeys", Declared::OwnAccount, Answer::NeverStored),
    route("makeAccessKey", Declared::OwnAccount, Answer::Secret),
    route("deleteAccessKey", Declared::OwnAccount, Answer::Change),
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
    route(
        "getSecondStepGap",
        Declared::Permission(Permission::ManageRoles),
        Answer::NeverStored,
    ),
    route(
        "listInvitations",
        Declared::Permission(Permission::ManageInvitations),
        Answer::NeverStored,
    ),
    route(
        "createInvitation",
        Declared::Permission(Permission::ManageInvitations),
        Answer::Secret,
    ),
    route(
        "revokeInvitation",
        Declared::Permission(Permission::ManageInvitations),
        Answer::Change,
    ),
    route(
        "getSignUpPolicy",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Kept,
    ),
    route(
        "changeSignUpPolicy",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Change,
    ),
    route(
        "getServerStatus",
        Declared::Permission(Permission::SeeStatus),
        Answer::NeverStored,
    ),
    route(
        "listOrganizationSettings",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "changeOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Change,
    ),
    route(
        "resetOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Change,
    ),
    route(
        "lockOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Change,
    ),
    route(
        "unlockOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Change,
    ),
    route(
        "listProjectSettings",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::NeverStored,
    ),
    route(
        "changeProjectSetting",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Change,
    ),
    route(
        "resetProjectSetting",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Change,
    ),
    route("listMySettings", Declared::SignedIn, Answer::NeverStored),
    route("checkOwnAccessChange", Declared::SignedIn, Answer::Change),
    route("changeMySetting", Declared::SignedIn, Answer::Change),
    route("resetMySetting", Declared::SignedIn, Answer::Change),
    route(
        "listProjects",
        Declared::Permission(Permission::SeeProjects),
        Answer::NeverStored,
    ),
    route(
        "getProject",
        Declared::Permission(Permission::SeeProjects),
        Answer::NeverStored,
    ),
    route(
        "makeProject",
        Declared::Permission(Permission::MakeProjects),
        Answer::Change,
    ),
    route(
        "makeSubProject",
        Declared::Permission(Permission::MakeSubProjects),
        Answer::Change,
    ),
    route(
        "changeProject",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Change,
    ),
    route(
        "previewProjectMove",
        Declared::Permission(Permission::MoveProjects),
        Answer::Change,
    ),
    route(
        "moveProject",
        Declared::Permission(Permission::MoveProjects),
        Answer::Change,
    ),
    route(
        "listPeople",
        Declared::Permission(Permission::ManagePeople),
        Answer::NeverStored,
    ),
    route(
        "getPerson",
        Declared::Permission(Permission::ManagePeople),
        Answer::NeverStored,
    ),
    route(
        "listGroups",
        Declared::Permission(Permission::ManageGroups),
        Answer::NeverStored,
    ),
    route(
        "makeGroup",
        Declared::Permission(Permission::ManageGroups),
        Answer::Change,
    ),
    route(
        "getGroup",
        Declared::Permission(Permission::ManageGroups),
        Answer::NeverStored,
    ),
    route(
        "renameGroup",
        Declared::Permission(Permission::ManageGroups),
        Answer::Change,
    ),
    route(
        "deleteGroup",
        Declared::Permission(Permission::ManageGroups),
        Answer::Change,
    ),
    route(
        "listGroupMembers",
        Declared::Permission(Permission::ManageGroups),
        Answer::NeverStored,
    ),
    route(
        "addGroupMember",
        Declared::Permission(Permission::ManageGroups),
        Answer::Change,
    ),
    route(
        "removeGroupMember",
        Declared::Permission(Permission::ManageGroups),
        Answer::Change,
    ),
    route(
        "listMemberChoices",
        Declared::Permission(Permission::ManageGroups),
        Answer::NeverStored,
    ),
    route(
        "listRoles",
        Declared::Permission(Permission::ManageRoles),
        Answer::NeverStored,
    ),
    route(
        "makeRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::Change,
    ),
    route(
        "getRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::NeverStored,
    ),
    route(
        "changeRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::Change,
    ),
    route(
        "deleteRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::Change,
    ),
    route(
        "copyRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::Change,
    ),
    route(
        "resetRole",
        Declared::Permission(Permission::ManageRoles),
        Answer::Change,
    ),
    route(
        "listPermissions",
        Declared::Permission(Permission::ManageRoles),
        Answer::NeverStored,
    ),
    route(
        "listAssignments",
        Declared::Permission(Permission::DecideRoles),
        Answer::NeverStored,
    ),
    route(
        "assignRole",
        Declared::Permission(Permission::DecideRoles),
        Answer::Change,
    ),
    route(
        "removeAssignment",
        Declared::Permission(Permission::DecideRoles),
        Answer::Change,
    ),
    route(
        "listAssignmentChoices",
        Declared::Permission(Permission::DecideRoles),
        Answer::NeverStored,
    ),
    route(
        "listProjectAssignments",
        Declared::Permission(Permission::DecideRoles),
        Answer::NeverStored,
    ),
    route(
        "assignProjectRole",
        Declared::Permission(Permission::DecideRoles),
        Answer::Change,
    ),
    route(
        "listProjectAssignmentChoices",
        Declared::Permission(Permission::DecideRoles),
        Answer::NeverStored,
    ),
    route(
        "getFieldPermissions",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::NeverStored,
    ),
    route(
        "changeFieldPermission",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::Change,
    ),
    route(
        "getPeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::NeverStored,
    ),
    route(
        "changePeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::Change,
    ),
    route(
        "checkAccess",
        Declared::Permission(Permission::CheckAccess),
        Answer::NeverStored,
    ),
    route(
        "listPeopleToCheck",
        Declared::Permission(Permission::CheckAccess),
        Answer::NeverStored,
    ),
];

/// The row of the route with this operation ID, if it is declared.
#[must_use]
pub fn declared(operation_id: &str) -> Option<&'static Route> {
    ROUTES.iter().find(|row| row.id == operation_id)
}

/// Whether a request with this method to this address is open without the token of a session: the
/// routes that are `public` and change something. People who sign in or sign up have no session
/// yet; they are protected by the origin check and carry no secret a forged request could use
/// (SEC-12).
#[must_use]
pub fn csrf_exempt(method: &Method, path: &str) -> bool {
    static OPEN: OnceLock<Vec<(Method, &'static str)>> = OnceLock::new();
    OPEN.get_or_init(|| {
        crate::openapi::operations()
            .iter()
            .filter(|operation| operation.needs == Declared::Public && operation.changes())
            .map(|operation| (operation.method.clone(), operation.path.as_str()))
            .collect()
    })
    .iter()
    .any(|(open_method, open_path)| open_method == method && *open_path == path)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::openapi::operations;

    #[test]
    fn m1_f32_no_row_names_a_route_that_does_not_exist_or_a_route_twice() {
        let ids: BTreeSet<&str> = operations()
            .iter()
            .map(|operation| operation.id.as_str())
            .collect();
        for row in ROUTES {
            assert!(
                ids.contains(row.id),
                "{} is declared but is no route",
                row.id
            );
        }
        let rows: BTreeSet<&str> = ROUTES.iter().map(|row| row.id).collect();
        assert_eq!(
            rows.len(),
            ROUTES.len(),
            "an operation ID is declared twice"
        );
    }

    #[test]
    fn perf_03_every_read_decides_whether_it_may_be_kept_and_a_change_never_is() {
        for operation in operations() {
            let reads = operation.method == Method::GET;
            match operation.kind {
                Answer::Kept | Answer::NeverStored => assert!(
                    reads,
                    "{} {} is a change: its answer is not kept",
                    operation.method, operation.path
                ),
                Answer::Change => assert!(
                    !reads,
                    "GET {} is a read: it is either kept privately or never stored",
                    operation.path
                ),
                Answer::Secret => {}
            }
        }
    }

    #[test]
    fn sec_12_only_public_changes_are_open_without_a_token_and_they_name_one_exact_address() {
        let mut open = 0;
        for operation in operations() {
            let exempt = csrf_exempt(&operation.method, &operation.path);
            let public_change = operation.changes() && operation.needs == Declared::Public;
            assert_eq!(
                exempt, public_change,
                "{} {}",
                operation.method, operation.path
            );
            if exempt {
                open += 1;
                assert_eq!(operation.method, Method::POST, "{}", operation.path);
                // The address is compared as it is written, so it has no part that varies.
                assert!(!operation.path.contains('{'), "{}", operation.path);
            }
        }
        assert!(open > 0, "no route is open without a token");
    }
}
