//! The keep periods of *Keep and delete rules* (PRIV-05, ADR-0045): how long each kind of data that
//! has a period of its own is kept. Each is a setting of the registry ([`super::registry`] lists
//! them), its lower bound is the floor of its row, and the rows of `crate::keeping` name them. The
//! periods that were settings before (the security log, work days) stay where they are and are named
//! by their rows, not copied here. Every change is also written to the security log
//! ([`super::SettingDefinition::touches_security`]): a shorter period deletes data.

use super::{ChangedBy, Integer, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// How many days a shortened network address is kept on a session (ADR-0045, decision 1). At
    /// least one day: the address of a sign-in from today is what stops an attack on it.
    KEEPING_IP_ADDRESS_DAYS: Integer = {
        key: "keeping.ip-address-days",
        name: message::SETTING_KEEPING_IP_ADDRESS_DAYS_NAME,
        default: 7,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_KEEPING_IP_ADDRESS_DAYS_HELP,
        lockable: false,
        bounds: (1, 90),
    }

    /// How many days a notification is kept (ADR-0033 §5, ADR-0045). At least a week, so a person
    /// back from a few days away still finds what happened.
    KEEPING_NOTIFICATION_DAYS: Integer = {
        key: "keeping.notification-days",
        name: message::SETTING_KEEPING_NOTIFICATION_DAYS_NAME,
        default: 90,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_KEEPING_NOTIFICATION_DAYS_HELP,
        lockable: false,
        bounds: (7, 3650),
    }

    /// How many days what was deleted stays in the trash before it is deleted for good (M3-F25,
    /// M4-F131, ADR-0045). At least one day, so a deletion can still be undone.
    KEEPING_TRASH_DAYS: Integer = {
        key: "keeping.trash-days",
        name: message::SETTING_KEEPING_TRASH_DAYS_NAME,
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_KEEPING_TRASH_DAYS_HELP,
        lockable: false,
        bounds: (1, 3650),
    }

    /// How many days a job that gave up stays on the list of failed jobs (M1-F21, ADR-0045). Its
    /// payload can hold the address and the text of a person. At least one day, so whoever looks
    /// after the server sees why it failed.
    KEEPING_DEAD_JOB_DAYS: Integer = {
        key: "keeping.dead-job-days",
        name: message::SETTING_KEEPING_DEAD_JOB_DAYS_NAME,
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_KEEPING_DEAD_JOB_DAYS_HELP,
        lockable: false,
        bounds: (1, 3650),
    }

    /// How many whole years from its day a time entry is kept at least before an administrator may
    /// delete it (M9-Q2, ADR-0045). No law sets a floor for every entry; receipts are kept 8 years
    /// and the books 10, so the start is 10.
    KEEPING_TIME_ENTRY_YEARS: Integer = {
        key: "keeping.time-entry-years",
        name: message::SETTING_KEEPING_TIME_ENTRY_YEARS_NAME,
        default: 10,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_KEEPING_TIME_ENTRY_YEARS_HELP,
        lockable: false,
        bounds: (1, 100),
        unit: Years,
    }
}
