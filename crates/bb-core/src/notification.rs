//! Notifications (NTF-01, NTF-02, ADR-0033): what people are told about, in the app and by e-mail.
//!
//! A notification is a row for one person, built from one domain event by the outbox reader
//! `notifications`. The row keeps the facts and the fields that the person could see when it was
//! built; what is shown, and what is put in an e-mail, is decided again when it is read or sent.
//!
//! Every number of the mechanism (the keep period, the 5 minutes of an e-mail, the size of a round)
//! is a named constant here, written once.

use jiff::Timestamp;

use crate::event::FieldChange;
use crate::id::{CommentId, EntityId, EventId, IssueId, NotificationId, UserId};
use crate::personal::Entity;
use crate::settings::{self, Bool, Setting};
use crate::time::SECONDS_PER_DAY;

/// The name of the outbox consumer that builds the notifications (ADR-0007): its place in the
/// outbox is kept under this name.
pub const CONSUMER: &str = "notifications";

/// How many events the reader reads in one go.
pub const BATCH: u64 = 200;

/// How long a notification is kept, in days (PRIV-05, ADR-0033 §5).
pub const KEEP_DAYS: i64 = 90;

/// How long changes to one issue wait for each other before one e-mail goes, in seconds
/// (M4-Q6): the first change starts the wait, everything that comes within it is in the same
/// e-mail.
pub const MAIL_BATCH_SECONDS: i64 = 300;

/// The most changed fields a notification keeps, so a change to a hundred fields of an issue does
/// not make a row a page long.
pub const FIELDS_MAX: usize = 25;

/// The most characters of an issue title, a comment or a field in an e-mail; the rest is cut and
/// the link leads to it.
pub const MAIL_TEXT_MAX: usize = 2000;

/// The most notifications one e-mail names; the rest are in the app.
pub const MAIL_ENTRIES_MAX: usize = 20;

/// The most notifications one request takes in: the count of unread ones looks at this many and
/// shows never more, so what a person cannot see any more costs a bounded amount of reading and no
/// total ever includes it (ARCH-NO-COUNTERS); one *Mark as read* names this many at most.
pub const AT_ONCE_MAX: usize = 100;

/// How long a notification is kept.
#[must_use]
pub const fn keep() -> jiff::SignedDuration {
    jiff::SignedDuration::from_secs(KEEP_DAYS * SECONDS_PER_DAY)
}

/// The moment before which a notification is too old to keep, at `now`.
#[must_use]
pub fn cutoff(now: Timestamp) -> Option<Timestamp> {
    now.checked_sub(keep()).ok()
}

/// How long changes to one issue are collected into one e-mail.
#[must_use]
pub const fn mail_batch() -> jiff::SignedDuration {
    jiff::SignedDuration::from_secs(MAIL_BATCH_SECONDS)
}

words! {
    /// What a notification is about: the rows of the table in ADR-0033 §1. A person decides for
    /// each kind whether they hear of it in the app, by e-mail or not at all (*Customize*).
    pub enum NotificationKind {
        /// The person, or a group or role they are in, was mentioned in a comment.
        Mentioned => "mentioned",
        /// An issue was assigned to the person.
        Assigned => "assigned",
        /// A status change, a comment or a file on an issue the person follows.
        FollowedActivity => "followed_activity",
        /// Other changes to fields of an issue the person follows.
        FollowedFields => "followed_fields",
        /// A run of many changes the person asked for has ended, or a job they asked for died.
        RunEnded => "run_ended",
        /// Somebody asked the person to finish editing, or took the editing over (*Ask … to
        /// finish*, *Edit anyway*).
        EditNotice => "edit_notice",
    }
    messages label = "notification.kind."
}

words_in_api!(
    NotificationKind,
    "What a notification is about: a mention, an assignment, activity on an issue the person follows, other changes to what they follow, the end of a run of many changes, or a note about their editing."
);

words! {
    /// Which notifications a list shows.
    pub enum NotificationFilter {
        /// Every one, newest first.
        All => "all",
        /// The ones not read yet.
        Unread => "unread",
        /// The ones that mention the person (the filter that replaces the *Mentions* block).
        Mentions => "mentions",
    }
}

