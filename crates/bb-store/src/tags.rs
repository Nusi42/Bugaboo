//! The adapter of the tags port (`bb_app::tags::TagStore`): tags, labels and groups of labels, and
//! the links between them and issues (TAG-01 … TAG-03).
//!
//! Like the issues adapter, reads run in one snapshot and writes are associated functions that run
//! inside the transaction of a change. The unique keys (the name of a tag, the name of a label in
//! its group) stop two people who add the same word at the same moment from both succeeding.

use bb_app::tags::TagStore;
use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId, TagId, UserId};
use bb_core::own_fields::ChoiceColor;
use bb_core::tags::{
    IssueLabel, IssueSight, IssueTag, Label, LabelGroup, Tag, TagProject, name_key,
};
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, LikeExpr, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, STATEMENT_ITEMS, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    IssueLabelRow, IssueRow, IssueTagRow, LabelGroupRow, LabelRow, TagProjectRow, TagRow,
};

fn read_color(row: &Row, column: impl sea_query::Iden + Copy) -> Result<ChoiceColor, StoreError> {
    ChoiceColor::from_code(&row.get::<String>(column)?)
        .ok_or(StoreError::Corrupt("the color of a tag or a label"))
}

/// The condition that an issue (table `issue`) is one the person reads (SEC-06): in a project where
/// they see every issue, or one they reported where they see only their own; not in the trash unless
/// asked; and not of a type the sight hides in its project.
pub(crate) fn sight_condition(sight: &IssueSight) -> Cond {
    let readable = Cond::any()
        .add(
            Expr::col((IssueRow::Table, IssueRow::Project))
                .is_in(sight.all.iter().map(|project| project.stored())),
        )
        .add(
            Expr::col((IssueRow::Table, IssueRow::Project))
                .is_in(sight.own.iter().map(|project| project.stored()))
                .and(Expr::col((IssueRow::Table, IssueRow::ReportedBy)).eq(sight.person.stored())),
        );
    let visible = sight
        .hidden
        .iter()
        .fold(Cond::all(), |cond, (project, issue_type)| {
            cond.add(
                Expr::col((IssueRow::Table, IssueRow::Project))
                    .ne(project.stored())
                    .or(Expr::col((IssueRow::Table, IssueRow::IssueType)).ne(issue_type.stored())),
            )
        });
    let kept = if sight.with_trashed {
        Cond::all()
    } else {
        Cond::all().add(Expr::col((IssueRow::Table, IssueRow::DeletedAt)).is_null())
    };
    Cond::all().add(kept).add(readable).add(visible)
}

fn tag_columns() -> [TagRow; 6] {
    [
        TagRow::Id,
        TagRow::Name,
        TagRow::NameKey,
        TagRow::Color,
        TagRow::CreatedAt,
        TagRow::Version,
    ]
}

fn tag_select() -> SelectStatement {
    let mut select = Query::select();
    select.columns(tag_columns()).from(TagRow::Table);
    select
}

fn read_tag(row: &Row) -> Result<Tag, StoreError> {
    Ok(Tag {
        id: row.id::<TagId>(TagRow::Id)?,
        name: row.get(TagRow::Name)?,
        color: read_color(row, TagRow::Color)?,
        created_at: row.time(TagRow::CreatedAt)?,
        version: row.get(TagRow::Version)?,
    })
}

fn issue_tag_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueTagRow::Issue,
            IssueTagRow::Tag,
            IssueTagRow::AddedBy,
            IssueTagRow::AddedAt,
        ])
        .from(IssueTagRow::Table);
    select
}

fn read_issue_tag(row: &Row) -> Result<IssueTag, StoreError> {
    Ok(IssueTag {
        issue: row.id::<IssueId>(IssueTagRow::Issue)?,
        tag: row.id::<TagId>(IssueTagRow::Tag)?,
        added_by: row.id::<UserId>(IssueTagRow::AddedBy)?,
        added_at: row.time(IssueTagRow::AddedAt)?,
    })
}

fn group_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            LabelGroupRow::Id,
            LabelGroupRow::Name,
            LabelGroupRow::PickOne,
            LabelGroupRow::Project,
            LabelGroupRow::Position,
            LabelGroupRow::RetiredAt,
            LabelGroupRow::Version,
        ])
        .from(LabelGroupRow::Table);
    select
}

