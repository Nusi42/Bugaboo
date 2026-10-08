//! Two-step sign-in required by role and by the organization (AUTH-03, M1-F45, ADR-0025
//! decision 7): a person who needs a second step and has none can only set one up, from the next
//! request on, also in a session that was open before.

use bb_app::access::Permission;
use bb_app::auth::{
    AccessKeyRulesQuery, AccessKeysQuery, AppError, AuthStore as _, AuthenticateQuery,
    ConfirmCodeApp, DeleteAccessKey, InvitationsQuery, MakeAccessKey, SecondStepGapQuery,
    SessionsQuery, StartCodeApp, TwoStepQuery,
};
use bb_app::context::Clock as _;
use bb_app::roles::{AssignRole, ChangeRole, Given, MakeRole, Subject};
use bb_auth::TotpSecret;
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::access_key::KeyChoice;
use bb_core::id::{AccessKeyId, RoleId, UserId};
use bb_testing::{TestResult, each_database};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::policy_rig::{Member, Org, Run};
use super::rig::{PASSWORD, Rig, refused};

const REQUIRED: Option<ErrorCode> = Some(ErrorCode::SecondStepRequired);

/// The role *Inviter* (it lets people invite), with the switch as given.
pub(super) async fn inviter(org: &Org<'_>, two_step: bool) -> Run<RoleId> {
    role_with(org, Permission::ManageInvitations, "Inviter", two_step).await
}

/// A role with one permission and the switch as given.
pub(super) async fn role_with(
    org: &Org<'_>,
    permission: Permission,
    name: &str,
    two_step: bool,
) -> Run<RoleId> {
    Ok(org
        .rig
        .api()
        .make_role(
            Rig::info(),
            MakeRole {
                session_token: org.admin.session.clone(),
                name: name.to_owned(),
                permissions: vec![permission],
                two_step,
            },
        )
        .await?
        .into_value()
        .id)
}

/// The switch of a role, set by the administrator.
pub(super) async fn switch(org: &Org<'_>, role: RoleId, two_step: bool) -> Run<()> {
    org.rig
        .api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: org.admin.session.clone(),
                role,
                name: None,
                permissions: None,
                two_step: Some(two_step),
            },
        )
        .await?;
    Ok(())
}

pub(super) async fn invitations(rig: &Rig, session: &str) -> Result<(), AppError> {
    rig.api()
        .invitations(
            Rig::info(),
            InvitationsQuery {
                session_token: session.to_owned(),
                cursor: None,
                limit: 10,
            },
        )
        .await
        .map(|_| ())
}

/// Sets up the code app for the person behind `session`.
pub(super) async fn set_up_a_code_app(rig: &Rig, person: UserId, session: &str) -> Run<()> {
    rig.api()
        .start_code_app(
            Rig::info(),
            StartCodeApp {
                session_token: session.to_owned(),
                password: PASSWORD.to_owned(),
            },
        )
        .await?;
    let secret = rig.storage.totp_secret(person).await?.ok_or("no secret")?;
    let code = TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?;
    rig.api()
        .confirm_code_app(
            Rig::info(),
            ConfirmCodeApp {
                session_token: session.to_owned(),
                code,
                password: PASSWORD.to_owned(),
            },
        )
        .await?;
    Ok(())
}

