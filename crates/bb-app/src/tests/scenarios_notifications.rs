//! The scenarios of the notification and mail use cases (ADR-0007, M1-F18).

use bb_core::notification::NotificationFilter;
use bb_core::settings::SettingValue;

use super::scenarios::{Added, measured, text, token};
use super::world::World;
use crate::notifications::{
    ChangeHandle, CreateMissingHandles, NotificationsQuery, ReadNotifications, SendTestEmail,
    SetMailPassword, Which, change_handle, create_missing_handles, notifications,
    read_notifications, send_test_email, set_mail_password,
};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

pub(crate) async fn read_notifications_scenario(world: &World) -> Added {
    // The page of notifications is a read; marking them is the command.
    notifications(
        &world.ctx(),
        NotificationsQuery {
            session_token: token(world),
            filter: NotificationFilter::All,
            cursor: None,
            limit: 10,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        read_notifications(
            &world.ctx(),
            ReadNotifications {
                session_token: token(world),
                which: Which::UpTo(the_newest_there_can_be()),
            },
        ),
    )
    .await
}

pub(crate) async fn set_mail_password_scenario(world: &World) -> Added {
    measured(
        &world.store,
        set_mail_password(
            &world.ctx(),
            SetMailPassword {
                session_token: token(world),
                password: Some("a made-up mail password".to_owned()),
            },
        ),
    )
    .await
}

/// The mail server entered in the settings, as the tests of e-mail need it.
pub(crate) async fn enter_the_mail_server(world: &World) -> Result<(), String> {
    for (key, value) in [
        (
            "mail.host",
            SettingValue::Text("mail.example.test".to_owned()),
        ),
        (
            "mail.from",
            SettingValue::Text("bugaboo@example.test".to_owned()),
        ),
    ] {
        change_setting(
            &world.ctx(),
            ChangeSetting {
                session_token: token(world),
                scope: SettingsScope::Organization,
                key: key.to_owned(),
                value,
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(())
}

/// Enters the mail server and switches e-mail on.
pub(crate) async fn switch_mail_on(world: &World) -> Result<(), String> {
    enter_the_mail_server(world).await?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            session_token: token(world),
            scope: SettingsScope::Organization,
            key: "mail.enabled".to_owned(),
            value: SettingValue::Bool(true),
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

pub(crate) async fn send_test_email_scenario(world: &World) -> Added {
    enter_the_mail_server(world).await?;
    measured(
        &world.store,
        send_test_email(
            &world.ctx(),
            SendTestEmail {
                session_token: token(world),
            },
        ),
    )
    .await
}

pub(crate) async fn change_handle_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_handle(
            &world.ctx(),
            ChangeHandle {
                session_token: token(world),
                handle: "ada.a".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn create_missing_handles_scenario(world: &World) -> Added {
    // A person made before handles existed has none.
    world.store.lock().state.handles.clear();
    measured(
        &world.store,
        create_missing_handles(&world.ctx(), CreateMissingHandles),
    )
    .await
}

/// An ID that sorts after every notification there is: *up to* it marks all of them.
pub(crate) fn the_newest_there_can_be() -> bb_core::id::NotificationId {
    bb_core::id::NotificationId::from_parts(
        jiff::Timestamp::from_second(253_402_207_200).expect("a moment in the year 9999"),
        [0xff; 10],
    )
    .expect("a moment after 1970")
}
