//! The settings use cases against the in-memory store (CFG-06, CFG-04): what each one records, what
//! it refuses, and what the lists say about where a value comes from. Who may change what, on both
//! databases and through the policy engine, is tested in `bb-server`.

use bb_core::ErrorCode;
use bb_core::settings::SettingValue;

use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::settings::{
    ChangeSetting, LockSetting, ResetSetting, SettingList, SettingSource, SettingView,
    SettingsQuery, SettingsScope, change_setting, lock_setting, reset_setting, settings,
};

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.to_owned())
}

async fn change(
    world: &World,
    scope: SettingsScope,
    key: &str,
    value: SettingValue,
) -> Result<SettingView, AppError> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            session_token: world.admin.session_token.clone(),
            scope,
            key: key.to_owned(),
            value,
        },
    )
    .await
    .map(bb_policy::Authorized::into_value)
}

async fn list(world: &World, scope: SettingsScope, topic: &str) -> Result<SettingList, AppError> {
    settings(
        &world.ctx(),
        SettingsQuery {
            session_token: world.admin.session_token.clone(),
            scope,
            topic: Some(topic.to_owned()),
            project: None,
        },
    )
    .await
    .map(bb_policy::Authorized::into_value)
}

fn refused(result: Result<SettingView, AppError>) -> ErrorCode {
    match result {
        Err(AppError::Refused(code)) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn cfg_06_a_change_writes_one_event_and_one_entry_of_the_log_when_it_touches_security()
-> Outcome {
    let world = World::new().await?;
    let (events, log) = (world.store.event_count(), world.store.log_count());
    change(
        &world,
        SettingsScope::Organization,
        "security.session-idle-minutes",
        SettingValue::Integer(20),
    )
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(world.store.event_count(), events + 1);
    assert_eq!(world.store.log_count(), log + 1);
    // A choice of the look is nobody's business but the person's: an event, no entry.
    let (events, log) = (world.store.event_count(), world.store.log_count());
    change(
        &world,
        SettingsScope::Person,
        "appearance.mode",
        text("dark"),
    )
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(world.store.event_count(), events + 1);
    assert_eq!(world.store.log_count(), log);
    Ok(())
}

#[tokio::test]
async fn cfg_06_the_event_says_what_the_value_was_and_what_it_became() -> Outcome {
    let world = World::new().await?;
    change(
        &world,
        SettingsScope::Person,
        "appearance.mode",
        text("dark"),
    )
    .await
    .map_err(|error| error.to_string())?;
    change(
        &world,
        SettingsScope::Person,
        "appearance.mode",
        text("light"),
    )
    .await
    .map_err(|error| error.to_string())?;
    let events = world.store.lock().events.clone();
    let last = events.last().ok_or("no event")?;
    let change = last.changes.first().ok_or("no change")?;
    assert_eq!(change.field, "appearance_mode");
    assert_eq!(change.before, Some(serde_json::json!("dark")));
    assert_eq!(change.after, Some(serde_json::json!("light")));
    assert_eq!(last.entity.kind.as_str(), "setting");
    Ok(())
}

#[tokio::test]
async fn cfg_06_a_value_the_setting_does_not_accept_is_refused_and_nothing_is_written() -> Outcome {
    let world = World::new().await?;
    let events = world.store.event_count();
    let bad = [
        (
            SettingsScope::Organization,
            "security.session-idle-minutes",
            SettingValue::Integer(1),
        ),
        (
            SettingsScope::Organization,
            "security.session-idle-minutes",
            text("soon"),
        ),
        (SettingsScope::Person, "appearance.mode", text("sometimes")),
        (SettingsScope::Person, "appearance.less-motion", text("yes")),
        (
            SettingsScope::Person,
            "start.page",
            text("https://evil.test/"),
        ),
        (SettingsScope::Person, "region.time-zone", text("Nowhere")),
        (SettingsScope::Person, "shortcuts.keys", text("[]")),
    ];
    for (scope, key, value) in bad {
        let code = refused(change(&world, scope, key, value).await);
        assert_eq!(code, ErrorCode::InvalidRequest, "{key}");
    }
    assert_eq!(world.store.event_count(), events);
    assert!(world.store.read(|state| state.settings.is_empty()));
    Ok(())
}

#[tokio::test]
async fn cfg_06_a_setting_that_does_not_exist_or_not_on_that_level_is_refused() -> Outcome {
    let world = World::new().await?;
    let unknown = change(
        &world,
        SettingsScope::Person,
        "appearance.colour",
        text("x"),
    )
    .await;
    assert_eq!(refused(unknown), ErrorCode::NotFound);
    // The keys of shortcuts are a person's alone; the organization has no value for them.
    let organization = change(
        &world,
        SettingsScope::Organization,
        "shortcuts.keys",
        text("{}"),
    )
    .await;
    assert_eq!(refused(organization), ErrorCode::InvalidRequest);
    // The sign-in rules are the organization's: there is no person level to set them on.
    let own = change(
        &world,
        SettingsScope::Person,
        "security.session-idle-minutes",
        SettingValue::Integer(10),
    )
    .await;
    assert_eq!(refused(own), ErrorCode::InvalidRequest);
    // A project that does not exist answers like one the person may not know about.
    let project = bb_core::id::ProjectId::generate(&mut crate::context::SystemIds)?;
    let missing = change(
        &world,
        SettingsScope::Project(project),
        "appearance.mode",
        text("x"),
    )
    .await;
    assert_eq!(refused(missing), ErrorCode::NotFound);
    Ok(())
}

#[tokio::test]
async fn cfg_06_reset_of_a_scope_without_a_value_writes_nothing() -> Outcome {
    let world = World::new().await?;
    let events = world.store.event_count();
    let view = reset_setting(
        &world.ctx(),
        ResetSetting {
            session_token: world.admin.session_token.clone(),
            scope: SettingsScope::Person,
            key: "appearance.mode".to_owned(),
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    assert!(!view.own);
    assert_eq!(world.store.event_count(), events);
    Ok(())
}

#[tokio::test]
async fn cfg_06_the_list_says_where_a_value_comes_from_and_what_reset_takes_away() -> Outcome {
    let world = World::new().await?;
    let mode = |list: &SettingList| -> SettingView {
        list.settings
            .iter()
            .find(|view| view.definition.key() == "appearance.mode")
            .cloned()
            .expect("the list holds the setting")
    };
    let mine = list(&world, SettingsScope::Person, "appearance.").await?;
    assert_eq!(mine.settings.len(), 7, "appearance has seven settings");
    let view = mode(&mine);
    assert_eq!(
        (view.source, view.own, view.locked),
        (SettingSource::Default, false, false)
    );

    // The organization's default shows as the organization's choice, and as the person's value.
    change(
        &world,
        SettingsScope::Organization,
        "appearance.mode",
        text("light"),
    )
    .await?;
    let view = mode(&list(&world, SettingsScope::Person, "appearance.").await?);
    assert_eq!(view.value, text("light"));
    assert_eq!(view.source, SettingSource::Organization);
    // The person's own value counts, and is theirs to reset.
    change(
        &world,
        SettingsScope::Person,
        "appearance.mode",
        text("dark"),
    )
    .await?;
    let view = mode(&list(&world, SettingsScope::Person, "appearance.").await?);
    assert_eq!(
        (view.value.clone(), view.source, view.own),
        (text("dark"), SettingSource::Person, true)
    );
    let view = reset_setting(
        &world.ctx(),
        ResetSetting {
            session_token: world.admin.session_token.clone(),
            scope: SettingsScope::Person,
            key: "appearance.mode".to_owned(),
        },
    )
    .await?
    .into_value();
    assert_eq!(
        (view.value, view.source, view.own),
        (text("light"), SettingSource::Organization, false)
    );
    // The organization's list shows its own value, not the person's.
    let org = mode(&list(&world, SettingsScope::Organization, "appearance.").await?);
    assert_eq!((org.value, org.own), (text("light"), true));
    Ok(())
}

#[tokio::test]
async fn cfg_06_a_lock_is_refused_for_a_setting_that_cannot_be_locked() -> Outcome {
    let world = World::new().await?;
    let result = lock_setting(
        &world.ctx(),
        LockSetting {
            session_token: world.admin.session_token.clone(),
            key: "security.session-idle-minutes".to_owned(),
            locked: true,
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(AppError::Refused(ErrorCode::InvalidRequest))
    ));
    Ok(())
}

#[tokio::test]
async fn cfg_06_a_locked_value_refuses_what_is_set_below_it_and_names_why() -> Outcome {
    let world = World::new().await?;
    change(
        &world,
        SettingsScope::Organization,
        "appearance.mode",
        text("dark"),
    )
    .await?;
    lock_setting(
        &world.ctx(),
        LockSetting {
            session_token: world.admin.session_token.clone(),
            key: "appearance.mode".to_owned(),
            locked: true,
        },
    )
    .await?;
    let events = world.store.event_count();
    let code = refused(
        change(
            &world,
            SettingsScope::Person,
            "appearance.mode",
            text("light"),
        )
        .await,
    );
    assert_eq!(code, ErrorCode::SettingLocked);
    assert_eq!(world.store.event_count(), events);
    let view = list(&world, SettingsScope::Person, "appearance.mode")
        .await?
        .settings
        .remove(0);
    assert_eq!(
        (view.value, view.source, view.locked),
        (text("dark"), SettingSource::Organization, true)
    );
    Ok(())
}
