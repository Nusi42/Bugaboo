//! Migration 10: sub-issues, related issues, comments, mentions, followers and the history of
//! issues (ISS-03 … ISS-07, ADR-0007, ADR-0027).
//!
//! A relation is stored once, from the issue that reads the forward name of its kind to the issue
//! that reads the backward name; the unique key keeps the same relation from being added twice.
//! A comment keeps its earlier texts in `comment_version`. `mention` has one row for each comment
//! and person who is put into *Mentions*, by the person's name, by a group or by a role.
//! `history_entry` is the read model that an outbox consumer builds: one row for each event and
//! issue, so replaying an event changes nothing.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{
    CommentRow, CommentVersionRow, FollowerRow, HistoryEntryRow, IssueParentRow, IssueRow,
    MentionRow, RelationKindRow, RelationRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = relations();
    steps.extend(comments());
    steps.extend(followers());
    steps.extend(history());
    steps
}

fn relations() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(RelationKindRow::Table)
                .col(
                    ColumnDef::new(RelationKindRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(RelationKindRow::Forward).text().not_null())
                .col(ColumnDef::new(RelationKindRow::Backward).text().not_null())
                .col(
                    ColumnDef::new(RelationKindRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(RelationKindRow::RetiredAt).big_integer())
                .col(
                    ColumnDef::new(RelationKindRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(RelationRow::Table)
                .col(
                    ColumnDef::new(RelationRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(RelationRow::Kind).blob().not_null())
                .col(ColumnDef::new(RelationRow::FromIssue).blob().not_null())
                .col(ColumnDef::new(RelationRow::ToIssue).blob().not_null())
                // No reference to the person: a removed person leaves a tombstone (PRIV-04).
                .col(ColumnDef::new(RelationRow::AddedBy).blob().not_null())
                .col(
                    ColumnDef::new(RelationRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RelationRow::Table, RelationRow::Kind)
                        .to(RelationKindRow::Table, RelationKindRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RelationRow::Table, RelationRow::FromIssue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(RelationRow::Table, RelationRow::ToIssue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The same relation is added once: two people who add it at the same moment cannot both
        // succeed.
        Step::Index(
            Index::create()
                .name("relation_by_pair")
                .table(RelationRow::Table)
                .col(RelationRow::Kind)
                .col(RelationRow::FromIssue)
                .col(RelationRow::ToIssue)
                .unique()
                .to_owned(),
        ),
        // The relations of an issue are found from either end.
        Step::Index(
            Index::create()
                .name("relation_by_from")
                .table(RelationRow::Table)
                .col(RelationRow::FromIssue)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("relation_by_to")
                .table(RelationRow::Table)
                .col(RelationRow::ToIssue)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueParentRow::Table)
                .col(
                    ColumnDef::new(IssueParentRow::Child)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(IssueParentRow::Parent).blob().not_null())
                .col(ColumnDef::new(IssueParentRow::SetBy).blob().not_null())
                .col(
                    ColumnDef::new(IssueParentRow::SetAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueParentRow::Table, IssueParentRow::Child)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueParentRow::Table, IssueParentRow::Parent)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_parent_by_parent")
                .table(IssueParentRow::Table)
                .col(IssueParentRow::Parent)
                .col(IssueParentRow::Child)
                .to_owned(),
        ),
    ]
}

fn comments() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(CommentRow::Table)
                .col(
                    ColumnDef::new(CommentRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(CommentRow::Issue).blob().not_null())
                .col(ColumnDef::new(CommentRow::Author).blob().not_null())
                .col(ColumnDef::new(CommentRow::Text).text().not_null())
                .col(
                    ColumnDef::new(CommentRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(CommentRow::EditedAt).big_integer())
                .col(ColumnDef::new(CommentRow::DeletedAt).big_integer())
                .col(ColumnDef::new(CommentRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(CommentRow::Table, CommentRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The comments of an issue in the order they were written.
        Step::Index(
            Index::create()
                .name("comment_by_issue")
                .table(CommentRow::Table)
                .col(CommentRow::Issue)
                .col(CommentRow::Id)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(CommentVersionRow::Table)
                .col(ColumnDef::new(CommentVersionRow::Comment).blob().not_null())
                .col(
                    ColumnDef::new(CommentVersionRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(CommentVersionRow::Text).text().not_null())
                .col(
                    ColumnDef::new(CommentVersionRow::SavedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(CommentVersionRow::Comment)
                        .col(CommentVersionRow::Version),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(CommentVersionRow::Table, CommentVersionRow::Comment)
                        .to(CommentRow::Table, CommentRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(MentionRow::Table)
                .col(ColumnDef::new(MentionRow::Comment).blob().not_null())
                .col(ColumnDef::new(MentionRow::Recipient).blob().not_null())
                .col(ColumnDef::new(MentionRow::Via).text().not_null())
                .col(ColumnDef::new(MentionRow::ReadAt).big_integer())
                .primary_key(
                    Index::create()
                        .col(MentionRow::Comment)
                        .col(MentionRow::Recipient),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(MentionRow::Table, MentionRow::Comment)
                        .to(CommentRow::Table, CommentRow::Id),
                )
                .to_owned(),
        ),
        // *Mentions* of a person, found without reading the others.
        Step::Index(
            Index::create()
                .name("mention_by_recipient")
                .table(MentionRow::Table)
                .col(MentionRow::Recipient)
                .col(MentionRow::Comment)
                .to_owned(),
        ),
    ]
}

fn followers() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(FollowerRow::Table)
                .col(ColumnDef::new(FollowerRow::Issue).blob().not_null())
                .col(ColumnDef::new(FollowerRow::Person).blob().not_null())
                .col(ColumnDef::new(FollowerRow::Reason).text().not_null())
                .col(ColumnDef::new(FollowerRow::Since).big_integer().not_null())
                .primary_key(
                    Index::create()
                        .col(FollowerRow::Issue)
                        .col(FollowerRow::Person),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(FollowerRow::Table, FollowerRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The issues one person follows.
        Step::Index(
            Index::create()
                .name("follower_by_person")
                .table(FollowerRow::Table)
                .col(FollowerRow::Person)
                .col(FollowerRow::Issue)
                .to_owned(),
        ),
    ]
}

fn history() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(HistoryEntryRow::Table)
                .col(ColumnDef::new(HistoryEntryRow::Event).blob().not_null())
                .col(ColumnDef::new(HistoryEntryRow::Issue).blob().not_null())
                .col(
                    ColumnDef::new(HistoryEntryRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(HistoryEntryRow::At)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(HistoryEntryRow::By).blob())
                .col(ColumnDef::new(HistoryEntryRow::Kind).text().not_null())
                .col(ColumnDef::new(HistoryEntryRow::Record).blob().not_null())
                .col(
                    ColumnDef::new(HistoryEntryRow::RecordKind)
                        .text()
                        .not_null(),
                )
                .col(ColumnDef::new(HistoryEntryRow::Changes).text().not_null())
                // One row for each event and issue: the consumer can read an event again.
                .primary_key(
                    Index::create()
                        .col(HistoryEntryRow::Event)
                        .col(HistoryEntryRow::Issue),
                )
                .to_owned(),
        ),
        // The history of one issue in the order of the outbox.
        Step::Index(
            Index::create()
                .name("history_entry_by_issue")
                .table(HistoryEntryRow::Table)
                .col(HistoryEntryRow::Issue)
                .col(HistoryEntryRow::Position)
                .to_owned(),
        ),
    ]
}
