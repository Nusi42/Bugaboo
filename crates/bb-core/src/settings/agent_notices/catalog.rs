//! The settings of the notifications of agent work: a switch for each kind and way, with the
//! starting values of ADR-0033, amendment M7.

use super::super::{Bool, ChangedBy, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// Whether a person hears in the app of: an agent asks you (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_ASKS_IN_APP: Bool = {
        key: "notifications.agent-asks-in-app",
        name: message::SETTING_NOTIFICATIONS_AGENT_ASKS_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_ASKS_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: an agent asks you (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_ASKS_BY_MAIL: Bool = {
        key: "notifications.agent-asks-by-mail",
        name: message::SETTING_NOTIFICATIONS_AGENT_ASKS_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_ASKS_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether a person hears in the app of: a plan waits for you (ADR-0033, amendment M7).
    NOTIFICATIONS_PLAN_WAITS_IN_APP: Bool = {
        key: "notifications.plan-waits-in-app",
        name: message::SETTING_NOTIFICATIONS_PLAN_WAITS_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_PLAN_WAITS_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: a plan waits for you (ADR-0033, amendment M7).
    NOTIFICATIONS_PLAN_WAITS_BY_MAIL: Bool = {
        key: "notifications.plan-waits-by-mail",
        name: message::SETTING_NOTIFICATIONS_PLAN_WAITS_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_PLAN_WAITS_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether a person hears in the app of: ready for review (ADR-0033, amendment M7).
    NOTIFICATIONS_READY_FOR_REVIEW_IN_APP: Bool = {
        key: "notifications.ready-for-review-in-app",
        name: message::SETTING_NOTIFICATIONS_READY_FOR_REVIEW_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_READY_FOR_REVIEW_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: ready for review (ADR-0033, amendment M7).
    NOTIFICATIONS_READY_FOR_REVIEW_BY_MAIL: Bool = {
        key: "notifications.ready-for-review-by-mail",
        name: message::SETTING_NOTIFICATIONS_READY_FOR_REVIEW_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_READY_FOR_REVIEW_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether a person hears in the app of: an agent gave up (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_GAVE_UP_IN_APP: Bool = {
        key: "notifications.agent-gave-up-in-app",
        name: message::SETTING_NOTIFICATIONS_AGENT_GAVE_UP_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_GAVE_UP_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: an agent gave up (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_GAVE_UP_BY_MAIL: Bool = {
        key: "notifications.agent-gave-up-by-mail",
        name: message::SETTING_NOTIFICATIONS_AGENT_GAVE_UP_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_GAVE_UP_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether a person hears in the app of: a standing rule runs out soon (ADR-0033, amendment M7).
    NOTIFICATIONS_RULE_RUNS_OUT_SOON_IN_APP: Bool = {
        key: "notifications.rule-runs-out-soon-in-app",
        name: message::SETTING_NOTIFICATIONS_RULE_RUNS_OUT_SOON_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_RULE_RUNS_OUT_SOON_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: a standing rule runs out soon (ADR-0033, amendment M7).
    NOTIFICATIONS_RULE_RUNS_OUT_SOON_BY_MAIL: Bool = {
        key: "notifications.rule-runs-out-soon-by-mail",
        name: message::SETTING_NOTIFICATIONS_RULE_RUNS_OUT_SOON_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_RULE_RUNS_OUT_SOON_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether a person hears in the app of: an agent was stopped (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_STOPPED_IN_APP: Bool = {
        key: "notifications.agent-stopped-in-app",
        name: message::SETTING_NOTIFICATIONS_AGENT_STOPPED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_STOPPED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether a person hears by e-mail of: an agent was stopped (ADR-0033, amendment M7).
    NOTIFICATIONS_AGENT_STOPPED_BY_MAIL: Bool = {
        key: "notifications.agent-stopped-by-mail",
        name: message::SETTING_NOTIFICATIONS_AGENT_STOPPED_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_AGENT_STOPPED_BY_MAIL_HELP,
        lockable: true,
    }
}
