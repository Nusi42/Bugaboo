//! Settings in layers, through the real wiring and on both databases (CFG-06, CFG-04, ADR-0018,
//! ADR-0025 decision 14): which value counts, who may change what, *Reset to default*, the lock
//! of an administrator, and that a person's own settings follow them to another browser.
//!
//! No real setting is set on a project yet, so the project level is tested with the settings that
//! exist only for tests (`test.*`, the feature `test-settings` of `bb-core`).

use bb_app::access::Permission;
use bb_app::auth::AppError;
use bb_app::settings::{
    ChangeSetting, LockSetting, ResetSetting, SettingList, SettingSource, SettingView,
    SettingsQuery, SettingsScope,
};
use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::settings::SettingValue;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Member, Org, Run};
use super::rig::{PASSWORD, Rig, refused};

pub(super) fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.to_owned())
}

pub(super) const NOTE: &str = "test.project-note";

pub(super) async fn change(
    rig: &Rig,
    who: &Member,
    scope: SettingsScope,
    key: &str,
    value: SettingValue,
) -> Result<SettingView, AppError> {
    Ok(rig
        .api()
        .change_setting(
            Rig::info(),
            ChangeSetting {
                session_token: who.session.clone(),
                scope,
                key: key.to_owned(),
                value,
            },
        )
        .await?
        .into_value())
}

