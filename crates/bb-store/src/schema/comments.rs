//! The tables of comments, mentions, followers and the history.

use super::Iden;

/// Comments under issues (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum CommentRow {
    #[iden = "comment"]
    Table,
    Id,
    Issue,
    Author,
    Text,
    CreatedAt,
    EditedAt,
    DeletedAt,
    Version,
}

/// The earlier texts of edited comments (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum CommentVersionRow {
    #[iden = "comment_version"]
    Table,
    Comment,
    Version,
    Text,
    SavedAt,
}

/// The people a comment puts into *Mentions* (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum MentionRow {
    #[iden = "mention"]
    Table,
    Comment,
    Recipient,
    Via,
}

/// The people who follow an issue (ISS-07).
#[derive(Clone, Copy, Iden)]
pub(crate) enum FollowerRow {
    #[iden = "follower"]
    Table,
    Issue,
    Person,
    Reason,
    Since,
}

/// The history of issues, built from the events by an outbox consumer (ISS-06).
#[derive(Clone, Copy, Iden)]
pub(crate) enum HistoryEntryRow {
    #[iden = "history_entry"]
    Table,
    Event,
    Issue,
    Position,
    At,
    By,
    Kind,
    Record,
    RecordKind,
    Changes,
}
