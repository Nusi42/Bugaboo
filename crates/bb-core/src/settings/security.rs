//! The setting of where security problems are reported (COMP-03, ADR-0047 decision 4, M9-Q16):
//! the organization's own contact, which `/.well-known/security.txt` names before ours. Empty at the
//! start, so only ours is named. Administrators change it with the other organization settings.

use super::{ChangedBy, Format, Level, Setting, SettingDefinition, Text, settings};
use crate::message;

settings! {
    /// The organization's contact for security reports: an e-mail address or an `https://` page
    /// (`bb_core::release::contact_address` reads it). Anything else is refused when it is saved,
    /// and left out of `security.txt` should a database hold it.
    SECURITY_CONTACT: Text = {
        key: "security.contact",
        name: message::SETTING_SECURITY_CONTACT_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_CONTACT_HELP,
        lockable: false,
        format: Format::SecurityContact,
    }
}
