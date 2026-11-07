//! The tables of comments, mentions, followers and the history.

use super::{Iden, StoredTable};

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

/// How a text that AI wrote was written (COMP-08, COMP-09): one row for each such part of a record.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TextMarkRow {
    #[iden = "text_mark"]
    Table,
    EntityKind,
    EntityId,
    Part,
    WrittenBy,
    App,
    Agent,
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
    Agent,
    Kind,
    Record,
    RecordKind,
    Changes,
}

stored_tables! {
    CommentRow => bb_core::comment::Comment {
        Id => id,
        Issue => issue,
        Author => author,
        Text => text,
        CreatedAt => created_at,
        EditedAt => edited_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    CommentVersionRow => bb_core::comment::CommentVersion {
        Comment => comment,
        Version => version,
        Text => text,
        SavedAt => saved_at,
    },
    MentionRow => bb_core::comment::Mention {
        Comment => comment,
        Recipient => recipient,
        Via => via,
    },
    FollowerRow => bb_core::follow::Follower {
        Issue => issue,
        Person => person,
        Reason => reason,
        Since => since,
    },
    HistoryEntryRow => bb_core::history::HistoryEntry {
        Event => event,
        Issue => issue,
        Position => position,
        At => at,
        By => by,
        Agent => agent,
        Kind => kind,
        Record => record,
        RecordKind => record_kind,
        Changes => changes,
    },
    TextMarkRow => bb_core::mark::TextMark {
        EntityKind => entity_kind,
        EntityId => entity_id,
        Part => part,
        WrittenBy => written_by,
        App => app,
        Agent => agent,
    },
}
