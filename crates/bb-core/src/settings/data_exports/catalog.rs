//! The settings of the notifications of *Download my data*: a switch for each way of *Your data is
//! ready*, both on at the start, because the person asked for the file and waits for it.

use super::super::{Bool, ChangedBy, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// Whether a person hears in the app that the file of their data is ready (ADR-0044, decision 2).
    NOTIFICATIONS_EXPORT_READY_IN_APP: Bool = {
        key: "notifications.export-ready-in-app",
        name: message::SETTING_NOTIFICATIONS_EXPORT_READY_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_EXPORT_READY_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail that the file of their data is ready (ADR-0044, decision 2).
    NOTIFICATIONS_EXPORT_READY_BY_MAIL: Bool = {
        key: "notifications.export-ready-by-mail",
        name: message::SETTING_NOTIFICATIONS_EXPORT_READY_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_EXPORT_READY_BY_MAIL_HELP,
        lockable: true,
    }
}