fn read_group(row: &Row) -> Result<LabelGroup, StoreError> {
    Ok(LabelGroup {
        id: row.id::<LabelGroupId>(LabelGroupRow::Id)?,
        name: row.get(LabelGroupRow::Name)?,
        pick_one: row.get(LabelGroupRow::PickOne)?,
        project: row.optional_id(LabelGroupRow::Project)?,
        position: row.get(LabelGroupRow::Position)?,
        retired_at: row.optional_time(LabelGroupRow::RetiredAt)?,
        version: row.get(LabelGroupRow::Version)?,
    })
}

fn label_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            LabelRow::Id,
            LabelRow::Group,
            LabelRow::Name,
            LabelRow::Color,
            LabelRow::Position,
            LabelRow::RetiredAt,
            LabelRow::Version,
        ])
        .from(LabelRow::Table);
    select
}

fn read_label(row: &Row) -> Result<Label, StoreError> {
    Ok(Label {
        id: row.id::<LabelId>(LabelRow::Id)?,
        group: row.optional_id::<LabelGroupId>(LabelRow::Group)?,
        name: row.get(LabelRow::Name)?,
        color: read_color(row, LabelRow::Color)?,
        position: row.get(LabelRow::Position)?,
        retired_at: row.optional_time(LabelRow::RetiredAt)?,
        version: row.get(LabelRow::Version)?,
    })
}

fn issue_label_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueLabelRow::Issue,
            IssueLabelRow::Label,
            IssueLabelRow::AddedBy,
            IssueLabelRow::AddedAt,
        ])
        .from(IssueLabelRow::Table);
    select
}

fn read_issue_label(row: &Row) -> Result<IssueLabel, StoreError> {
    Ok(IssueLabel {
        issue: row.id::<IssueId>(IssueLabelRow::Issue)?,
        label: row.id::<LabelId>(IssueLabelRow::Label)?,
        added_by: row.id::<UserId>(IssueLabelRow::AddedBy)?,
        added_at: row.time(IssueLabelRow::AddedAt)?,
    })
}

/// The key a label holds for its group: the group's ID, or nothing.
fn group_key(group: Option<LabelGroupId>) -> String {
    group.map(|group| group.to_string()).unwrap_or_default()
}