/// What a session that must set up a second step may still do, and what it may not.
async fn only_setting_up_is_open(rig: &Rig, bob: &Member) -> Run<()> {
    let session = bob.session.clone();
    // Nothing that needs a permission, and nothing of the rest of the account.
    assert_eq!(refused(invitations(rig, &session).await), REQUIRED);
    let sessions = rig
        .api()
        .sessions(
            Rig::info(),
            SessionsQuery {
                session_token: session.clone(),
            },
        )
        .await;
    assert_eq!(refused(sessions), REQUIRED);
    let keys = rig
        .api()
        .access_keys(
            Rig::info(),
            AccessKeysQuery {
                session_token: session.clone(),
                cursor: None,
                limit: 10,
            },
        )
        .await;
    assert_eq!(refused(keys), REQUIRED);
    // Nor can it make or delete a key, or ask for the rules of a new one.
    let made = rig
        .api()
        .make_access_key(
            Rig::info(),
            MakeAccessKey {
                session_token: session.clone(),
                password: PASSWORD.to_owned(),
                name: "A key".to_owned(),
                choice: KeyChoice::SeeOnly,
                permissions: Vec::new(),
                projects: Vec::new(),
                expires_at: None,
            },
        )
        .await;
    assert_eq!(refused(made), REQUIRED);
    let deleted = rig
        .api()
        .delete_access_key(
            Rig::info(),
            DeleteAccessKey {
                session_token: session.clone(),
                id: "0197a5e2-7c1f-4b0a-9a3e-2f6d1c8b4a10".parse::<AccessKeyId>()?,
            },
        )
        .await;
    assert_eq!(refused(deleted), REQUIRED);
    let rules = rig
        .api()
        .access_key_rules(
            Rig::info(),
            AccessKeyRulesQuery {
                session_token: session.clone(),
            },
        )
        .await;
    assert_eq!(refused(rules), REQUIRED);
    // Who is behind it is still answered, with nothing they may do and the reason.
    let me = rig
        .api()
        .authenticate(
            Rig::info(),
            AuthenticateQuery {
                session_token: session.clone(),
            },
        )
        .await?;
    assert!(me.second_step_setup_required);
    assert!(me.access.permissions().is_empty());
    // How the second step is set up is open: that is what the person has come to do.
    let status = rig
        .api()
        .two_step(
            Rig::info(),
            TwoStepQuery {
                session_token: session,
            },
        )
        .await?;
    assert!(!status.code_app && status.passkeys.is_empty());
    Ok(())
}

