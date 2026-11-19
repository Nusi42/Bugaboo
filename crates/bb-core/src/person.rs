//! A person with an account: the first entity, and the one the others point to.

use jiff::Timestamp;
use jiff::civil::Date;

use crate::id::UserId;
use crate::notification::NotificationKind;
use crate::personal::Entity;

/// A person who can sign in to Bugaboo.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Person {
    /// The person's ID, fully random so it does not show when the account was made. It is a
    /// pseudonym for the person; after the person is removed it is kept only as the link to the
    /// "Former user …" tombstone.
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(subject)]
    pub id: UserId,
    /// The name others see, as far as *Who sees people* allows (ADR-0022).
    #[personal(category = "identity", basis = "contract")]
    pub display_name: String,
    /// The e-mail address used to sign in and to send notifications.
    #[personal(category = "identity", basis = "contract")]
    pub email: String,
    /// When the account was made.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
}

words! {
    /// Where a person stands (PRIV-04, ADR-0044, decisions 5 and 6): *active*, *switched off*
    /// (signs in to nothing, keeps every record) or *removed* (a tombstone shown as *Former user*).
    pub enum Standing {
        /// The person signs in and works.
        Active => "active",
        /// An administrator switched the person off: no sign-in, no key, no agent, no message.
        SwitchedOff => "switched-off",
        /// The person was removed for good; what is left shows *Former user* and the code.
        Removed => "removed",
    }
}

words_in_api!(
    Standing,
    "Where a person stands: active, switched off (signs in to nothing and keeps every record) or removed (shown as Former user)."
);

words! {
    /// Why a removed person's name is kept in the legal-hold field (ADR-0044, decision 6).
    pub enum HoldReason {
        /// Their work days are inside their keep period (`ArbZG` §16, ADR-0035, decision 7).
        WorkDays => "work-days",
    }
}

/// The record of a person who is not *active*: who switched them off or removed them and when,
/// and for a removed person the code of the tombstone and the legal-hold field (PRIV-04). An
/// active person has no such record.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct PersonStanding {
    /// The person.
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(subject)]
    pub person: UserId,
    /// *Switched off* or *removed*.
    #[not_personal]
    pub standing: Standing,
    /// When the person was switched off or removed.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub since: Timestamp,
    /// The administrator who did it; none when a restore did it again from the replay list
    /// (ADR-0044, decision 8), whose line names no administrator.
    #[personal(category = "activity", basis = "legitimate_interests")]
    #[person(actor)]
    pub by: Option<UserId>,
    /// The random code of a removed person, drawn by `bb_policy::former_user_label`; unique. A
    /// pseudonym: it stands for the person and tells nothing of them.
    #[personal(category = "identity", basis = "legitimate_interests")]
    pub label: Option<String>,
    /// The name of a removed person while their work days are kept: the only place it survives.
    #[personal(category = "identity", basis = "legal_obligation")]
    pub hold_name: Option<String>,
    /// Why the name is kept.
    #[not_personal]
    pub hold_reason: Option<HoldReason>,
    /// The last day the name is kept: the last kept work day plus the keep period.
    #[not_personal]
    pub hold_until: Option<Date>,
}

impl PersonStanding {
    /// Whether the person was removed.
    #[must_use]
    pub fn removed(&self) -> bool {
        self.standing == Standing::Removed
    }
}

/// The text a removed person's e-mail address is replaced by: the address is cleared, but the
/// column is unique and never empty. It holds the code of the tombstone and nothing of the person,
/// and its domain can never receive mail (RFC 2606).
#[must_use]
pub fn cleared_address(label: &str) -> String {
    format!("former-{}@{CLEARED_DOMAIN}", label.to_ascii_lowercase())
}

/// The domain of a cleared address.
const CLEARED_DOMAIN: &str = "removed.invalid";

/// A person with their standing, as one read of the store gives them: what every place that shows
/// a person needs to tell an active, a switched-off and a removed person apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownPerson {
    /// The person.
    pub person: Person,
    /// Their standing; none while they are active.
    pub standing: Option<PersonStanding>,
}

impl KnownPerson {
    /// Whether the person is active: neither switched off nor removed.
    #[must_use]
    pub const fn active(&self) -> bool {
        self.standing.is_none()
    }

    /// Whether the person hears of `kind`, in the app and by e-mail: an active person does; a
    /// switched-off person hears only of an administrator's export of their data, at the address
    /// on record; a removed person hears of nothing (ADR-0044, decisions 4 and 5). The one rule of
    /// who is told, for the notifications and for the e-mails that wait.
    #[must_use]
    pub fn hears(&self, kind: NotificationKind) -> bool {
        match &self.standing {
            None => true,
            Some(standing) => !standing.removed() && kind.reaches_a_switched_off_person(),
        }
    }

    /// Where the person stands: *active* while they have no standing.
    #[must_use]
    pub fn standing(&self) -> Standing {
        self.standing
            .as_ref()
            .map_or(Standing::Active, |standing| standing.standing)
    }
}
