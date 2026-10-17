//! The scenarios of the notification and mail use cases (ADR-0007, M1-F18).

use bb_core::notification::NotificationFilter;
use bb_core::settings::SettingValue;

use super::scenarios::{Added, measured, text, token};
use super::world::World;
use crate::notifications::{
    NotificationsQuery, ReadNotifications, SendTestEmail, SetMailPassword, Which, notifications,
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
                which: Which::All,
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
