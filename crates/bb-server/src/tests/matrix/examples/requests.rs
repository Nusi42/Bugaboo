//! The requests the matrix sends for an example: what each kind of route is asked with.

use super::*;

/// A route that judges the record it names, as the example names the target project.
pub(super) const ON_THE_TARGET: Quirk =
    Quirk::OnTheTargetProject(Permission::ChangeProjectSettings);

/// A route of the person's own state about an issue: the issue must be one they may see.
pub(super) const SEES_THE_ISSUE: Quirk = Quirk::OnTheTargetProject(Permission::SeeIssues);

/// An ID that no invitation, session, passkey or access key has (the random kind: a time-ordered
/// one would not be an ID of these, and the route would say "not found" before it looked at
/// anything).
pub(super) const NOBODYS_RANDOM_ID: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// A short name that no other call of the matrix used, for a project to make.
pub(super) fn unique_short_name(fixture: &Fixture<'_>) -> String {
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

/// The routes of everybody and of whoever is signed in.
pub(super) fn open_request(operation: &str) -> Result<Option<Request>, Failure> {
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
        // The person's own notifications.
        "listNotifications" => ("GET", "/api/v1/notifications".into(), None),
        "getUnreadNotifications" => ("GET", "/api/v1/notifications/unread".into(), None),
        "readNotifications" => (
            "POST",
            "/api/v1/notifications/read".into(),
            Some(json!({"all": true})),
        ),
        "signOut" => ("POST", "/api/v1/auth/sign-out".into(), None),
        _ => return Ok(None),
    }))
}

/// A person's own sign-in data and access keys.
pub(super) async fn own_request(
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
pub(super) async fn organization_request(
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
        "setMailPassword" => (
            "PUT",
            "/api/v1/mail/password".into(),
            Some(json!({"password": "a made-up mail password"})),
        ),
        // No mail server is entered in the matrix, so the test e-mail is turned away for that.
        "sendTestEmail" => ("POST", "/api/v1/mail/test".into(), None),
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
pub(super) fn project_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
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
