//! The settings of backups (OPS-07, OPS-06, ADR-0046 decisions 3, 5 and 6): whether the schedule
//! creates one every day at the quiet hour, how many daily and weekly backups it keeps, how many of
//! the backups made before an upgrade, and the folder they go into. They are part of the registry
//! like every other setting ([`super::registry`] lists them); administrators change them on
//! *Organization settings › System › Backups*.

use super::{Bool, ChangedBy, Integer, Level, Setting, SettingDefinition, Text, settings};
use crate::message;

settings! {
    /// Whether the schedule creates a backup every day at the quiet hour of the organization. On
    /// at the start.
    BACKUPS_DAILY: Bool = {
        key: "backups.daily",
        name: message::SETTING_BACKUPS_DAILY_NAME,
        default: true,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_BACKUPS_DAILY_HELP,
        lockable: false,
    }

    /// How many days keep their newest backup.
    BACKUPS_KEEP_DAILY: Integer = {
        key: "backups.keep-daily",
        name: message::SETTING_BACKUPS_KEEP_DAILY_NAME,
        default: 7,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_BACKUPS_KEEP_DAILY_HELP,
        lockable: false,
        bounds: (1, 366),
    }

    /// How many weeks keep their newest backup.
    BACKUPS_KEEP_WEEKLY: Integer = {
        key: "backups.keep-weekly",
        name: message::SETTING_BACKUPS_KEEP_WEEKLY_NAME,
        default: 4,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_BACKUPS_KEEP_WEEKLY_HELP,
        lockable: false,
        bounds: (0, 520),
    }

    /// How many of the backups made before an upgrade are kept, the newest; older ones go with the
    /// rotation of the schedule.
    BACKUPS_KEEP_BEFORE_UPGRADE: Integer = {
        key: "backups.keep-before-upgrade",
        name: message::SETTING_BACKUPS_KEEP_BEFORE_UPGRADE_NAME,
        default: 3,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_BACKUPS_KEEP_BEFORE_UPGRADE_HELP,
        lockable: false,
        bounds: (1, 100),
    }

    /// The folder backups go into, on the computer of the server; empty for the backups folder of
    /// the data folder. It must be a folder that only the account of the server can open.
    BACKUPS_FOLDER: Text = {
        key: "backups.folder",
        name: message::SETTING_BACKUPS_FOLDER_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_BACKUPS_FOLDER_HELP,
        lockable: false,
    }
}
