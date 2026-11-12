//! The two switches of the notification *Your time factor was changed*.

use super::super::{Bool, ChangedBy, Level, Setting, SettingDefinition, settings};

settings! {
    /// Whether the person on the bar is told in the app when somebody else changes the time factor
    /// of their issue (ADR-0033, amendment M8).
    NOTIFICATIONS_TIME_FACTOR_CHANGED_IN_APP: Bool = {
        key: "notifications.time-factor-changed-in-app",
        name: crate::message::SETTING_NOTIFICATIONS_TIME_FACTOR_CHANGED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_TIME_FACTOR_CHANGED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person on the bar is told by e-mail when somebody else changes the time factor
    /// of their issue.
    NOTIFICATIONS_TIME_FACTOR_CHANGED_BY_MAIL: Bool = {
        key: "notifications.time-factor-changed-by-mail",
        name: crate::message::SETTING_NOTIFICATIONS_TIME_FACTOR_CHANGED_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_TIME_FACTOR_CHANGED_BY_MAIL_HELP,
        lockable: true,
    }
}
