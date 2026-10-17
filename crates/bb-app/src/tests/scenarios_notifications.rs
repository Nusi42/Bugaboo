//! The scenarios of the notification and mail use cases (ADR-0007, M1-F18).

use bb_core::confirmation::{AddressConfirmation, lasts};
use bb_core::notification::NotificationFilter;
use bb_core::settings::SettingValue;

use super::scenarios::{Added, measured, text, token};
use super::world::World;
use crate::notifications::{
    ChangeHandle, ConfirmAddress, CreateMissingHandles, NotificationsQuery, ReadNotifications,
    SendTestEmail, SetMailPassword, Which, change_handle, confirm_address, create_missing_handles,
    notifications, read_notifications, send_test_email, set_mail_password,
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

pub(crate) async fn confirm_address_scenario(world: &World) -> Added {
    // An account that waits for its address, and the link that confirms it.
    let secret = bb_auth::SecretToken::generate().map_err(text)?;
    let person = world
        .person("waiting@example.test")
        .await
        .map_err(text)?
        .person
        .id;
    let waiting = AddressConfirmation {
        person,
        token_hash: secret.hash(bb_auth::Purpose::AddressConfirmation),
        created_at: world.ctx().now(),
        expires_at: world.ctx().now().checked_add(lasts()).map_err(text)?,
    };
    world.store.lock().state.confirmations.push(waiting);
    measured(
        &world.store,
        confirm_address(
            &world.ctx(),
            ConfirmAddress {
                secret: secret.encode(),
            },
        ),
    )
    .await
}
