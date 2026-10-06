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

use bb_app::auth::CreateInvitation;
use serde_json::{Value, json};

use super::actors::{Actor, Project};
use super::routes::Fixture;
use crate::sign_in_tests::http::{Browser, Failure};
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
    /// The call ends the session of whoever makes it, so they sign in again afterwards.
    EndsTheSession,
    /// The route names a thing that is not there (a passkey nobody added), so a person who is let
    /// in is told `resource.not_found`, which is not a refusal of the permission.
    NamesAMissingThing,
    /// The route finishes a sign-in that is half done (the second step): a session that is whole
    /// has nothing to finish, so it is told it is not signed in for this.
    FinishesAHalfDoneSignIn,
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
    Example::route("addPasskey"),
    Example::route("startAddPasskey"),
    Example::with("removePasskey", &[Quirk::NamesAMissingThing]),
    Example::route("changePassword"),
    Example::with("finishSignInWithCode", &[Quirk::FinishesAHalfDoneSignIn]),
    Example::with("finishSignInWithPasskey", &[Quirk::FinishesAHalfDoneSignIn]),
    Example::with(
        "startSecondStepWithPasskey",
        &[Quirk::FinishesAHalfDoneSignIn],
    ),
    Example::route("listSessions"),
    Example::with("signOutEverywhere", &[Quirk::EndsTheSession]),
    Example::with("endSession", &[Quirk::EndsTheSession]),
    Example::route("completeSetup"),
    Example::route("signIn"),
    Example::with("signOut", &[Quirk::EndsTheSession]),
    Example::route("signUp"),
    Example::route("getSignUpPolicy"),
    Example::route("changeSignUpPolicy"),
    Example::route("getAuthStatus"),
    Example::route("getTwoStep"),
    Example::route("renewBackupCodes"),
    Example::route("startCodeApp"),
    Example::route("confirmCodeApp"),
    Example::route("removeCodeApp"),
    Example::route("getOpenApi"),
    Example::with("listProjects", &[Quirk::FiltersWhatIsSeen]),
    Example::route("makeProject"),
    Example::with("getProject", &[Quirk::HidesUnseen]),
    Example::with("changeProject", &[Quirk::NeedsToSeeTheProject]),
    Example::with("moveProject", &[Quirk::HidesUnseen]),
    Example::with("previewProjectMove", &[Quirk::HidesUnseen]),
    Example::route("makeSubProject"),
    Example::route("getStatus"),
    Example::route("listAccessKeys"),
    Example::route("makeAccessKey"),
    Example::with("deleteAccessKey", &[Quirk::NamesAMissingThing]),
    Example::route("getAccessKeyRules"),
    Example::route("listProjectChoices"),
    Example::route("getSecondStepGap"),
];

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
type Request = (&'static str, String, Option<Value>);

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
        // Whoever is signed in.
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
        "makeAccessKey" => (
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
        "getSignUpPolicy" => ("GET", "/api/v1/auth/sign-up-policy".into(), None),
        "changeSignUpPolicy" => (
            "PUT",
            "/api/v1/auth/sign-up-policy".into(),
            Some(json!({"enabled": false, "domains": []})),
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
        "makeProject" => (
            "POST",
            "/api/v1/projects".into(),
            Some(json!({"short_name": unique_short_name(fixture), "name": "Made by the matrix"})),
        ),
        "getProject" => ("GET", format!("/api/v1/projects/{target}"), None),
        "changeProject" => (
            "PATCH",
            format!("/api/v1/projects/{target}"),
            Some(json!({"version": 1, "name": "Renamed by the matrix"})),
        ),
        "makeSubProject" => (
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