pub(super) async fn reset(
    rig: &Rig,
    who: &Member,
    scope: SettingsScope,
    key: &str,
) -> Result<SettingView, AppError> {
    Ok(rig
        .api()
        .reset_setting(
            Rig::info(),
            ResetSetting {
                session_token: who.session.clone(),
                scope,
                key: key.to_owned(),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn list(
    rig: &Rig,
    who: &Member,
    scope: SettingsScope,
    topic: &str,
    project: Option<ProjectId>,
) -> Result<SettingList, AppError> {
    Ok(rig
        .api()
        .settings(
            Rig::info(),
            SettingsQuery {
                session_token: who.session.clone(),
                scope,
                topic: Some(topic.to_owned()),
                project: project.map(|project| project.to_string()),
            },
        )
        .await?
        .into_value())
}

/// The one setting of a list of one key.
pub(super) async fn one(
    rig: &Rig,
    who: &Member,
    scope: SettingsScope,
    key: &str,
    project: Option<ProjectId>,
) -> Result<SettingView, AppError> {
    let mut found = list(rig, who, scope, key, project).await?.settings;
    assert_eq!(found.len(), 1, "{key}");
    Ok(found.remove(0))
}

pub(super) async fn lock(
    rig: &Rig,
    who: &Member,
    key: &str,
    locked: bool,
) -> Result<SettingView, AppError> {
    Ok(rig
        .api()
        .lock_setting(
            Rig::info(),
            LockSetting {
                session_token: who.session.clone(),
                key: key.to_owned(),
                locked,
            },
        )
        .await?
        .into_value())
}

/// The administrator, a lead of ACME (and so of WEB inside it), and somebody who may only look.
pub(super) struct People {
    pub(super) lead: Member,
    pub(super) viewer: Member,
    pub(super) acme: ProjectId,
    pub(super) web: ProjectId,
}

pub(super) async fn people(org: &Org<'_>) -> Run<People> {
    let acme = org.project(None, "ACME").await?.id;
    let web = org.project(Some(acme), "WEB").await?.id;
    let leader = org
        .role(
            "Leader",
            &[Permission::SeeProjects, Permission::ChangeProjectSettings],
        )
        .await?;
    let looker = org.role("Looker", &[Permission::SeeProjects]).await?;
    let lead = org.person().await?;
    let viewer = org.person().await?;
    org.give(&lead, leader.id, acme).await?;
    org.give(&viewer, looker.id, acme).await?;
    Ok(People {
        lead,
        viewer,
        acme,
        web,
    })
}

#[tokio::test]
async fn cfg_06_a_project_value_is_inherited_by_sub_projects_and_a_person_value_wins_for_the_person()
-> TestResult {
    each_database("cfg_06_inherited_use_cases", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);

        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;
        // A lead of ACME sets it on ACME; WEB inside ACME takes it, and says it comes from ACME.
        change(
            &rig,
            lead,
            SettingsScope::Project(team.acme),
            NOTE,
            text("acme"),
        )
        .await?;
        let in_web = one(&rig, lead, SettingsScope::Project(team.web), NOTE, None).await?;
        assert_eq!(in_web.value, text("acme"));
        assert!(!in_web.own, "WEB has no value of its own");
        match &in_web.source {
            SettingSource::Project(label) => {
                let shown = format!("{label:?}");
                assert!(shown.contains("ACME"), "{shown}");
            }
            other => panic!("expected a project, got {other:?}"),
        }
        // A person working in WEB gets ACME's value; their own value wins for them, and only them.
        let seen = one(&rig, viewer, SettingsScope::Person, NOTE, Some(team.web)).await?;
        assert_eq!((seen.value, seen.own), (text("acme"), false));
        change(&rig, viewer, SettingsScope::Person, NOTE, text("mine")).await?;
        let mine = one(&rig, viewer, SettingsScope::Person, NOTE, Some(team.web)).await?;
        assert_eq!(
            (mine.value, mine.source, mine.own),
            (text("mine"), SettingSource::Person, true)
        );
        let theirs = one(&rig, lead, SettingsScope::Person, NOTE, Some(team.web)).await?;
        assert_eq!(theirs.value, text("acme"));
        // Outside any project the organization's value counts.
        let outside = one(&rig, lead, SettingsScope::Person, NOTE, None).await?;
        assert_eq!(
            (outside.value, outside.source),
            (text("org"), SettingSource::Organization)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_locked_organization_value_wins_and_lower_values_are_refused() -> TestResult {
    each_database("cfg_06_locked_use_cases", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);
        change(
            &rig,
            lead,
            SettingsScope::Project(team.acme),
            NOTE,
            text("acme"),
        )
        .await?;
        change(&rig, viewer, SettingsScope::Person, NOTE, text("mine")).await?;

        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;
        let locked = lock(&rig, ada, NOTE, true).await?;
        assert!(locked.locked);
        // Every lower layer is overridden, and says why.
        for (who, scope, project) in [
            (viewer, SettingsScope::Person, Some(team.web)),
            (lead, SettingsScope::Project(team.web), None),
        ] {
            let view = one(&rig, who, scope, NOTE, project).await?;
            assert_eq!(
                (view.value, view.source, view.locked),
                (text("org"), SettingSource::Organization, true)
            );
        }
        // And what they try to set is refused, as a sentence that names the lock.
        let by_lead = change(
            &rig,
            lead,
            SettingsScope::Project(team.acme),
            NOTE,
            text("x"),
        )
        .await;
        assert_eq!(refused(by_lead), Some(ErrorCode::SettingLocked));
        let by_person = change(&rig, viewer, SettingsScope::Person, NOTE, text("x")).await;
        assert_eq!(refused(by_person), Some(ErrorCode::SettingLocked));
        // The administrator can still change the locked value; the lock stays.
        let changed = change(&rig, ada, SettingsScope::Organization, NOTE, text("org2")).await?;
        assert_eq!((changed.value, changed.locked), (text("org2"), true));
        // A person who has nothing to lose may take their own value away.
        let back = reset(&rig, viewer, SettingsScope::Person, NOTE).await?;
        assert!(!back.own && back.locked);

        // Unlocked, the layers below count again: the project's value and the person's.
        let open = lock(&rig, ada, NOTE, false).await?;
        assert!(!open.locked);
        let view = one(&rig, lead, SettingsScope::Project(team.web), NOTE, None).await?;
        assert_eq!(view.value, text("acme"));
        change(&rig, viewer, SettingsScope::Person, NOTE, text("again")).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_resetting_a_locked_organization_value_keeps_the_lock() -> TestResult {
    each_database("cfg_06_reset_keeps_lock", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, viewer) = (&org.admin, &team.viewer);
        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;
        lock(&rig, ada, NOTE, true).await?;
        let back = reset(&rig, ada, SettingsScope::Organization, NOTE).await?;
        // The standard value counts, still locked; there is nothing of its own left to reset.
        assert_eq!(
            (back.value, back.locked, back.own),
            (text("none"), true, false)
        );
        let by_person = change(&rig, viewer, SettingsScope::Person, NOTE, text("x")).await;
        assert_eq!(refused(by_person), Some(ErrorCode::SettingLocked));
        // Unlocking is its own action, and leaves no value behind.
        let open = lock(&rig, ada, NOTE, false).await?;
        assert_eq!((open.locked, open.own), (false, false));
        change(&rig, viewer, SettingsScope::Person, NOTE, text("mine")).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_reset_to_default_removes_only_that_layer() -> TestResult {
    each_database("cfg_06_reset_use_cases", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);
        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;
        change(
            &rig,
            lead,
            SettingsScope::Project(team.acme),
            NOTE,
            text("acme"),
        )
        .await?;
        change(
            &rig,
            lead,
            SettingsScope::Project(team.web),
            NOTE,
            text("web"),
        )
        .await?;
        change(&rig, viewer, SettingsScope::Person, NOTE, text("mine")).await?;

        let counts = async |project| -> Result<(SettingValue, SettingSource), AppError> {
            let view = one(&rig, viewer, SettingsScope::Person, NOTE, project).await?;
            Ok((view.value, view.source))
        };
        let label = |view: &SettingView| format!("{:?}", view.source);
        // The person's reset shows the nearest project's value; the project values stay.
        let after_person = reset(&rig, viewer, SettingsScope::Person, NOTE).await?;
        assert!(!after_person.own);
        assert_eq!(counts(Some(team.web)).await?.0, text("web"));
        // WEB's reset shows ACME's value, and ACME keeps its own.
        let after_web = reset(&rig, lead, SettingsScope::Project(team.web), NOTE).await?;
        assert_eq!(after_web.value, text("acme"));
        assert!(label(&after_web).contains("ACME"), "{}", label(&after_web));
        let acme = one(&rig, lead, SettingsScope::Project(team.acme), NOTE, None).await?;
        assert_eq!((acme.value, acme.own), (text("acme"), true));
        // ACME's reset shows the organization's value; the organization keeps its own.
        let after_acme = reset(&rig, lead, SettingsScope::Project(team.acme), NOTE).await?;
        assert_eq!(
            (after_acme.value, after_acme.source),
            (text("org"), SettingSource::Organization)
        );
        let organization = one(&rig, ada, SettingsScope::Organization, NOTE, None).await?;
        assert_eq!((organization.value, organization.own), (text("org"), true));
        // The organization's reset is the standard value.
        let standard = reset(&rig, ada, SettingsScope::Organization, NOTE).await?;
        assert_eq!(
            (standard.value, standard.source),
            (text("none"), SettingSource::Default)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_only_administrators_change_organization_values_and_each_person_only_their_own()
-> TestResult {
    each_database("cfg_06_who_may_change", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);
        let outsider = org.person().await?;

        // The organization: only somebody who may change organization settings.
        for who in [lead, viewer, &outsider] {
            let tried = change(&rig, who, SettingsScope::Organization, NOTE, text("x")).await;
            assert_eq!(refused(tried), Some(ErrorCode::Forbidden));
            let listed = list(&rig, who, SettingsScope::Organization, "test.", None).await;
            assert_eq!(refused(listed), Some(ErrorCode::Forbidden));
            let locked = lock(&rig, who, NOTE, true).await;
            assert_eq!(refused(locked), Some(ErrorCode::Forbidden));
        }
        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_only_project_leads_change_the_values_of_their_project() -> TestResult {
    each_database("cfg_06_who_may_change_project", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);
        let outsider = org.person().await?;
        change(&rig, ada, SettingsScope::Organization, NOTE, text("org")).await?;

        // A project: a lead for it (and its sub-projects), not a person who may only look, and
        // not somebody who cannot see it, who is told it does not exist.
        change(
            &rig,
            lead,
            SettingsScope::Project(team.web),
            NOTE,
            text("web"),
        )
        .await?;
        let by_viewer = change(
            &rig,
            viewer,
            SettingsScope::Project(team.acme),
            NOTE,
            text("x"),
        )
        .await;
        assert_eq!(refused(by_viewer), Some(ErrorCode::Forbidden));
        let by_outsider = change(
            &rig,
            &outsider,
            SettingsScope::Project(team.acme),
            NOTE,
            text("x"),
        )
        .await;
        assert_eq!(refused(by_outsider), Some(ErrorCode::NotFound));
        // A setting only administrators change is out of reach of a lead, on their own project too.
        let tried = change(
            &rig,
            lead,
            SettingsScope::Project(team.acme),
            "test.admin-choice",
            text("short"),
        )
        .await;
        assert_eq!(refused(tried), Some(ErrorCode::Forbidden));
        change(
            &rig,
            ada,
            SettingsScope::Project(team.acme),
            "test.admin-choice",
            text("short"),
        )
        .await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_each_person_changes_only_their_own_values() -> TestResult {
    each_database("cfg_06_who_may_change_person", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let team = people(&org).await?;
        let (ada, lead, viewer) = (&org.admin, &team.lead, &team.viewer);
        let outsider = org.person().await?;

        // A person: only their own value, for a setting each person changes. Everybody who is
        // signed in may; nothing names another person, so nobody can reach another's value.
        for who in [ada, lead, viewer, &outsider] {
            change(
                &rig,
                who,
                SettingsScope::Person,
                "appearance.mode",
                text("dark"),
            )
            .await?;
        }
        let tried = change(
            &rig,
            viewer,
            SettingsScope::Person,
            "test.admin-person",
            text("short"),
        )
        .await;
        assert_eq!(
            refused(tried),
            Some(ErrorCode::Forbidden),
            "an administrators' setting"
        );
        change(
            &rig,
            ada,
            SettingsScope::Organization,
            "test.admin-person",
            text("short"),
        )
        .await?;
        let tried = change(
            &rig,
            viewer,
            SettingsScope::Person,
            "security.session-idle-minutes",
            SettingValue::Integer(10),
        )
        .await;
        assert_eq!(
            refused(tried),
            Some(ErrorCode::InvalidRequest),
            "no person level"
        );
        let tried = change(
            &rig,
            viewer,
            SettingsScope::Person,
            "test.project-limit",
            SettingValue::Integer(5),
        )
        .await;
        assert_eq!(
            refused(tried),
            Some(ErrorCode::InvalidRequest),
            "no person level"
        );
        // A person's value is theirs: the others still see the organization's.
        let ada_mode = one(
            &rig,
            ada,
            SettingsScope::Organization,
            "appearance.mode",
            None,
        )
        .await?;
        assert!(!ada_mode.own, "the organization has set nothing");
        // A project they cannot see is no place to work in.
        let hidden = list(
            &rig,
            &outsider,
            SettingsScope::Person,
            "test.",
            Some(team.acme),
        )
        .await;
        assert_eq!(refused(hidden), Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_04_my_settings_are_kept_for_the_person_and_follow_them_to_another_browser()
-> TestResult {
    each_database("cfg_04_follows_the_person", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let bea = org.person().await?;
        // The same person signed in from a second browser: another session, the same settings.
        let second = rig.sign_in("person1@example.test", PASSWORD).await?;
        let elsewhere = Member {
            id: bea.id,
            session: second.session_token,
        };
        assert_ne!(bea.session, elsewhere.session);

        for (key, value) in [
            ("appearance.mode", text("dark")),
            ("appearance.dark-theme", text("midnight")),
            ("appearance.less-motion", SettingValue::Bool(true)),
            ("appearance.larger-text", SettingValue::Bool(true)),
            ("region.time-zone", text("Pacific/Auckland")),
            ("start.page", text("/me/appearance")),
            ("shortcuts.keys", text(r#"{"go-home":["g i"]}"#)),
        ] {
            change(&rig, &bea, SettingsScope::Person, key, value).await?;
        }
        for (key, value) in [
            ("appearance.mode", text("dark")),
            ("appearance.dark-theme", text("midnight")),
            ("appearance.less-motion", SettingValue::Bool(true)),
            ("appearance.larger-text", SettingValue::Bool(true)),
            ("region.time-zone", text("Pacific/Auckland")),
            ("start.page", text("/me/appearance")),
            ("shortcuts.keys", text(r#"{"go-home":["g i"]}"#)),
        ] {
            let view = one(&rig, &elsewhere, SettingsScope::Person, key, None).await?;
            assert_eq!((view.value, view.own), (value, true), "{key}");
        }
        // After signing out of both and in again, nothing is lost: it is the person's, not the
        // browser's.
        for member in [&bea, &elsewhere] {
            rig.api()
                .sign_out(
                    Rig::info(),
                    bb_app::auth::SignOut {
                        session_token: member.session.clone(),
                    },
                )
                .await?;
        }
        let again = rig.sign_in("person1@example.test", PASSWORD).await?;
        let back = Member {
            id: bea.id,
            session: again.session_token,
        };
        let mode = one(&rig, &back, SettingsScope::Person, "appearance.mode", None).await?;
        assert_eq!(mode.value, text("dark"));
        // Somebody else on that computer sees none of it.
        let other = org.person().await?;
        let theirs = one(&rig, &other, SettingsScope::Person, "appearance.mode", None).await?;
        assert_eq!((theirs.value, theirs.own), (text("device"), false));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_change_of_a_setting_that_touches_security_is_in_the_security_log() -> TestResult {
    each_database("cfg_06_security_log", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let entries = async |action: SecurityAction| -> Result<usize, bb_store::StoreError> {
            Ok(rig
                .database
                .security_log_entries(0, 10_000)
                .await?
                .iter()
                .filter(|entry| entry.action.as_str() == action.code())
                .count())
        };
        let ada = &org.admin;
        change(
            &rig,
            ada,
            SettingsScope::Organization,
            "security.session-idle-minutes",
            SettingValue::Integer(20),
        )
        .await?;
        assert_eq!(entries(SecurityAction::SettingChanged).await?, 1);
        reset(
            &rig,
            ada,
            SettingsScope::Organization,
            "security.session-idle-minutes",
        )
        .await?;
        assert_eq!(entries(SecurityAction::SettingReset).await?, 1);
        // A choice of the look is not a matter of safety.
        change(
            &rig,
            ada,
            SettingsScope::Person,
            "appearance.mode",
            text("dark"),
        )
        .await?;
        lock(&rig, ada, "appearance.mode", true).await?;
        assert_eq!(entries(SecurityAction::SettingChanged).await?, 1);
        assert_eq!(entries(SecurityAction::SettingLocked).await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_the_changes_of_a_setting_are_events_about_that_setting() -> TestResult {
    each_database("cfg_06_events", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let ada = &org.admin;
        change(
            &rig,
            ada,
            SettingsScope::Organization,
            "security.session-idle-minutes",
            SettingValue::Integer(20),
        )
        .await?;
        change(
            &rig,
            ada,
            SettingsScope::Organization,
            "security.session-idle-minutes",
            SettingValue::Integer(25),
        )
        .await?;
        let entity = bb_core::event::EntityRef {
            kind: bb_core::event::EntityKind::new("setting")?,
            id: bb_core::id::EntityId::from_bytes(bb_auth::stable_id(
                "setting",
                "security.session-idle-minutes",
            ))?,
        };
        let events = rig.database.events_about(&entity, 10).await?;
        assert_eq!(events.len(), 2);
        // Two events in one millisecond have no order of their own: find the later change by what
        // it set.
        let step = events
            .iter()
            .filter_map(|event| event.changes.first())
            .find(|step| step.after == Some(serde_json::json!(25)))
            .ok_or("no event set 25")?;
        assert_eq!(step.before, Some(serde_json::json!(20)));
        // The value counts for the sign-in rules at once.
        let settings = bb_app::auth::AuthStore::auth_settings(&rig.storage).await?;
        assert_eq!(settings.session_idle_minutes, 25);
        Ok(())
    })
    .await
}
