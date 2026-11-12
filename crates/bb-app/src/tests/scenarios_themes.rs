//! The scenarios of the use cases of own themes (THM-04), and what their tests share: a theme made
//! by a person, its edit lock, and the values a save sends.

use bb_core::edit::THEME_PART;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{LockToken, ThemeId, UserId};
use bb_core::theme::ThemeKind;
use serde_json::{Value, json};

use super::scenarios::{Added, measured, text, token};
use super::scenarios_layouts::lock_part_for;
use super::world::World;
use crate::themes::{
    CopyTheme, CreateTheme, DeleteTheme, SaveTheme, ThemeDetail, ThemeView, UploadTheme,
    copy_theme, create_theme, delete_theme, save_theme, upload_theme,
};

/// Values that change one color, which stays easy to read on the dark theme it is saved on.
pub(crate) fn an_accent() -> Value {
    json!({"colors": {"accent": {"value": "#b4a8ff"}}})
}

/// Values whose text is as light as the background of *Daylight*: hard to read.
pub(crate) fn white_text() -> Value {
    json!({"colors": {"fg": {"value": "#ffffff"}}})
}

/// A new theme of `kind` that extends `extends`, made by the person of `session`.
pub(crate) async fn a_theme(
    world: &World,
    session: &str,
    kind: ThemeKind,
    extends: &str,
) -> Result<ThemeView, String> {
    Ok(create_theme(
        &world.ctx(),
        CreateTheme {
            session_token: session.to_owned(),
            name: "Evening".to_owned(),
            kind,
            extends: extends.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value())
}

/// The edit lock on a theme, held by `holder`.
pub(crate) fn lock_the_theme_for(world: &World, theme: ThemeId, holder: UserId) -> LockToken {
    lock_part_for(
        world,
        EntityRef {
            kind: RecordKind::Theme.into(),
            id: theme.into(),
        },
        THEME_PART,
        holder,
    )
}

/// Saves `values` on a theme as the person of `session`, who holds its lock.
pub(crate) async fn save_values(
    world: &World,
    session: &str,
    holder: UserId,
    view: &ThemeView,
    values: Value,
) -> Result<ThemeDetail, crate::auth::AppError> {
    let lock_token = lock_the_theme_for(world, view.theme.id, holder);
    save_theme(
        &world.ctx(),
        SaveTheme {
            session_token: session.to_owned(),
            theme: view.theme.id,
            name: view.theme.name.as_str().to_owned(),
            values,
            version: view.theme.version,
            lock_token,
        },
    )
    .await
    .map(bb_policy::Authorized::into_value)
}

fn admin_id(world: &World) -> UserId {
    world.admin.person.id
}

pub(crate) async fn create_theme_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_theme(
            &world.ctx(),
            CreateTheme {
                session_token: token(world),
                name: "Company look".to_owned(),
                kind: ThemeKind::ForEveryone,
                extends: "dusk".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn save_theme_scenario(world: &World) -> Added {
    let view = a_theme(world, &token(world), ThemeKind::ForEveryone, "dusk").await?;
    measured(
        &world.store,
        save_values(world, &token(world), admin_id(world), &view, an_accent()),
    )
    .await
}

pub(crate) async fn copy_theme_scenario(world: &World) -> Added {
    let view = a_theme(world, &token(world), ThemeKind::ForEveryone, "dusk").await?;
    save_values(world, &token(world), admin_id(world), &view, an_accent())
        .await
        .map_err(text)?;
    measured(
        &world.store,
        copy_theme(
            &world.ctx(),
            CopyTheme {
                session_token: token(world),
                theme: view.theme.id,
                name: "Mine".to_owned(),
                kind: ThemeKind::Personal,
            },
        ),
    )
    .await
}

pub(crate) async fn upload_theme_scenario(world: &World) -> Added {
    let file = json!({"name": "Uploaded", "extends": "dusk", "values": an_accent()});
    measured(
        &world.store,
        upload_theme(
            &world.ctx(),
            UploadTheme {
                session_token: token(world),
                kind: ThemeKind::Personal,
                file: file.to_string().into_bytes(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_theme_scenario(world: &World) -> Added {
    let view = a_theme(world, &token(world), ThemeKind::Personal, "dusk").await?;
    measured(
        &world.store,
        delete_theme(
            &world.ctx(),
            DeleteTheme {
                session_token: token(world),
                theme: view.theme.id,
                version: view.theme.version,
            },
        ),
    )
    .await
}
