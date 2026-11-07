//! The setting of AI apps (AI-22, ADR-0042, decision 4): whether outside AI apps may register and
//! sign in. Administrators change it on *Organization settings › Connections › AI*.

use super::{Bool, ChangedBy, Level, Setting, SettingDefinition, settings};
use crate::message;

settings! {
    /// Whether AI apps may register and sign in. Off, no app registers or signs in and every grant
    /// is refused at its next call, but none is deleted, so switching it on again brings them back.
    AI_APPS_ENABLED: Bool = {
        key: "ai-apps.enabled",
        name: message::SETTING_AI_APPS_ENABLED_NAME,
        default: true,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AI_APPS_ENABLED_HELP,
        lockable: false,
    }
}