words_in_api!(
    NotificationFilter,
    "Which notifications a list shows: all, the unread ones, or the ones that mention the person."
);

words! {
    /// Where a person hears of something.
    pub enum Via {
        /// In the app, under *Notifications*.
        App => "app",
        /// By e-mail.
        Mail => "mail",
    }
}

impl NotificationKind {
    /// The setting that says whether a person hears of this kind of event this way: one switch
    /// for each kind and way, with the starting values of M4-Q5 (ADR-0033 §1). The organization's
    /// value is the default of everybody; each person changes their own under *Customize*.
    #[must_use]
    pub const fn setting(self, via: Via) -> &'static Setting<Bool> {
        use settings::{
            NOTIFICATIONS_ASSIGNED_BY_MAIL, NOTIFICATIONS_ASSIGNED_IN_APP,
            NOTIFICATIONS_EDIT_NOTICE_BY_MAIL, NOTIFICATIONS_EDIT_NOTICE_IN_APP,
            NOTIFICATIONS_FOLLOWED_ACTIVITY_BY_MAIL, NOTIFICATIONS_FOLLOWED_ACTIVITY_IN_APP,
            NOTIFICATIONS_FOLLOWED_FIELDS_BY_MAIL, NOTIFICATIONS_FOLLOWED_FIELDS_IN_APP,
            NOTIFICATIONS_MENTIONED_BY_MAIL, NOTIFICATIONS_MENTIONED_IN_APP,
            NOTIFICATIONS_RUN_ENDED_BY_MAIL, NOTIFICATIONS_RUN_ENDED_IN_APP,
        };
        match (self, via) {
            (Self::Mentioned, Via::App) => &NOTIFICATIONS_MENTIONED_IN_APP,
            (Self::Mentioned, Via::Mail) => &NOTIFICATIONS_MENTIONED_BY_MAIL,
            (Self::Assigned, Via::App) => &NOTIFICATIONS_ASSIGNED_IN_APP,
            (Self::Assigned, Via::Mail) => &NOTIFICATIONS_ASSIGNED_BY_MAIL,
            (Self::FollowedActivity, Via::App) => &NOTIFICATIONS_FOLLOWED_ACTIVITY_IN_APP,
            (Self::FollowedActivity, Via::Mail) => &NOTIFICATIONS_FOLLOWED_ACTIVITY_BY_MAIL,
            (Self::FollowedFields, Via::App) => &NOTIFICATIONS_FOLLOWED_FIELDS_IN_APP,
            (Self::FollowedFields, Via::Mail) => &NOTIFICATIONS_FOLLOWED_FIELDS_BY_MAIL,
            (Self::RunEnded, Via::App) => &NOTIFICATIONS_RUN_ENDED_IN_APP,
            (Self::RunEnded, Via::Mail) => &NOTIFICATIONS_RUN_ENDED_BY_MAIL,
            (Self::EditNotice, Via::App) => &NOTIFICATIONS_EDIT_NOTICE_IN_APP,
            (Self::EditNotice, Via::Mail) => &NOTIFICATIONS_EDIT_NOTICE_BY_MAIL,
        }
    }
}

words! {
    /// Where the e-mail of a notification is.
    pub enum MailState {
        /// No e-mail is sent for it: the person did not want one, or e-mail was off.
        Never => "never",
        /// It waits for the 5 minutes to pass, to go with the other changes to the issue.
        Waiting => "waiting",
        /// It went.
        Sent => "sent",
    }
}

/// What makes a notification once for a person: a second one with the same key is not made, so
/// reading an event again, or an edited comment that still mentions the person, changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Once {
    /// The event the row was built from: the person hears of an event once.
    Event(EventId),
    /// The comment that mentions the person: they hear of a mention once, however often the
    /// comment is edited.
    Mention(CommentId),
}

// The ID of an event or of a comment: no personal data.
impl crate::personal::NotPersonal for Once {}

