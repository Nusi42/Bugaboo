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
        bounds: (5, 1440),
    }

    /// How many hours a sign-in can last at most, however busy the person is.
    SECURITY_SESSION_ABSOLUTE_HOURS: Integer = {
        key: "security.session-absolute-hours",
        default: 24,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_SESSION_ABSOLUTE_HOURS_HELP,
        lockable: false,
        bounds: (1, 720),
    }

    /// Whether people may make an account without an invitation (AUTH-06).
    AUTH_SIGN_UP_ENABLED: Bool = {
        key: "auth.sign-up-enabled",
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_SIGN_UP_ENABLED_HELP,
        lockable: false,
    }

    /// The e-mail domains that may sign up on their own (a list separated by commas).
    AUTH_SIGN_UP_DOMAINS: Text = {
        key: "auth.sign-up-domains",
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_SIGN_UP_DOMAINS_HELP,
        lockable: false,
        list: (100, 253),
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

    /// How many days a new access key lives when the person picks no other date.
    AUTH_ACCESS_KEY_DEFAULT_DAYS: Integer = {
        key: "auth.access-key-default-days",
        default: 90,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_ACCESS_KEY_DEFAULT_DAYS_HELP,
        lockable: false,
        bounds: (1, 365),
    }

    /// The most days an access key can live, whatever date the person picks.
    AUTH_ACCESS_KEY_LONGEST_DAYS: Integer = {
        key: "auth.access-key-longest-days",
        default: 365,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_ACCESS_KEY_LONGEST_DAYS_HELP,
        lockable: false,
        bounds: (1, 365),
    }
}
