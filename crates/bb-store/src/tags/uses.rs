//! The one statement that says what decides whether a person may know a tag (SEC-06): whether the
//! tag is used on an issue they read, whether any issue has it, and its limits to projects.

use bb_app::tags::{TagLimit, TagUse};
use bb_core::id::ProjectId;
use bb_core::tags::IssueSight;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::{read_tag, tag_columns};
use crate::auth::AuthStorage;
use crate::db::StoredId as _;
use crate::error::StoreError;
use crate::schema::{IssueRow, IssueTagRow, TagProjectRow, TagRow};
use crate::sight::sight_condition;

/// The columns of the question whether a tag is used.
#[derive(Clone, Copy)]
enum UseColumn {
    Seen,
    Anywhere,
    Limited,
    LimitedToVisible,
}

impl sea_query::Iden for UseColumn {
    fn unquoted(&self) -> &str {
        match self {
            Self::Seen => "seen",
            Self::Anywhere => "anywhere",
            Self::Limited => "limited",
            Self::LimitedToVisible => "limited_to_visible",
        }
    }
}

/// Every tag with the answers the rule of what a person may know of a tag needs, in one statement.
pub(super) async fn tags_with_use(
    storage: &AuthStorage,
    seen: &IssueSight,
    anywhere: &IssueSight,
    visible: &[ProjectId],
) -> Result<Vec<TagUse>, StoreError> {
    // A tag is used when one issue of the sight has it: the question stops at the first.
    let used_in = |sight: &IssueSight| {
        Expr::exists(
            Query::select()
                .expr(Expr::val(1))
                .from(IssueTagRow::Table)
                .inner_join(
                    IssueRow::Table,
                    Expr::col((IssueRow::Table, IssueRow::Id))
                        .equals((IssueTagRow::Table, IssueTagRow::Issue)),
                )
                .and_where(
                    Expr::col((IssueTagRow::Table, IssueTagRow::Tag))
                        .equals((TagRow::Table, TagRow::Id)),
                )
                .cond_where(sight_condition(sight))
                .to_owned(),
        )
    };
    // A tag is limited when a row says so, and limited to a project the person sees when one
    // of its rows is for such a project.
    let limits = |only: Option<&[ProjectId]>| {
        let mut limit = Query::select();
        limit
            .expr(Expr::val(1))
            .from(TagProjectRow::Table)
            .and_where(
                Expr::col((TagProjectRow::Table, TagProjectRow::Tag))
                    .equals((TagRow::Table, TagRow::Id)),
            );
        if let Some(projects) = only {
            limit.and_where(
                Expr::col((TagProjectRow::Table, TagProjectRow::Project))
                    .is_in(projects.iter().map(|project| project.stored())),
            );
        }
        Expr::exists(limit.clone())
    };
    let mut select = Query::select();
    select
        .columns(tag_columns().map(|column| (TagRow::Table, column)))
        .expr_as(used_in(seen), UseColumn::Seen)
        .expr_as(used_in(anywhere), UseColumn::Anywhere)
        .expr_as(limits(None), UseColumn::Limited)
        .expr_as(limits(Some(visible)), UseColumn::LimitedToVisible)
        .from(TagRow::Table)
        .order_by((TagRow::Table, TagRow::NameKey), Order::Asc)
        .order_by((TagRow::Table, TagRow::Id), Order::Asc);
    storage
        .database()
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok(TagUse {
                tag: read_tag(row)?,
                seen: row.get::<bool>(UseColumn::Seen)?,
                anywhere: row.get::<bool>(UseColumn::Anywhere)?,
                limit: if row.get::<bool>(UseColumn::LimitedToVisible)? {
                    TagLimit::Visible
                } else if row.get::<bool>(UseColumn::Limited)? {
                    TagLimit::Hidden
                } else {
                    TagLimit::None
                },
            })
        })
        .collect()
}