impl Once {
    const EVENT: &'static str = "event:";
    const MENTION: &'static str = "mention:";

    /// The text the key is stored as.
    #[must_use]
    pub fn key(self) -> String {
        match self {
            Self::Event(event) => format!("{}{event}", Self::EVENT),
            Self::Mention(comment) => format!("{}{comment}", Self::MENTION),
        }
    }

    /// The key a stored text stands for.
    #[must_use]
    pub fn of_key(text: &str) -> Option<Self> {
        if let Some(event) = text.strip_prefix(Self::EVENT) {
            return event.parse().ok().map(Self::Event);
        }
        text.strip_prefix(Self::MENTION)?
            .parse()
            .ok()
            .map(Self::Mention)
    }
}

/// A notification for one person (NTF-01).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Notification {
    /// The ID of the notification.
    #[not_personal]
    pub id: NotificationId,
    /// Who it is for.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub recipient: UserId,
    /// What it is about.
    #[not_personal]
    pub kind: NotificationKind,
    /// The issue it is about; none for the end of a run or a job that is not about one issue.
    #[not_personal]
    pub issue: Option<IssueId>,
    /// The event it was built from.
    #[not_personal]
    pub event: EventId,
    /// What makes it once for the person.
    #[not_personal]
    pub once: Once,
    /// The record it is about when that is not the issue: the comment, or the run.
    #[not_personal]
    pub record: Option<EntityId>,
    /// Who did it; nobody when Bugaboo did.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub by: Option<UserId>,
    /// When it happened.
    #[not_personal]
    pub at: Timestamp,
    /// The fields that changed, only those the person could see when the row was built. Read
    /// again with the person's mask when it is shown or sent.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub changes: Vec<FieldChange>,
    /// Whether the person hears of it in the app.
    #[not_personal]
    pub in_app: bool,
    /// Where its e-mail is.
    #[not_personal]
    pub mail: MailState,
    /// When the person read it; unread while empty.
    #[not_personal]
    pub read_at: Option<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntf_01_every_kind_has_a_setting_for_each_way_and_the_starting_values_are_the_answer_of_m4_q5()
     {
        // The table of ADR-0033 §1 / M4-Q5: (in the app, by e-mail).
        let expected = [
            (NotificationKind::Mentioned, (true, true)),
            (NotificationKind::Assigned, (true, true)),
            (NotificationKind::FollowedActivity, (true, false)),
            (NotificationKind::FollowedFields, (false, false)),
            (NotificationKind::RunEnded, (true, false)),
            (NotificationKind::EditNotice, (true, false)),
        ];
        assert_eq!(expected.len(), NotificationKind::ALL.len());
        for (kind, (app, mail)) in expected {
            assert_eq!(kind.setting(Via::App).default_value(), Ok(app), "{kind:?}");
            assert_eq!(
                kind.setting(Via::Mail).default_value(),
                Ok(mail),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn ntf_01_every_kind_has_a_sentence_for_the_customize_form() {
        for kind in NotificationKind::ALL {
            assert!(
                !crate::message::Message::from(kind.label())
                    .render()
                    .is_empty()
            );
        }
    }

    #[test]
    fn a_key_is_read_back_as_what_it_was_made_from() {
        let mut ids = crate::id::SequentialIds::default();
        let event = EventId::generate(&mut ids).unwrap();
        let comment = CommentId::generate(&mut ids).unwrap();
        for once in [Once::Event(event), Once::Mention(comment)] {
            assert_eq!(Once::of_key(&once.key()), Some(once));
        }
        assert_eq!(Once::of_key("mention:nothing"), None);
        assert_eq!(Once::of_key(""), None);
    }

    #[test]
    fn priv_05_a_notification_is_kept_for_ninety_days() {
        let now: Timestamp = "2027-04-01T00:00:00Z".parse().unwrap();
        assert_eq!(cutoff(now), Some("2027-01-01T00:00:00Z".parse().unwrap()));
    }

    #[test]
    fn ntf_02_changes_wait_five_minutes_for_each_other() {
        assert_eq!(mail_batch().as_secs(), 300);
    }
}