#[tokio::test]
async fn auth_03_a_role_that_requires_two_step_sign_in_makes_its_people_set_it_up_first()
-> TestResult {
    each_database("auth_03_role_requires_two_step", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let role = inviter(&org, false).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, role).await?;
        // Bob works with his password alone, and his session is open.
        assert_eq!(refused(invitations(&rig, &bob.session).await), None);

        // The administrator switches the role's switch on. From Bob's next request, in the same
        // session, he can only set up a second step.
        switch(&org, role, true).await?;
        only_setting_up_is_open(&rig, &bob).await?;
        // A new sign-in is the same: the password opens a session that can only do that.
        let again = rig.sign_in("person1@example.test", PASSWORD).await?;
        assert!(
            again.second_step_needed.is_none(),
            "he has no second step to ask for"
        );
        assert_eq!(
            refused(invitations(&rig, &again.session_token).await),
            REQUIRED
        );

        // Once the code app is set up, the very next request works: no new sign-in needed.
        set_up_a_code_app(&rig, bob.id, &bob.session).await?;
        assert_eq!(refused(invitations(&rig, &bob.session).await), None);
        let me = rig
            .api()
            .authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: bob.session.clone(),
                },
            )
            .await?;
        assert!(!me.second_step_setup_required);
        assert!(
            me.access
                .permissions()
                .contains(&Permission::ManageInvitations)
        );
        // And from now on signing in asks for the second step, like for anybody who has one.
        let next = rig.sign_in("person1@example.test", PASSWORD).await?;
        assert!(next.second_step_needed.is_some());

        // The switch off again asks nothing of people who have no second step.
        let carol = org.person().await?;
        org.give_everywhere(&carol, role).await?;
        assert_eq!(
            refused(invitations(&rig, &carol.session).await),
            REQUIRED,
            "the switch is on for the role"
        );
        switch(&org, role, false).await?;
        assert_eq!(refused(invitations(&rig, &carol.session).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_passkey_is_a_second_step_too() -> TestResult {
    each_database("auth_03_passkey_ends_the_requirement", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let role = inviter(&org, true).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, role).await?;
        only_setting_up_is_open(&rig, &bob).await?;
        super::passkeys::add(&rig, &bob.session, &mut SoftPasskey::new(true), "Phone").await?;
        assert_eq!(refused(invitations(&rig, &bob.session).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_role_held_through_a_group_or_on_a_project_counts_too() -> TestResult {
    each_database("auth_03_group_and_project_roles", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let role = inviter(&org, true).await?;
        // Through a group, for the whole organization.
        let in_group = org.person().await?;
        let group = org.group("Inviters").await?;
        org.add_to_group(group, in_group.id).await?;
        assert_eq!(
            refused(invitations(&rig, &in_group.session).await),
            Some(ErrorCode::Forbidden),
            "a group without the role asks nothing yet"
        );
        org.rig
            .api()
            .assign_role(
                Rig::info(),
                AssignRole {
                    session_token: org.admin.session.clone(),
                    subject: Subject::Group(group),
                    given: Given::Role(role),
                    project: None,
                    applies_to: AppliesTo::Organization,
                },
            )
            .await?;
        assert_eq!(
            refused(invitations(&rig, &in_group.session).await),
            REQUIRED
        );
        // On a project only: a role they hold anywhere is enough.
        let on_a_project = org.person().await?;
        let project = org.project(None, "WEB").await?;
        org.give(&on_a_project, role, project.id).await?;
        assert_eq!(
            refused(invitations(&rig, &on_a_project.session).await),
            REQUIRED
        );
        // Somebody without the role is asked nothing.
        let free = org.person().await?;
        assert_eq!(
            refused(invitations(&rig, &free.session).await),
            Some(ErrorCode::Forbidden)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_the_organization_switch_asks_everybody_for_a_second_step() -> TestResult {
    each_database("auth_03_organization_switch", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let role = inviter(&org, false).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, role).await?;
        assert_eq!(refused(invitations(&rig, &bob.session).await), None);
        assert_eq!(refused(invitations(&rig, &org.admin.session).await), None);
        rig.set_organization_switch("security.two-step-required", true)
            .await?;
        // Everybody, the administrator too, from the next request.
        only_setting_up_is_open(&rig, &bob).await?;
        only_setting_up_is_open(&rig, &org.admin).await?;
        set_up_a_code_app(&rig, bob.id, &bob.session).await?;
        assert_eq!(refused(invitations(&rig, &bob.session).await), None);
        assert_eq!(
            refused(invitations(&rig, &org.admin.session).await),
            REQUIRED
        );
        rig.set_organization_switch("security.two-step-required", false)
            .await?;
        assert_eq!(refused(invitations(&rig, &org.admin.session).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_the_administrator_sees_how_many_people_of_a_role_still_have_no_second_step()
-> TestResult {
    each_database("auth_03_second_step_gap", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let role = inviter(&org, false).await?;
        let (bob, carol, dora) = (
            org.person().await?,
            org.person().await?,
            org.person().await?,
        );
        org.give_everywhere(&bob, role).await?;
        // Carol has the role through a group, and Dora does not have it.
        let group = org.group("Inviters").await?;
        org.add_to_group(group, carol.id).await?;
        org.rig
            .api()
            .assign_role(
                Rig::info(),
                AssignRole {
                    session_token: org.admin.session.clone(),
                    subject: Subject::Group(group),
                    given: Given::Role(role),
                    project: None,
                    applies_to: AppliesTo::Organization,
                },
            )
            .await?;
        let gap = |session: String| {
            let rig = &rig;
            async move {
                rig.api()
                    .second_step_gap(
                        Rig::info(),
                        SecondStepGapQuery {
                            session_token: session,
                            role,
                        },
                    )
                    .await
                    .map(|gap| gap.into_value().still_to_set_up)
            }
        };
        assert_eq!(gap(org.admin.session.clone()).await?, 2);
        set_up_a_code_app(&rig, bob.id, &bob.session).await?;
        assert_eq!(gap(org.admin.session.clone()).await?, 1);
        // Only those who manage roles ask; Dora does not.
        assert_eq!(
            refused(gap(dora.session.clone()).await),
            Some(ErrorCode::Forbidden)
        );
        // A role that does not exist is not found.
        let missing = rig
            .api()
            .second_step_gap(
                Rig::info(),
                SecondStepGapQuery {
                    session_token: org.admin.session.clone(),
                    role: RoleId::from_parts(rig.clock_now(), [3; 10])?,
                },
            )
            .await;
        assert_eq!(refused(missing), Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}
