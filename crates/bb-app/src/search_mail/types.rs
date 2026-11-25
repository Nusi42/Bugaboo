//! What the use cases of e-mailed searches take and give back: inputs that change something are
//! named after what they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything given
//! back is wrapped in `Authorized<T>` by the use case.

use bb_core::id::SavedSearchId;
use bb_core::search_mail::MailPause;
use bb_policy::Masked;
use jiff::Timestamp;

/// Get a saved search by e-mail, or change when it comes: the person's own e-mailed search of it,
/// made or written anew. Starting it again after it paused is the same.
pub struct SetSearchMail {
    /// The secret of the session of the request: only a session, never a key (M10-Q20).
    pub session_token: String,
    /// The saved search, which the person may open.
    pub search: SavedSearchId,
    /// The weekdays, Monday first; at least one.
    pub days: [bool; 7],
    /// The whole hour of the day, in the person's time zone, 0 to 23.
    pub hour: i64,
    /// *Only when something is new or changed*.
    pub only_when_changed: bool,
}

/// Stop getting a saved search by e-mail.
pub struct StopSearchMail {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The saved search.
    pub search: SavedSearchId,
}

/// Ask for the e-mailed searches of the person.
pub struct SearchMailsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// One e-mailed search as its person sees it.
#[derive(Clone, Debug)]
pub struct SearchMailView {
    /// The saved search it sends.
    pub search: SavedSearchId,
    /// The name of the saved search, while the person may open it.
    pub name: Option<String>,
    /// The weekdays it goes on, Monday first.
    pub days: [bool; 7],
    /// The hour it goes at, in the person's time zone.
    pub hour: i8,
    /// *Only when something is new or changed*.
    pub only_when_changed: bool,
    /// Why it sends nothing until the person starts it again, while it does.
    pub paused: Option<MailPause>,
    /// When its last e-mail went.
    pub last_sent_at: Option<Timestamp>,
}

impl Masked for SearchMailView {}

/// The e-mailed searches of a person, with what the window and the list say about all of them.
#[derive(Clone, Debug)]
pub struct SearchMailList {
    /// The e-mailed searches, oldest first; at most [`bb_core::search_mail::SEARCH_MAILS_MAX`].
    pub items: Vec<SearchMailView>,
    /// The most a person may have.
    pub most: u32,
    /// The most issues one e-mail lists.
    pub most_listed: u32,
    /// Whether e-mail and *E-mail me this search* are switched on for the organization: off, none
    /// sends anything until they are on again.
    pub sending: bool,
    /// Whether the organization uses *Short e-mails*: the e-mails have no titles and no name.
    pub short_e_mails: bool,
}

impl Masked for SearchMailList {}
