//! What each route needs: declared in one place, written into the API description as
//! `x-permission` (M1-F32, ADR-0025 decision 1, architecture §7 *The permission catalogue*).
//!
//! A route declares either a key of the permission catalogue (`manage-invitations`) or one of three
//! words for the routes that need no catalogue permission: `public` (anybody, also nobody signed
//! in), `signed-in` (a signed-in person, nothing more) and `own-account` (a person's own sign-in
//! data: password, passkeys, second step, sessions). The use case behind the route enforces what
//! it declares; ARCH-PERMS (WP-10) reads the description and fails a route without a declaration,
//! and SEC-MATRIX runs every route as every kind of actor.

use bb_policy::Permission;

#[cfg(test)]
use crate::openapi::PERMISSION_MARK;

/// What a route needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declared {
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
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::SignedIn => "signed-in",
            Self::OwnAccount => "own-account",
            Self::Permission(permission) => permission.code(),
        }
    }
}

/// Every route by its operation ID, with what it needs. A new route is added here in the same
/// change that adds it: the test below fails for a route that is missing, and for an entry that
/// names no route.
pub(crate) const ROUTES: &[(&str, Declared)] = &[
    ("getStatus", Declared::Public),
    ("getOpenApi", Declared::Public),
    ("getAuthStatus", Declared::Public),
    ("completeSetup", Declared::Public),
    ("signIn", Declared::Public),
    ("signUp", Declared::Public),
    ("signInWithPasskey", Declared::Public),
    ("startPasskeySignIn", Declared::Public),
    ("lookUpInvitation", Declared::Public),
    ("acceptInvitation", Declared::Public),
    ("getMe", Declared::SignedIn),
    ("signOut", Declared::SignedIn),
    ("signOutEverywhere", Declared::OwnAccount),
    ("listSessions", Declared::OwnAccount),
    ("endSession", Declared::OwnAccount),
    ("changePassword", Declared::OwnAccount),
    ("getTwoStep", Declared::OwnAccount),
    ("startCodeApp", Declared::OwnAccount),
    ("confirmCodeApp", Declared::OwnAccount),
    ("removeCodeApp", Declared::OwnAccount),
    ("renewBackupCodes", Declared::OwnAccount),
    ("startAddPasskey", Declared::OwnAccount),
    ("addPasskey", Declared::OwnAccount),
    ("removePasskey", Declared::OwnAccount),
    ("finishSignInWithCode", Declared::OwnAccount),
    ("startSecondStepWithPasskey", Declared::OwnAccount),
    ("finishSignInWithPasskey", Declared::OwnAccount),
    ("listAccessKeys", Declared::OwnAccount),
    ("makeAccessKey", Declared::OwnAccount),
    ("deleteAccessKey", Declared::OwnAccount),
    ("getAccessKeyRules", Declared::OwnAccount),
    ("listProjectChoices", Declared::OwnAccount),
    (
        "getSecondStepGap",
        Declared::Permission(Permission::ManageRoles),
    ),
    (
        "listInvitations",
        Declared::Permission(Permission::ManageInvitations),
    ),
    (
        "createInvitation",
        Declared::Permission(Permission::ManageInvitations),
    ),
    (
        "revokeInvitation",
        Declared::Permission(Permission::ManageInvitations),
    ),
    (
        "getSignUpPolicy",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "changeSignUpPolicy",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "getServerStatus",
        Declared::Permission(Permission::SeeStatus),
    ),
    (
        "listOrganizationSettings",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "changeOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "resetOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "lockOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "unlockOrganizationSetting",
        Declared::Permission(Permission::ChangeOrganizationSettings),
    ),
    (
        "listProjectSettings",
        Declared::Permission(Permission::ChangeProjectSettings),
    ),
    (
        "changeProjectSetting",
        Declared::Permission(Permission::ChangeProjectSettings),
    ),
    (
        "resetProjectSetting",
        Declared::Permission(Permission::ChangeProjectSettings),
    ),
    ("listMySettings", Declared::SignedIn),
    ("changeMySetting", Declared::SignedIn),
    ("resetMySetting", Declared::SignedIn),
    (
        "listProjects",
        Declared::Permission(Permission::SeeProjects),
    ),
    ("getProject", Declared::Permission(Permission::SeeProjects)),
    (
        "makeProject",
        Declared::Permission(Permission::MakeProjects),
    ),
    (
        "makeSubProject",
        Declared::Permission(Permission::MakeSubProjects),
    ),
    (
        "changeProject",
        Declared::Permission(Permission::ChangeProjectSettings),
    ),
    (
        "previewProjectMove",
        Declared::Permission(Permission::MoveProjects),
    ),
    (
        "moveProject",
        Declared::Permission(Permission::MoveProjects),
    ),
    (
        "getFieldPermissions",
        Declared::Permission(Permission::DecideWhatRolesSee),
    ),
    (
        "changeFieldPermission",
        Declared::Permission(Permission::DecideWhatRolesSee),
    ),
    (
        "getPeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
    ),
    (
        "changePeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
    ),
    ("checkAccess", Declared::Permission(Permission::CheckAccess)),
    (
        "listPeopleToCheck",
        Declared::Permission(Permission::CheckAccess),
    ),
];

/// What the route with this operation ID needs, if it is declared.
pub(crate) fn declared(operation_id: &str) -> Option<Declared> {
    ROUTES
        .iter()
        .find(|(id, _)| *id == operation_id)
        .map(|(_, declared)| *declared)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    use super::*;

    fn document() -> Value {
        serde_json::to_value(crate::openapi::document()).unwrap()
    }

    #[test]
    fn m1_f32_every_route_declares_its_permission_in_the_api_description() {
        let document = document();
        let mut seen = BTreeSet::new();
        for (route, item) in document["paths"].as_object().unwrap() {
            for (method, operation) in item.as_object().unwrap() {
                let id = operation["operationId"].as_str().unwrap();
                let declared = declared(id)
                    .unwrap_or_else(|| panic!("{method} {route} ({id}) declares no permission"));
                assert_eq!(
                    operation[PERMISSION_MARK],
                    declared.key(),
                    "{method} {route} ({id})"
                );
                seen.insert(id.to_owned());
            }
        }
        // And nothing in the table names a route that does not exist.
        for (id, _) in ROUTES {
            assert!(seen.contains(*id), "{id} is declared but is no route");
        }
        assert_eq!(
            seen.len(),
            ROUTES.len(),
            "an operation ID is declared twice"
        );
    }

    #[test]
    fn every_declaration_is_a_key_of_the_catalogue_or_one_of_the_three_words() {
        for (id, declared) in ROUTES {
            let key = declared.key();
            let known = matches!(key, "public" | "signed-in" | "own-account")
                || Permission::from_code(key).is_some();
            assert!(known, "{id}: {key}");
        }
    }

    #[test]
    fn every_permission_that_a_route_asks_for_is_in_the_catalogue_with_its_sentence() {
        // The catalogue has the sentence of each (ARCH-PERMS also wants every permission used by a
        // route in WP-10; M2's own routes use only some of them so far).
        let asked: BTreeSet<&str> = ROUTES
            .iter()
            .filter_map(|(_, declared)| match declared {
                Declared::Permission(permission) => Some(permission.code()),
                _ => None,
            })
            .collect();
        assert!(asked.contains("manage-invitations"));
        assert!(asked.contains("see-status"));
        for key in asked {
            assert!(Permission::from_code(key).is_some());
        }
    }
}
