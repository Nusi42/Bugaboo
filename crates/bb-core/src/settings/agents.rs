//! The settings of agents (AI-30, ADR-0029): whether agents may work, for the organization and for
//! a project, and the most that any agent can be given as a limit. They are part of the registry
//! like every other setting ([`super::registry`] lists them); they stand in a file of their own so
//! the main catalog does not grow past its size. Administrators change them on *Organization
//! settings › Connections › Agents*.

use super::{Bool, ChangedBy, Integer, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// Whether agents may work (AI-30, ADR-0029): the organization's value is the default of every
    /// project, and a project can switch its agents off. When it is off, the agents of people are
    /// refused there at their next request.
    AGENTS_ENABLED: Bool = {
        key: "agents.enabled",
        name: message::SETTING_AGENTS_ENABLED_NAME,
        default: true,
        levels: [Organization, Project],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_ENABLED_HELP,
        lockable: false,
    }

    /// The most changes a minute any agent may be given as its limit (ADR-0029, decision 9).
    AGENTS_MAXIMUM_CHANGES_A_MINUTE: Integer = {
        key: "agents.maximum-changes-a-minute",
        name: message::SETTING_AGENTS_MAXIMUM_CHANGES_A_MINUTE_NAME,
        default: 60,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_MAXIMUM_CHANGES_A_MINUTE_HELP,
        lockable: false,
        bounds: (1, 10000),
    }

    /// The most issues at once any agent may be given as its limit.
    AGENTS_MAXIMUM_ISSUES_AT_ONCE: Integer = {
        key: "agents.maximum-issues-at-once",
        name: message::SETTING_AGENTS_MAXIMUM_ISSUES_AT_ONCE_NAME,
        default: 5,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_MAXIMUM_ISSUES_AT_ONCE_HELP,
        lockable: false,
        bounds: (1, 1000),
    }

    /// The most attempts on one issue any agent may be given as its limit.
    AGENTS_MAXIMUM_ATTEMPTS_ON_ONE_ISSUE: Integer = {
        key: "agents.maximum-attempts-on-one-issue",
        name: message::SETTING_AGENTS_MAXIMUM_ATTEMPTS_ON_ONE_ISSUE_NAME,
        default: 5,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_MAXIMUM_ATTEMPTS_ON_ONE_ISSUE_HELP,
        lockable: false,
        bounds: (1, 100),
    }

    /// Whether an attempt of an agent is accepted by itself when its checks pass (ADR-0029,
    /// decision 6, M7-Q8): off at the start; a project can switch it on, and a hand-over can turn it
    /// off again.
    AGENTS_ACCEPT_BY_ITSELF: Bool = {
        key: "agents.accept-by-itself",
        name: message::SETTING_AGENTS_ACCEPT_BY_ITSELF_NAME,
        default: false,
        levels: [Organization, Project],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_ACCEPT_BY_ITSELF_HELP,
        lockable: false,
    }

    /// After how many days a hand-over ends when the person set no end (ADR-0029, decision 3).
    AGENTS_HAND_OVER_DAYS: Integer = {
        key: "agents.hand-over-days",
        name: message::SETTING_AGENTS_HAND_OVER_DAYS_NAME,
        default: 14,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_HAND_OVER_DAYS_HELP,
        lockable: false,
        bounds: (1, 90),
    }

    /// After how many days a standing rule ends when the person set no end, at most 90.
    AGENTS_RULE_DAYS: Integer = {
        key: "agents.rule-days",
        name: message::SETTING_AGENTS_RULE_DAYS_NAME,
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AGENTS_RULE_DAYS_HELP,
        lockable: false,
        bounds: (1, 90),
    }
}
