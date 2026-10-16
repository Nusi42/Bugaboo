//! What a key can never do, and when keys end (AUTH-07, ADR-0025 decision 8, decisions on
//! access keys): a key never invites people or changes who has which role, whatever its choice, and
//! *Sign out everywhere* ends the keys of the person while a password change leaves them.

use bb_app::access::{NOT_FOR_KEYS, Permission};
use bb_app::auth::{
    AccessKeyRulesQuery, AppError, ChangePassword, ChangeSignUpPolicy, CreateAccessKey,
    CreateInvitation, SignOutEverywhere, SignUpPolicy, SignUpPolicyQuery,
};
use bb_app::roles::ChangeRole;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::access_keys::{bob, check, key_of, made};
use super::policy_rig::Org;
use super::rig::{PASSWORD, Rig, Tries, refused};
use super::second_step_by_role::invitations;

async fn invite(rig: &Rig, credential: &str) -> Result<(), AppError> {
    rig.api()
        .create_invitation(
            Rig::info(),
            CreateInvitation {
                session_token: credential.to_owned(),
                email: "new.person@example.test".to_owned(),
            },
        )
        .await
        .map(|_| ())
}

/// Reads and changes who may sign up on their own: an organization setting.
async fn settings_of_the_organization(rig: &Rig, credential: &str) -> Result<(), AppError> {
    rig.api()
        .sign_up_policy(
            Rig::info(),
            SignUpPolicyQuery {
                session_token: credential.to_owned(),
            },
        )
        .await?;
    rig.api()
        .change_sign_up_policy(
            Rig::info(),
            ChangeSignUpPolicy {
                session_token: credential.to_owned(),
                policy: SignUpPolicy {
                    enabled: true,
                    domains: Vec::new(),
                    version: 0,
                },
            },
        )
        .await
        .map(|_| ())
}

async fn change_a_role(
    rig: &Rig,
    credential: &str,
    role: bb_core::id::RoleId,
) -> Result<(), AppError> {
    let version = rig
        .role_version(role)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    rig.api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: credential.to_owned(),
                role,
                version,
                name: Some("Renamed".to_owned()),
                permissions: None,
                two_step: None,
            },
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn auth_07_a_key_never_invites_people_or_changes_roles_whatever_its_choice() -> TestResult {
    each_database("auth_07_keys_never_invite_or_change_roles", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let role = org.role("Helper", &[Permission::SeeProjects]).await?;
        // The administrator can do all of it with the session ...
        assert!(invite(&rig, &org.admin.session).await.is_ok());
        assert!(
            change_a_role(&rig, &org.admin.session, role.id)
                .await
                .is_ok()
        );
        // ... and with a key of any choice, none of it: not the one that says everything.
        let everything = key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let see_and_change = key_of(&rig, &org.admin, KeyChoice::SeeAndChange).await?;
        for key in [&everything, &see_and_change] {
            assert_eq!(refused(invite(&rig, key).await), Some(ErrorCode::Forbidden));
            assert_eq!(
                refused(change_a_role(&rig, key, role.id).await),
                Some(ErrorCode::Forbidden)
            );
            assert_eq!(
                refused(invitations(&rig, key).await),
                Some(ErrorCode::Forbidden)
            );
        }
        // Nor the settings of the organization: a key cannot turn off the two-step requirement,
        // open signing up or change how long sessions last.
        assert!(
            settings_of_the_organization(&rig, &org.admin.session)
                .await
                .is_ok()
        );
        for key in [&everything, &see_and_change] {
            assert_eq!(
                refused(settings_of_the_organization(&rig, key).await),
                Some(ErrorCode::Forbidden)
            );
        }
        // What a key may do goes on working: asking who may do what.
        assert!(
            check(&rig, &everything, org.admin.id, None).await.is_ok(),
            "a key that sees and checks still works"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_single_permissions_of_a_key_never_include_those_a_key_never_holds() -> TestResult {
    each_database(
        "auth_07_single_permissions_exclude_the_never_ones",
        async |db| {
            let rig = Rig::start_with(db, Tries::Roomy).await?;
            let org = Org::start(&rig).await?;
            // The key form does not offer them ...
            let rules = rig
                .api()
                .access_key_rules(
                    Rig::info(),
                    AccessKeyRulesQuery {
                        session_token: org.admin.session.clone(),
                    },
                )
                .await?
                .into_value();
            let offered: Vec<Permission> = rules.permissions.iter().map(|(p, _)| *p).collect();
            assert!(NOT_FOR_KEYS.iter().all(|never| !offered.contains(never)));
            assert!(offered.contains(&Permission::SeeProjects));
            // ... and the server does not make a key that asks for one.
            for never in NOT_FOR_KEYS {
                let asked = rig
                    .api()
                    .create_access_key(
                        Rig::info(),
                        CreateAccessKey {
                            session_token: org.admin.session.clone(),
                            password: PASSWORD.to_owned(),
                            name: "Inviting".to_owned(),
                            choice: KeyChoice::Permissions,
                            permissions: vec![*never],
                            projects: Vec::new(),
                            expires_at: None,
                        },
                    )
                    .await;
                assert_eq!(refused(asked), Some(ErrorCode::InvalidRequest), "{never}");
            }
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn auth_07_sign_out_everywhere_ends_the_keys_and_a_password_change_leaves_them() -> TestResult
{
    each_database("auth_07_sign_out_everywhere_ends_keys", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let first = made(&rig, &bob, KeyChoice::SeeOnly, None).await?;
        let second = made(&rig, &bob, KeyChoice::SeeOnly, None).await?;
        let other = made(&rig, &org.admin, KeyChoice::SeeOnly, None).await?;
        assert!(check(&rig, &first.secret, bob.id, None).await.is_ok());

        // A password change ends the other sessions, and the keys stay.
        let changed = rig
            .api()
            .change_password(
                Rig::info(),
                ChangePassword {
                    session_token: bob.session.clone(),
                    current_password: PASSWORD.to_owned(),
                    new_password: "Another-Long-Password-7".to_owned(),
                },
            )
            .await?;
        assert!(check(&rig, &first.secret, bob.id, None).await.is_ok());
        assert!(check(&rig, &second.secret, bob.id, None).await.is_ok());

        // Sign out everywhere ends every key of the person, and each one is in the log.
        let before = rig.database.security_log_entries(0, 1000).await?.len();
        rig.api()
            .sign_out_everywhere(
                Rig::info(),
                SignOutEverywhere {
                    session_token: changed.session_token.clone(),
                },
            )
            .await?;
        assert_eq!(
            refused(check(&rig, &first.secret, bob.id, None).await),
            Some(ErrorCode::NotSignedIn)
        );
        assert_eq!(
            refused(check(&rig, &second.secret, bob.id, None).await),
            Some(ErrorCode::NotSignedIn)
        );
        let entries = rig.database.security_log_entries(0, 1000).await?;
        let deleted: Vec<_> = entries
            .iter()
            .skip(before)
            .filter(|entry| entry.action.as_str() == SecurityAction::AccessKeyDeleted.code())
            .collect();
        assert_eq!(deleted.len(), 2);
        assert!(deleted.iter().all(|entry| entry.person == Some(bob.id)));
        assert!(deleted.iter().all(|entry| entry.subject.is_some()));
        // The keys of other people are not touched.
        assert!(
            check(&rig, &other.secret, org.admin.id, None).await.is_ok(),
            "the key of the administrator still works"
        );
        Ok(())
    })
    .await
}
