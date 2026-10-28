//! The scenarios of levels, settings and projects (M1-F18).

use super::*;

pub(crate) async fn set_field_level_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: token(world),
                role,
                project: None,
                issue_type: None,
                field: Field::Title.into(),
                level: Some(bb_core::access::FieldLevel::CanSee),
            },
        ),
    )
    .await
}

pub(crate) async fn set_people_level_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        set_level(
            &world.ctx(),
            SetPeopleLevel {
                session_token: token(world),
                role,
                project: None,
                kind: PeopleKind::Followers,
                level: Some(bb_core::access::PeopleLevel::Names),
            },
        ),
    )
    .await
}

pub(crate) async fn create_project_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_project(
            &world.ctx(),
            CreateProject {
                session_token: token(world),
                parent: None,
                short_name: "ACME".to_owned(),
                name: "Acme".to_owned(),
            },
        ),
    )
    .await
}

/// A value of the person's own, for the scenarios that take one away.
async fn a_person_value(world: &World) -> Result<(), String> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope: SettingsScope::Person,
            key: "appearance.mode".to_owned(),
            value: SettingValue::Text("dark".to_owned()),
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

pub(crate) async fn change_setting_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_setting(
            &world.ctx(),
            ChangeSetting {
                version: None,
                session_token: token(world),
                scope: SettingsScope::Organization,
                key: "security.session-idle-minutes".to_owned(),
                value: SettingValue::Integer(20),
            },
        ),
    )
    .await
}

pub(crate) async fn reset_setting_scenario(world: &World) -> Added {
    a_person_value(world).await?;
    measured(
        &world.store,
        reset_setting(
            &world.ctx(),
            ResetSetting {
                session_token: token(world),
                scope: SettingsScope::Person,
                key: "appearance.mode".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn lock_setting_scenario(world: &World) -> Added {
    measured(
        &world.store,
        lock_setting(
            &world.ctx(),
            LockSetting {
                session_token: token(world),
                key: "appearance.mode".to_owned(),
                locked: true,
            },
        ),
    )
    .await
}

pub(crate) async fn change_project_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    measured(
        &world.store,
        change_project(
            &world.ctx(),
            ChangeProject {
                session_token: token(world),
                project,
                version: 1,
                name: Some("Acme Corporation".to_owned()),
                short_name: Some("ACMECORP".to_owned()),
                fields: std::collections::BTreeMap::new(),
            },
        ),
    )
    .await
}

pub(crate) async fn move_project_scenario(world: &World) -> Added {
    let acme = a_project(world, "ACME").await?;
    let web = a_project(world, "WEB").await?;
    let preview = preview_move(
        &world.ctx(),
        MovePreviewQuery {
            session_token: token(world),
            project: web,
            to: Some(acme),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .preview;
    measured(
        &world.store,
        move_project(
            &world.ctx(),
            MoveProject {
                session_token: token(world),
                project: web,
                to: Some(acme),
                preview,
            },
        ),
    )
    .await
}
