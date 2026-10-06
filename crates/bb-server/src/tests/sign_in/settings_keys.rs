//! Settings and access keys (CFG-06, AUTH-07, ADR-0025 decision 8): a key that may only look
//! reads the person's own settings and writes none, no key reaches the settings of the
//! organization, and a key limited to a project reaches the settings of that project only.

use bb_app::access::Permission;
use bb_app::auth::AppError;
use bb_app::settings::{SettingsQuery, SettingsScope};
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;
use bb_testing::each_database;

use super::access_keys::make;
use super::policy_rig::{Member, Org, Run};
use super::rig::{Outcome, Rig, refused};
use super::settings::{NOTE, change, lock, people, reset, text};

const LOOK: &str = "appearance.mode";

/// `owner` as a script that came with the key: the same person, the key as the credential.
async fn with_key(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    projects: &[ProjectId],
) -> Run<Member> {
    let created = make(rig, owner, choice, &[], projects).await?;
    Ok(Member {
        id: owner.id,
        session: created.secret,
    })
}

async fn list_mine(rig: &Rig, who: &Member) -> Result<usize, AppError> {
    let list = rig
        .api()
        .settings(
            Rig::info(),
            SettingsQuery {
                session_token: who.session.clone(),
                scope: SettingsScope::Person,
                topic: Some("appearance.".to_owned()),
                project: None,
            },
        )
        .await?
        .into_value();
    Ok(list.settings.len())
}

#[tokio::test]
async fn auth_07_a_key_that_can_only_look_reads_my_settings_and_changes_none() -> Outcome {
    each_database("auth_07_keys_my_settings", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let person = &team.viewer;
        change(&rig, person, SettingsScope::Person, LOOK, text("dark")).await?;
        let looking = with_key(&rig, person, KeyChoice::SeeOnly, &[]).await?;
        assert!(list_mine(&rig, &looking).await? > 0, "it may read");
        let by_key = change(&rig, &looking, SettingsScope::Person, LOOK, text("light")).await;
        assert_eq!(refused(by_key), Some(ErrorCode::Forbidden));
        let reset_by_key = reset(&rig, &looking, SettingsScope::Person, LOOK).await;
        assert_eq!(refused(reset_by_key), Some(ErrorCode::Forbidden));
        // Nothing changed: the person's own value is still the one they chose.
        let mine = super::settings::one(&rig, person, SettingsScope::Person, LOOK, None).await?;
        assert_eq!((mine.value, mine.own), (text("dark"), true));
        // A key that may change does change them, as the person.
        let changing = with_key(&rig, person, KeyChoice::SeeAndChange, &[]).await?;
        change(&rig, &changing, SettingsScope::Person, LOOK, text("light")).await?;
        let mine = super::settings::one(&rig, person, SettingsScope::Person, LOOK, None).await?;
        assert_eq!(mine.value, text("light"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_no_key_reaches_the_settings_of_the_organization() -> Outcome {
    each_database("auth_07_keys_organization_settings", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let ada = &org.admin;
        // The administrator can, with the session.
        change(&rig, ada, SettingsScope::Organization, LOOK, text("dark")).await?;
        for choice in [
            KeyChoice::SeeOnly,
            KeyChoice::SeeAndChange,
            KeyChoice::Everything,
        ] {
            let key = with_key(&rig, ada, choice, &[]).await?;
            let forbidden = Some(ErrorCode::Forbidden);
            let read = rig
                .api()
                .settings(
                    Rig::info(),
                    SettingsQuery {
                        session_token: key.session.clone(),
                        scope: SettingsScope::Organization,
                        topic: None,
                        project: None,
                    },
                )
                .await
                .map(|_| ());
            assert_eq!(refused(read), forbidden, "{choice:?} reads");
            let changed =
                change(&rig, &key, SettingsScope::Organization, LOOK, text("light")).await;
            assert_eq!(refused(changed), forbidden, "{choice:?} changes");
            let was_reset = reset(&rig, &key, SettingsScope::Organization, LOOK).await;
            assert_eq!(refused(was_reset), forbidden, "{choice:?} resets");
            let locked = lock(&rig, &key, LOOK, true).await;
            assert_eq!(refused(locked), forbidden, "{choice:?} locks");
            let unlocked = lock(&rig, &key, LOOK, false).await;
            assert_eq!(refused(unlocked), forbidden, "{choice:?} unlocks");
        }
        // And nothing of it happened.
        let view = super::settings::one(&rig, ada, SettingsScope::Organization, LOOK, None).await?;
        assert_eq!(
            (view.value, view.own, view.locked),
            (text("dark"), true, false)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_limited_to_a_project_cannot_change_the_settings_of_another() -> Outcome {
    each_database("auth_07_keys_project_settings", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let other = org.project(None, "OTHER").await?.id;
        // A manager of both ACME and OTHER, with a key that counts in ACME only.
        let manager_role = org
            .role(
                "Manager",
                &[Permission::SeeProjects, Permission::ChangeProjectSettings],
            )
            .await?;
        let manager = org.person().await?;
        org.give(&manager, manager_role.id, team.acme).await?;
        org.give(&manager, manager_role.id, other).await?;
        change(
            &rig,
            &manager,
            SettingsScope::Project(other),
            NOTE,
            text("by me"),
        )
        .await?;
        // Project settings are *Manage* permissions: *See and change* does not reach them.
        let lesser = with_key(&rig, &manager, KeyChoice::SeeAndChange, &[team.acme]).await?;
        let by_lesser = change(
            &rig,
            &lesser,
            SettingsScope::Project(team.acme),
            NOTE,
            text("x"),
        )
        .await;
        assert_eq!(refused(by_lesser), Some(ErrorCode::Forbidden));
        let key = with_key(&rig, &manager, KeyChoice::Everything, &[team.acme]).await?;
        // In ACME, and in WEB inside it, the key works like its owner.
        change(
            &rig,
            &key,
            SettingsScope::Project(team.acme),
            NOTE,
            text("acme"),
        )
        .await?;
        change(
            &rig,
            &key,
            SettingsScope::Project(team.web),
            NOTE,
            text("web"),
        )
        .await?;
        // In OTHER it does nothing, however much its owner may do there.
        let refusal = |result| {
            matches!(
                refused(result),
                Some(ErrorCode::Forbidden | ErrorCode::NotFound)
            )
        };
        assert!(refusal(
            change(&rig, &key, SettingsScope::Project(other), NOTE, text("x")).await
        ));
        assert!(refusal(
            reset(&rig, &key, SettingsScope::Project(other), NOTE).await
        ));
        let read = rig
            .api()
            .settings(
                Rig::info(),
                SettingsQuery {
                    session_token: key.session.clone(),
                    scope: SettingsScope::Project(other),
                    topic: None,
                    project: None,
                },
            )
            .await
            .map(|_| ());
        assert!(matches!(
            refused(read),
            Some(ErrorCode::Forbidden | ErrorCode::NotFound)
        ));
        // The owner's value in OTHER is the one they set.
        let there =
            super::settings::one(&rig, &manager, SettingsScope::Project(other), NOTE, None).await?;
        assert_eq!(there.value, text("by me"));
        Ok(())
    })
    .await
}