/// `text` as the middle of a `LIKE` pattern, with the characters `LIKE` reads as its own written
/// as themselves (the escape is `\`).
fn contains_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    pattern.push('%');
    for character in text.chars() {
        if matches!(character, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

/// The IDs in order and once each, cut into chunks that one statement can bind
/// ([`STATEMENT_ITEMS`]); asking about the chunks one after the other, each ordered by issue,
/// gives the same order as one statement would.
fn sorted_chunks(ids: &[IssueId]) -> Vec<Vec<Vec<u8>>> {
    let mut stored: Vec<Vec<u8>> = ids.iter().map(|id| id.stored()).collect();
    stored.sort();
    stored.dedup();
    stored.chunks(STATEMENT_ITEMS).map(<[_]>::to_vec).collect()
}

impl TagStore for AuthStorage {
    async fn tags_in(transaction: &mut Transaction) -> Result<Vec<Tag>, StoreError> {
        let mut select = tag_select();
        select
            .order_by(TagRow::NameKey, Order::Asc)
            .order_by(TagRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_tag)
            .collect()
    }

    async fn tag_projects_in(transaction: &mut Transaction) -> Result<Vec<TagProject>, StoreError> {
        let mut select = Query::select();
        select
            .columns([TagProjectRow::Tag, TagProjectRow::Project])
            .from(TagProjectRow::Table)
            .order_by(TagProjectRow::Tag, Order::Asc)
            .order_by(TagProjectRow::Project, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(TagProject {
                    tag: row.id::<TagId>(TagProjectRow::Tag)?,
                    project: row.id::<ProjectId>(TagProjectRow::Project)?,
                })
            })
            .collect()
    }

    async fn issue_tags_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<IssueTag>, StoreError> {
        let mut select = issue_tag_select();
        select
            .and_where(Expr::col(IssueTagRow::Issue).eq(issue.stored()))
            .order_by(IssueTagRow::AddedAt, Order::Asc)
            .order_by(IssueTagRow::Tag, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_issue_tag)
            .collect()
    }

    async fn label_groups_in(transaction: &mut Transaction) -> Result<Vec<LabelGroup>, StoreError> {
        let mut select = group_select();
        select
            .order_by(LabelGroupRow::Position, Order::Asc)
            .order_by(LabelGroupRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_group)
            .collect()
    }

    async fn labels_in(transaction: &mut Transaction) -> Result<Vec<Label>, StoreError> {
        let mut select = label_select();
        select
            .order_by(LabelRow::Position, Order::Asc)
            .order_by(LabelRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_label)
            .collect()
    }

    async fn issue_labels_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<IssueLabel>, StoreError> {
        let mut select = issue_label_select();
        select
            .and_where(Expr::col(IssueLabelRow::Issue).eq(issue.stored()))
            .order_by(IssueLabelRow::AddedAt, Order::Asc)
            .order_by(IssueLabelRow::Label, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_issue_label)
            .collect()
    }

    async fn issues_with_tag_in(
        transaction: &mut Transaction,
        tag: TagId,
        after: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<IssueId>, StoreError> {
        let mut select = Query::select();
        select
            .column(IssueTagRow::Issue)
            .from(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Tag).eq(tag.stored()))
            .order_by(IssueTagRow::Issue, Order::Asc)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(IssueTagRow::Issue).gt(after.stored()));
        }
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<IssueId>(IssueTagRow::Issue))
            .collect()
    }

    async fn issues_with_label_in(
        transaction: &mut Transaction,
        label: LabelId,
        after: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<IssueId>, StoreError> {
        let mut select = Query::select();
        select
            .column(IssueLabelRow::Issue)
            .from(IssueLabelRow::Table)
            .and_where(Expr::col(IssueLabelRow::Label).eq(label.stored()))
            .order_by(IssueLabelRow::Issue, Order::Asc)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(IssueLabelRow::Issue).gt(after.stored()));
        }
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<IssueId>(IssueLabelRow::Issue))
            .collect()
    }

    async fn tags_of_issues_in(
        transaction: &mut Transaction,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueTag>, StoreError> {
        let mut found = Vec::new();
        for chunk in sorted_chunks(&issues) {
            let mut select = issue_tag_select();
            select
                .and_where(Expr::col(IssueTagRow::Issue).is_in(chunk))
                .order_by(IssueTagRow::Issue, Order::Asc)
                .order_by(IssueTagRow::AddedAt, Order::Asc)
                .order_by(IssueTagRow::Tag, Order::Asc);
            for row in transaction.fetch_all(&select).await? {
                found.push(read_issue_tag(&row)?);
            }
        }
        Ok(found)
    }

    async fn labels_of_issues_in(
        transaction: &mut Transaction,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueLabel>, StoreError> {
        let mut found = Vec::new();
        for chunk in sorted_chunks(&issues) {
            let mut select = issue_label_select();
            select
                .and_where(Expr::col(IssueLabelRow::Issue).is_in(chunk))
                .order_by(IssueLabelRow::Issue, Order::Asc)
                .order_by(IssueLabelRow::AddedAt, Order::Asc)
                .order_by(IssueLabelRow::Label, Order::Asc);
            for row in transaction.fetch_all(&select).await? {
                found.push(read_issue_label(&row)?);
            }
        }
        Ok(found)
    }

    async fn used_tags(
        &self,
        sight: &IssueSight,
        part: &str,
        limit: u64,
    ) -> Result<Vec<Tag>, StoreError> {
        if sight.all.is_empty() && sight.own.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .distinct()
            .columns(tag_columns().map(|column| (TagRow::Table, column)))
            .from(TagRow::Table)
            .inner_join(
                IssueTagRow::Table,
                Expr::col((IssueTagRow::Table, IssueTagRow::Tag))
                    .equals((TagRow::Table, TagRow::Id)),
            )
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueTagRow::Table, IssueTagRow::Issue)),
            )
            .cond_where(sight_condition(sight))
            .and_where(
                Expr::col((TagRow::Table, TagRow::NameKey))
                    .like(LikeExpr::new(contains_pattern(&name_key(part))).escape('\\')),
            )
            .order_by((TagRow::Table, TagRow::NameKey), Order::Asc)
            .order_by((TagRow::Table, TagRow::Id), Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_tag)
            .collect()
    }

    async fn insert_tag(transaction: &mut Transaction, tag: &Tag) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(TagRow::Table)
            .columns(tag_columns())
            .values([
                Expr::val(tag.id.stored()),
                Expr::val(tag.name.as_str()),
                Expr::val(name_key(&tag.name)),
                Expr::val(tag.color.code()),
                Expr::val(to_micros(tag.created_at)),
                Expr::val(tag.version),
            ])
            .map_err(|_| StoreError::Corrupt("a tag"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_tag(
        transaction: &mut Transaction,
        id: TagId,
        name: &str,
        color: ChoiceColor,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(TagRow::Table)
            .value(TagRow::Name, name)
            .value(TagRow::NameKey, name_key(name))
            .value(TagRow::Color, color.code())
            .value(TagRow::Version, Expr::col(TagRow::Version).add(1))
            .and_where(Expr::col(TagRow::Id).eq(id.stored()))
            .and_where(Expr::col(TagRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_tag_projects(
        transaction: &mut Transaction,
        tag: TagId,
        projects: &[ProjectId],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(TagProjectRow::Table)
            .and_where(Expr::col(TagProjectRow::Tag).eq(tag.stored()));
        transaction.execute(&delete).await?;
        for project in projects {
            let mut insert = Query::insert();
            insert
                .into_table(TagProjectRow::Table)
                .columns([TagProjectRow::Tag, TagProjectRow::Project])
                .values([Expr::val(tag.stored()), Expr::val(project.stored())])
                .map_err(|_| StoreError::Corrupt("a project of a tag"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn delete_tag(transaction: &mut Transaction, id: TagId) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Tag).eq(id.stored()));
        transaction.execute(&delete).await?;
        let mut delete = Query::delete();
        delete
            .from_table(TagProjectRow::Table)
            .and_where(Expr::col(TagProjectRow::Tag).eq(id.stored()));
        transaction.execute(&delete).await?;
        let mut delete = Query::delete();
        delete
            .from_table(TagRow::Table)
            .and_where(Expr::col(TagRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn insert_issue_tag(
        transaction: &mut Transaction,
        link: &IssueTag,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueTagRow::Table)
            .columns([
                IssueTagRow::Issue,
                IssueTagRow::Tag,
                IssueTagRow::AddedBy,
                IssueTagRow::AddedAt,
            ])
            .values([
                Expr::val(link.issue.stored()),
                Expr::val(link.tag.stored()),
                Expr::val(link.added_by.stored()),
                Expr::val(to_micros(link.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a tag on an issue"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_issue_tag(
        transaction: &mut Transaction,
        issue: IssueId,
        tag: TagId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(IssueTagRow::Tag).eq(tag.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn retarget_issue_tags(
        transaction: &mut Transaction,
        from: TagId,
        into: TagId,
    ) -> Result<(), StoreError> {
        // The issues that have the tag `into` already keep that link; the others get one.
        let mut have_it = Query::select();
        have_it
            .column(IssueTagRow::Issue)
            .from(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Tag).eq(into.stored()));
        let mut moved = Query::select();
        moved
            .expr(Expr::val(into.stored()))
            .columns([
                IssueTagRow::Issue,
                IssueTagRow::AddedBy,
                IssueTagRow::AddedAt,
            ])
            .from(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Tag).eq(from.stored()))
            .and_where(Expr::col(IssueTagRow::Issue).not_in_subquery(have_it));
        let mut insert = Query::insert();
        insert
            .into_table(IssueTagRow::Table)
            .columns([
                IssueTagRow::Tag,
                IssueTagRow::Issue,
                IssueTagRow::AddedBy,
                IssueTagRow::AddedAt,
            ])
            .select_from(moved)
            .map_err(|_| StoreError::Corrupt("the tags of issues"))?;
        transaction.execute(&insert).await?;
        let mut delete = Query::delete();
        delete
            .from_table(IssueTagRow::Table)
            .and_where(Expr::col(IssueTagRow::Tag).eq(from.stored()));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn insert_label_group(
        transaction: &mut Transaction,
        group: &LabelGroup,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(LabelGroupRow::Table)
            .columns([
                LabelGroupRow::Id,
                LabelGroupRow::Name,
                LabelGroupRow::NameKey,
                LabelGroupRow::PickOne,
                LabelGroupRow::Project,
                LabelGroupRow::Position,
                LabelGroupRow::RetiredAt,
                LabelGroupRow::Version,
            ])
            .values([
                Expr::val(group.id.stored()),
                Expr::val(group.name.as_str()),
                Expr::val(name_key(&group.name)),
                Expr::val(group.pick_one),
                Expr::val(group.project.map(StoredId::stored)),
                Expr::val(group.position),
                Expr::val(group.retired_at.map(to_micros)),
                Expr::val(group.version),
            ])
            .map_err(|_| StoreError::Corrupt("a group of labels"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_label_group(
        transaction: &mut Transaction,
        id: LabelGroupId,
        name: &str,
        pick_one: bool,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(LabelGroupRow::Table)
            .value(LabelGroupRow::Name, name)
            .value(LabelGroupRow::NameKey, name_key(name))
            .value(LabelGroupRow::PickOne, pick_one)
            .value(LabelGroupRow::RetiredAt, retired_at.map(to_micros))
            .value(
                LabelGroupRow::Version,
                Expr::col(LabelGroupRow::Version).add(1),
            )
            .and_where(Expr::col(LabelGroupRow::Id).eq(id.stored()))
            .and_where(Expr::col(LabelGroupRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_label(transaction: &mut Transaction, label: &Label) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(LabelRow::Table)
            .columns([
                LabelRow::Id,
                LabelRow::Group,
                LabelRow::GroupKey,
                LabelRow::Name,
                LabelRow::NameKey,
                LabelRow::Color,
                LabelRow::Position,
                LabelRow::RetiredAt,
                LabelRow::Version,
            ])
            .values([
                Expr::val(label.id.stored()),
                Expr::val(label.group.map(StoredId::stored)),
                Expr::val(group_key(label.group)),
                Expr::val(label.name.as_str()),
                Expr::val(name_key(&label.name)),
                Expr::val(label.color.code()),
                Expr::val(label.position),
                Expr::val(label.retired_at.map(to_micros)),
                Expr::val(label.version),
            ])
            .map_err(|_| StoreError::Corrupt("a label"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_label(
        transaction: &mut Transaction,
        id: LabelId,
        name: &str,
        color: ChoiceColor,
        group: Option<LabelGroupId>,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(LabelRow::Table)
            .value(LabelRow::Name, name)
            .value(LabelRow::NameKey, name_key(name))
            .value(LabelRow::Color, color.code())
            .value(LabelRow::Group, group.map(StoredId::stored))
            .value(LabelRow::GroupKey, group_key(group))
            .value(LabelRow::RetiredAt, retired_at.map(to_micros))
            .value(LabelRow::Version, Expr::col(LabelRow::Version).add(1))
            .and_where(Expr::col(LabelRow::Id).eq(id.stored()))
            .and_where(Expr::col(LabelRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_issue_label(
        transaction: &mut Transaction,
        link: &IssueLabel,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueLabelRow::Table)
            .columns([
                IssueLabelRow::Issue,
                IssueLabelRow::Label,
                IssueLabelRow::AddedBy,
                IssueLabelRow::AddedAt,
            ])
            .values([
                Expr::val(link.issue.stored()),
                Expr::val(link.label.stored()),
                Expr::val(link.added_by.stored()),
                Expr::val(to_micros(link.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a label on an issue"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_issue_label(
        transaction: &mut Transaction,
        issue: IssueId,
        label: LabelId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueLabelRow::Table)
            .and_where(Expr::col(IssueLabelRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(IssueLabelRow::Label).eq(label.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn retarget_issue_labels(
        transaction: &mut Transaction,
        from: LabelId,
        into: LabelId,
    ) -> Result<(), StoreError> {
        let mut have_it = Query::select();
        have_it
            .column(IssueLabelRow::Issue)
            .from(IssueLabelRow::Table)
            .and_where(Expr::col(IssueLabelRow::Label).eq(into.stored()));
        let mut moved = Query::select();
        moved
            .expr(Expr::val(into.stored()))
            .columns([
                IssueLabelRow::Issue,
                IssueLabelRow::AddedBy,
                IssueLabelRow::AddedAt,
            ])
            .from(IssueLabelRow::Table)
            .and_where(Expr::col(IssueLabelRow::Label).eq(from.stored()))
            .and_where(Expr::col(IssueLabelRow::Issue).not_in_subquery(have_it));
        let mut insert = Query::insert();
        insert
            .into_table(IssueLabelRow::Table)
            .columns([
                IssueLabelRow::Label,
                IssueLabelRow::Issue,
                IssueLabelRow::AddedBy,
                IssueLabelRow::AddedAt,
            ])
            .select_from(moved)
            .map_err(|_| StoreError::Corrupt("the labels of issues"))?;
        transaction.execute(&insert).await?;
        let mut delete = Query::delete();
        delete
            .from_table(IssueLabelRow::Table)
            .and_where(Expr::col(IssueLabelRow::Label).eq(from.stored()));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn delete_label(transaction: &mut Transaction, id: LabelId) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(LabelRow::Table)
            .and_where(Expr::col(LabelRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn lock_tags_and_labels(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction
            .advisory_lock(crate::locks::LockClass::TagsAndLabels, 0)
            .await
    }
}

#[cfg(test)]
#[path = "tests/tags.rs"]
mod tests;
