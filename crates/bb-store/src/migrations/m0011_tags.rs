//! Migration 11: tags, labels and runs of changes to many issues (TAG-01 … TAG-03, ISS-09,
//! ADR-0017).
//!
//! A tag and a label are found by the name in lower case (`name_key`), which is unique, so two
//! people who add the same word at the same moment make one tag. A label is unique inside its
//! group (`group_key` is the group's ID, or empty for a label without one). The links between
//! issues and tags or labels carry no reference to the person who made them: a removed person
//! leaves a tombstone (PRIV-04). A run of a change to many issues keeps the issues, the versions
//! the preview read and how far the work is, so a restart goes on where it stopped.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    IssueLabelRow, IssueRow, IssueTagRow, LabelGroupRow, LabelRow, ManyChangeRunRow, ProjectRow,
    TagProjectRow, TagRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = tags();
    steps.extend(labels());
    steps.extend(runs());
    steps
}

fn tags() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TagRow::Table)
                .col(id_column(TagRow::Id).primary_key())
                .col(ColumnDef::new(TagRow::Name).text().not_null())
                .col(ColumnDef::new(TagRow::NameKey).text().not_null())
                .col(ColumnDef::new(TagRow::Color).text().not_null())
                .col(ColumnDef::new(TagRow::CreatedAt).big_integer().not_null())
                .col(ColumnDef::new(TagRow::Version).big_integer().not_null())
                .to_owned(),
        ),
        // Two names with one key are one tag: the people who add it at the same moment make one.
        Step::Index(
            Index::create()
                .name("tag_by_key")
                .table(TagRow::Table)
                .col(TagRow::NameKey)
                .unique()
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(TagProjectRow::Table)
                .col(id_column(TagProjectRow::Tag))
                .col(id_column(TagProjectRow::Project))
                .primary_key(
                    Index::create()
                        .col(TagProjectRow::Tag)
                        .col(TagProjectRow::Project),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(TagProjectRow::Table, TagProjectRow::Tag)
                        .to(TagRow::Table, TagRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(TagProjectRow::Table, TagProjectRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueTagRow::Table)
                .col(id_column(IssueTagRow::Issue))
                .col(id_column(IssueTagRow::Tag))
                // No reference to the person: a removed person leaves a tombstone (PRIV-04).
                .col(id_column(IssueTagRow::AddedBy))
                .col(
                    ColumnDef::new(IssueTagRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(Index::create().col(IssueTagRow::Issue).col(IssueTagRow::Tag))
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueTagRow::Table, IssueTagRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueTagRow::Table, IssueTagRow::Tag)
                        .to(TagRow::Table, TagRow::Id),
                )
                .to_owned(),
        ),
        // The issues of a tag are found without reading the others.
        Step::Index(
            Index::create()
                .name("issue_tag_by_tag")
                .table(IssueTagRow::Table)
                .col(IssueTagRow::Tag)
                .col(IssueTagRow::Issue)
                .to_owned(),
        ),
    ]
}

fn labels() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(LabelGroupRow::Table)
                .col(id_column(LabelGroupRow::Id).primary_key())
                .col(ColumnDef::new(LabelGroupRow::Name).text().not_null())
                .col(ColumnDef::new(LabelGroupRow::NameKey).text().not_null())
                .col(ColumnDef::new(LabelGroupRow::PickOne).boolean().not_null())
                .col(
                    ColumnDef::new(LabelGroupRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(LabelGroupRow::RetiredAt).big_integer())
                .col(
                    ColumnDef::new(LabelGroupRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("label_group_by_key")
                .table(LabelGroupRow::Table)
                .col(LabelGroupRow::NameKey)
                .unique()
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(LabelRow::Table)
                .col(id_column(LabelRow::Id).primary_key())
                .col(ColumnDef::new(LabelRow::Group).blob())
                .col(ColumnDef::new(LabelRow::GroupKey).text().not_null())
                .col(ColumnDef::new(LabelRow::Name).text().not_null())
                .col(ColumnDef::new(LabelRow::NameKey).text().not_null())
                .col(ColumnDef::new(LabelRow::Color).text().not_null())
                .col(ColumnDef::new(LabelRow::Position).big_integer().not_null())
                .col(ColumnDef::new(LabelRow::RetiredAt).big_integer())
                .col(ColumnDef::new(LabelRow::Version).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(LabelRow::Table, LabelRow::Group)
                        .to(LabelGroupRow::Table, LabelGroupRow::Id),
                )
                .to_owned(),
        ),
        // A name is used once in a group, and once among the labels that sit in no group.
        Step::Index(
            Index::create()
                .name("label_by_group_and_key")
                .table(LabelRow::Table)
                .col(LabelRow::GroupKey)
                .col(LabelRow::NameKey)
                .unique()
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(IssueLabelRow::Table)
                .col(id_column(IssueLabelRow::Issue))
                .col(id_column(IssueLabelRow::Label))
                // No reference to the person: a removed person leaves a tombstone (PRIV-04).
                .col(id_column(IssueLabelRow::AddedBy))
                .col(
                    ColumnDef::new(IssueLabelRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(IssueLabelRow::Issue)
                        .col(IssueLabelRow::Label),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueLabelRow::Table, IssueLabelRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(IssueLabelRow::Table, IssueLabelRow::Label)
                        .to(LabelRow::Table, LabelRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("issue_label_by_label")
                .table(IssueLabelRow::Table)
                .col(IssueLabelRow::Label)
                .col(IssueLabelRow::Issue)
                .to_owned(),
        ),
    ]
}

fn runs() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(ManyChangeRunRow::Table)
            .col(id_column(ManyChangeRunRow::Id).primary_key())
            // No reference to the person: a removed person leaves a tombstone (PRIV-04).
            .col(id_column(ManyChangeRunRow::Person))
            .col(ColumnDef::new(ManyChangeRunRow::Action).text().not_null())
            .col(ColumnDef::new(ManyChangeRunRow::Items).text().not_null())
            .col(ColumnDef::new(ManyChangeRunRow::Status).text().not_null())
            .col(ColumnDef::new(ManyChangeRunRow::Done).big_integer().not_null())
            .col(ColumnDef::new(ManyChangeRunRow::Changed).big_integer().not_null())
            .col(ColumnDef::new(ManyChangeRunRow::Skipped).text().not_null())
            .col(
                ColumnDef::new(ManyChangeRunRow::CreatedAt)
                    .big_integer()
                    .not_null(),
            )
            .col(ColumnDef::new(ManyChangeRunRow::FinishedAt).big_integer())
            .to_owned(),
    )]
}
