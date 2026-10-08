//! Access keys and the second step that is asked for (AUTH-03, AUTH-07, M1-F45, M2-F46,
//! ADR-0025 decisions 7 and 8): a key follows its owner's two-step requirement, so a person who
//! has to set up a second step cannot get around it with a key they made earlier.

use bb_app::access::Permission;
use bb_app::auth::AppError;
use bb_app::roles::ChangeRole;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::RoleId;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::access_keys::{made, probe};
use super::policy_rig::{Org, Run};
use super::rig::{Rig, Tries, refused};
use super::second_step_by_role::{role_with, set_up_a_code_app, switch};

const REQUIRED: Option<ErrorCode> = Some(ErrorCode::SecondStepRequired);

/// A role that lets people check access (a key may hold it), with the switch as given.
async fn checker(org: &Org<'_>, two_step: bool) -> Run<RoleId> {
    role_with(org, Permission::CheckAccess, "Checker", two_step).await
}

/// The role *Inviter*'s switch, changed with whatever the caller holds (a session or a key).
async fn switch_with(rig: &Rig, credential: &str, role: RoleId) -> Result<(), AppError> {
    rig.api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: credential.to_owned(),
                role,
                name: None,
                permissions: None,
                two_step: Some(false),
            },
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn auth_03_a_key_made_before_the_switch_is_refused_until_its_owner_sets_up_a_second_step()
-> TestResult {
    each_database("auth_03_key_follows_two_step", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let role = checker(&org, false).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, role).await?;
        let key = made(&rig, &bob, KeyChoice::Everything, None).await?;
        let narrow = made(&rig, &bob, KeyChoice::SeeOnly, None).await?;
        assert_eq!(refused(probe(&rig, &key.secret).await), None);

        // The role asks for a second step. Bob's session and his keys are gated alike.
        switch(&org, role, true).await?;
        assert_eq!(refused(probe(&rig, &bob.session).await), REQUIRED);
        assert_eq!(refused(probe(&rig, &key.secret).await), REQUIRED);
        assert_eq!(refused(probe(&rig, &narrow.secret).await), REQUIRED);
        // The refusal is written down with the owner and the key, like any key turned away.
        let entries = rig.database.security_log_entries(0, 1000).await?;
        let refusals: Vec<_> = entries
            .iter()
            .filter(|entry| entry.action.as_str() == SecurityAction::AccessKeyRefused.code())
            .collect();
        assert_eq!(refusals.len(), 2);
        assert!(refusals.iter().all(|entry| entry.person == Some(bob.id)));
        assert!(refusals.iter().all(|entry| entry.subject.is_some()));

        // The code app is set up, and the same keys work again: nothing was lost.
        set_up_a_code_app(&rig, bob.id, &bob.session).await?;
        assert_eq!(refused(probe(&rig, &key.secret).await), None);
        assert_eq!(refused(probe(&rig, &narrow.secret).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_key_cannot_switch_off_the_requirement_of_its_owner() -> TestResult {
    each_database("auth_03_key_cannot_free_its_owner", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let role = checker(&org, false).await?;
        org.give_everywhere(&org.admin, role).await?;
        // The administrator holds the role, and her key is made while its switch is off.
        let key = made(&rig, &org.admin, KeyChoice::Everything, None).await?;
        switch(&org, role, true).await?;
        assert_eq!(refused(probe(&rig, &org.admin.session).await), REQUIRED);
        // The key may not do what the session may not: it cannot free its owner.
        assert_eq!(
            refused(switch_with(&rig, &key.secret, role).await),
            REQUIRED
        );
        assert_eq!(refused(probe(&rig, &key.secret).await), REQUIRED);
        Ok(())
    })
    .await
}
