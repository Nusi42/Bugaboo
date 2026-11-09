//! The setting of AI apps (AI-22, ADR-0042, decision 4): whether outside AI apps may register and
//! sign in. Administrators change it on *Organization settings › Connections › AI*.

use super::{Bool, ChangedBy, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// Whether AI apps may work (WP-15, ADR-0042 decision 4): the organization's value is the default
    /// of every project, and a project can switch AI apps off or on again below a level that is off.
    /// Off for the organization, no app registers or signs in and every grant is refused at its next
    /// call, but none is deleted, so switching it on again brings them back; off for a project, no
    /// app reads or changes anything in it. It does not depend on the switches of the assistant or of
    /// agents.
    AI_APPS_ENABLED: Bool = {
        key: "ai-apps.enabled",
        name: message::SETTING_AI_APPS_ENABLED_NAME,
        default: true,
        levels: [Organization, Project],
        changed_by: Administrators,
        help: message::SETTING_AI_APPS_ENABLED_HELP_ON,
        lockable: false,
    }
}
