//! What the notification use cases take and give back (NTF-01, SEC-06).
//!
//! A notification is shaped for the person who reads it, at the moment they read it: the fields
//! they may not see are not in it, the people are shown as the kinds of people information say,
//! and an issue they may not see any more is not named: the notification is left out whole.

use bb_core::edit::NoticeKind;
use bb_core::id::{CommentId, EntityId, NotificationId};
use bb_core::notification::{NotificationFilter, NotificationKind};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::comments::CommentView;
use crate::history::{FieldChangeView, HistoryWhat};
use crate::relations::IssueLine;

/// The notifications of the person who asks.
pub struct NotificationsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Which ones.
    pub filter: NotificationFilter,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many a page holds at most.
    pub limit: u32,
}

/// What a notification says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotificationWhat {
    /// A comment, as the person sees it now.
    Comment {
        /// The comment.
        comment: CommentView,
    },
    /// Fields of the issue that changed, only those the person may see.
    Changes {
        /// The fields.
        fields: Vec<FieldChangeView>,
    },
    /// Somebody asked the person to finish editing a text, or took the editing over.
    EditNotice {
        /// Which of the two.
        notice: NoticeKind,
        /// The part of the record that was being edited (`description`, `text`).
        part: String,
    },
    /// A change to many issues, or other work the person asked for, has ended.
    RunEnded {
        /// The record of the run; the run can be read by its ID.
        run: EntityId,
        /// Whether it ended well: `false` for a run that failed.
        done: bool,
    },
    /// Something about the issue that the person has no words for here (a mention of a comment
    /// that is gone, an assignment): the kind says what it is.
    Plain,
}

/// One notification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NotificationItem {
    /// The ID of the notification: *Mark as read* names it.
    pub id: NotificationId,
    /// What it is about.
    pub kind: NotificationKind,
    /// When it happened.
    pub at: Timestamp,
    /// Whether the person has read it.
    pub read: bool,
    /// The issue it is about, when the person may see it; a notification about an issue they may
    /// not see is not in the list at all.
    pub issue: Option<IssueLine>,
    /// The comment *Open* goes to, when it is about one.
    pub comment: Option<CommentId>,
    /// Who did it, as the kind *Authors* says for this person; nobody for Bugaboo itself.
    pub by: Option<PersonRef>,
    /// What it says.
    pub what: NotificationWhat,
}

/// A page of the notifications of the person, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NotificationPage {
    /// The notifications.
    pub items: Vec<NotificationItem>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for NotificationPage {}

/// How many notifications the person has not read (the number on the menu entry).
pub struct UnreadQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// How many unread notifications the person has, at most `AT_ONCE_MAX`. Only the ones they
/// may still see are counted: no total ever counts what a person cannot see (ARCH-NO-COUNTERS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Unread {
    /// The number.
    pub unread: usize,
}

impl Masked for Unread {}

/// Which notifications are marked as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Which {
    /// These.
    These(Vec<NotificationId>),
    /// Every unread one.
    All,
}

/// Marks notifications of the person as read (*Mark as read*).
pub struct ReadNotifications {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Which.
    pub which: Which,
}

impl From<HistoryWhat> for NotificationWhat {
    fn from(what: HistoryWhat) -> Self {
        match what {
            HistoryWhat::Comment { comment } => Self::Comment { comment },
            HistoryWhat::Changes { fields } => Self::Changes { fields },
            HistoryWhat::Relation { .. } => Self::Plain,
        }
    }
}
