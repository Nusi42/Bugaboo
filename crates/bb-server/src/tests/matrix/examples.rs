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

/// How a route behaves at the edges the API description does not say.
#[derive(Clone, Copy, Debug)]
pub(super) struct Example {
    /// The operation ID of the route in the API description.
    pub(super) operation: &'static str,
    /// The route answers a project that the actor cannot see with `resource.not_found` before it
    /// looks at the permission, also an actor who knows it exists (they hold *Decide who has which
    /// role* there): it is about what is inside the project.
    pub(super) hides_unseen: bool,
    /// The route looks at the permission first, and then answers `resource.not_found` to somebody
    /// who holds it but does not see the project: changing what one cannot see is not possible.
    pub(super) needs_to_see_the_project: bool,
    /// The route lists what the actor may see: a person without the permission is not refused but
    /// gets what they see, which may be nothing (M2-F30). Only the routes named here may do that.
    pub(super) filters_what_is_seen: bool,
    /// The call ends the session of whoever makes it, so they sign in again afterwards.
    pub(super) ends_the_session: bool,
    /// The route names a thing that is not there (a passkey nobody added), so a person who is let
    /// in is told `resource.not_found`, which is not a refusal of the permission.
    pub(super) names_a_missing_thing: bool,
    /// The route finishes a sign-in that is half done (the second step): a session that is whole
    /// has nothing to finish, so it is told it is not signed in for this.
    pub(super) finishes_a_half_done_sign_in: bool,
}

impl Example {
    const fn route(operation: &'static str) -> Self {
        Self {
            operation,
            hides_unseen: false,
            needs_to_see_the_project: false,
            filters_what_is_seen: false,
            ends_the_session: false,
            names_a_missing_thing: false,
            finishes_a_half_done_sign_in: false,
        }
    }

    const fn hides_unseen(mut self) -> Self {
        self.hides_unseen = true;
        self
    }

    const fn needs_to_see_the_project(mut self) -> Self {
        self.needs_to_see_the_project = true;
        self
    }

    const fn filters_what_is_seen(mut self) -> Self {
        self.filters_what_is_seen = true;
        self
    }

    const fn ends_the_session(mut self) -> Self {
        self.ends_the_session = true;
        self
    }

    const fn names_a_missing_thing(mut self) -> Self {
        self.names_a_missing_thing = true;
        self
    }

    const fn finishes_a_half_done_sign_in(mut self) -> Self {
        self.finishes_a_half_done_sign_in = true;
        self
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
    Example::route("removePasskey").names_a_missing_thing(),
    Example::route("changePassword"),
    Example::route("finishSignInWithCode").finishes_a_half_done_sign_in(),
    Example::route("finishSignInWithPasskey").finishes_a_half_done_sign_in(),
    Example::route("startSecondStepWithPasskey").finishes_a_half_done_sign_in(),
    Example::route("listSessions"),
    Example::route("signOutEverywhere").ends_the_session(),
    Example::route("endSession").ends_the_session(),
    Example::route("completeSetup"),
    Example::route("signIn"),
    Example::route("signOut").ends_the_session(),
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
    Example::route("listProjects").filters_what_is_seen(),
    Example::route("makeProject"),
    Example::route("getProject").hides_unseen(),
    Example::route("changeProject").needs_to_see_the_project(),
    Example::route("moveProject").hides_unseen(),
    Example::route("previewProjectMove").hides_unseen(),
    Example::route("makeSubProject"),
    Example::route("getStatus"),
    Example::route("listAccessKeys"),
    Example::route("makeAccessKey"),
    Example::route("deleteAccessKey").names_a_missing_thing(),
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
) -> Result<(&'static str, String, Option<Value>), Failure> {
    let target = fixture.project(Project::Target).to_string();
    let sibling = fixture.project(Project::Sibling).to_string();
    let wrong = "not the password";
    Ok(match example.operation {
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
        // How many people with a role still have to set up a second step.
        "getSecondStepGap" => (
            "GET",
            format!("/api/v1/roles/{}/second-step-gap", fixture.a_role()?),
            None,
        ),
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
        other => return Err(format!("{other} has a row in the table but no request").into()),
    })
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
            .filter(|example| example.filters_what_is_seen)
            .map(|example| example.operation)
            .collect();
        assert_eq!(
            filtering,
            ["listProjects"],
            "M2-F30: one route, with its reason"
        );
    }
}
