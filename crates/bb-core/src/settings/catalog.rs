//! The settings of Bugaboo. Add a setting here and nowhere else (ADR-0018).

use super::{Bool, ChangedBy, Integer, Level, Setting, SettingDefinition, Text, settings};
use crate::message;

settings! {
    /// The theme: *Daylight*, *Paper*, *Dusk*, *Midnight*, or `device` for *Match my device*.
    APPEARANCE_THEME: Text = {
        key: "appearance.theme",
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_THEME_HELP,
        lockable: true,
    }

    /// How many minutes without activity pass before people are signed out.
    SECURITY_SESSION_IDLE_MINUTES: Integer = {
        key: "security.session-idle-minutes",
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_SESSION_IDLE_MINUTES_HELP,
        lockable: false,
    }

    /// Whether everyone has to use two-step sign-in.
    SECURITY_TWO_STEP_REQUIRED: Bool = {
        key: "security.two-step-required",
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_TWO_STEP_REQUIRED_HELP,
        lockable: false,
    }
}
