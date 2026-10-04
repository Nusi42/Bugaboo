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

    /// The roles whose members must use two-step sign-in (a list separated by commas). Takes
    /// effect once roles exist (M2).
    SECURITY_TWO_STEP_REQUIRED_ROLES: Text = {
        key: "security.two-step-required-roles",
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_TWO_STEP_REQUIRED_ROLES_HELP,
        lockable: false,
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
