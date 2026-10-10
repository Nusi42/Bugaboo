//! Reading the history of an issue (ISS-06, SEC-06): the rows that `build.rs` made, shaped for one
//! reader.
//!
//! The one rule of the screen is that **a change to a field the reader may not see is not in the
//! history**: it is left out whole, not shown as hidden, and a row whose changes are all left out
//! is left out too. What a reader may see of a change of an issue is decided by the names of the
//! members of the events (`issues::member`): a field of the catalogue by the mask of the reader in
//! the project of the issue, the issue type, the parent and the sub-issues as open, everything else
//! never. A person in a value is shown as the kind of the field says, an issue by a line that the
//! reader may see, or not at all.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::event::FieldChange;
use bb_core::fields::Field;
use bb_core::history::{HistoryEntry, HistoryKind, HistoryTab};
use bb_core::id::{CommentId, IssueId, IssueTypeId, UserId};
use bb_core::paging::page_len;
use bb_core::people_kinds::PeopleKind;
use bb_core::relation::Direction;
use bb_policy::{Authorized, Evaluation, PeopleContext, Place};
use serde_json::Value;

use super::build::catch_up;
use super::types::{
    ChangeValue, FieldChangeView, HistoryItem, HistoryPage, HistoryQuery, HistoryWhat,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::comments::CommentView;
use crate::issues::member as issue_member;
use crate::member::relation as relation_member;
use crate::relations::LineMaker;
use bb_core::comment::Comment;
use bb_core::issue::Issue;

/// How a member of the event of an issue is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Member {
    /// A field of the catalogue: shown when the reader's mask lets them see it.
    Catalogue(Field),
    /// The issue type: shown to everyone who sees the issue.
    IssueType,
    /// The issue it sits under, or an issue that sits under it: shown as a line the reader may
    /// see.
    Issue,
    /// Never shown: the version, the project, the readable ID, the trash.
    Never,
}

impl Member {
    /// The way the member called `name` is shown. The one mapping from the names of the events to
    /// what a reader sees: a field of the catalogue is known by its key, the rest is listed here.
    pub(crate) fn of(name: &str) -> Self {
        if let Some(field) = Field::from_code(name) {
            return Self::Catalogue(field);
        }
        match name {
            issue_member::ISSUE_TYPE => Self::IssueType,
            issue_member::PARENT | issue_member::SUB_ISSUE => Self::Issue,
            _ => Self::Never,
        }
    }

    /// Whether a reader may see a change of the member, given which fields of the catalogue they
    /// may see (their mask).
    pub(crate) fn may_show(self, can_see: &impl Fn(Field) -> bool) -> bool {
        match self {
            Self::Catalogue(field) => can_see(field),
            Self::IssueType | Self::Issue => true,
            Self::Never => false,
        }
    }
}

/// The changes of an event that a reader may see, in the order of the event, given which fields of
/// the catalogue they may see (their mask).
pub(crate) fn visible_changes<'a>(
    can_see: &impl Fn(Field) -> bool,
    changes: &'a [FieldChange],
) -> Vec<&'a FieldChange> {
    changes
        .iter()
        .filter(|change| Member::of(&change.field).may_show(can_see))
        .collect()
}

/// The words of a JSON value as a change shows it.
fn plain_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(plain_text).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The people a value names, as the kind says for the reader.
    async fn people_in(
        &self,
        held: &Evaluation,
        kind: PeopleKind,
        value: &Value,
    ) -> Result<ChangeValue, AppError> {
        let ids: Vec<&str> = match value {
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            Value::String(text) => vec![text.as_str()],
            _ => Vec::new(),
        };
        let mut people = Vec::with_capacity(ids.len());
        for text in ids {
            if let Ok(person) = text.parse::<UserId>() {
                people.push(
                    self.person_ref(held, kind, PeopleContext::Ordinary, person)
                        .await?,
                );
            }
        }
        Ok(ChangeValue::People { people })
    }

    /// One value of a change as the reader may see it; `None` when there is no value, the value
    /// is a long text, or it names an issue the reader may not see.
    async fn change_value(
        &self,
        actor: &Actor,
        held: &Evaluation,
        maker: &mut LineMaker,
        member: Member,
        value: Option<&Value>,
    ) -> Result<Option<ChangeValue>, AppError> {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return Ok(None);
        };
        Ok(match member {
            Member::Catalogue(Field::Description) | Member::Never => None,
            Member::Catalogue(field) => match field.person_kind() {
                Some(kind) => Some(self.people_in(held, kind, value).await?),
                None => Some(ChangeValue::Text {
                    text: plain_text(value),
                }),
            },
            Member::IssueType => value
                .as_str()
                .and_then(|text| text.parse::<IssueTypeId>().ok())
                .and_then(|id| maker.type_name(id))
                .map(|name| ChangeValue::Text {
                    text: name.to_owned(),
                }),
            Member::Issue => {
                let Some(id) = value.as_str().and_then(|text| text.parse::<IssueId>().ok()) else {
                    return Ok(None);
                };
                match self.issues_named(&[id]).await?.first() {
                    Some(issue) => self
                        .line_of(actor, maker, issue)
                        .await?
                        .map(|issue| ChangeValue::Issue { issue }),
                    None => None,
                }
            }
        })
    }

    /// The fields of a row of changes that the reader may see; `None` when none of them is.
    async fn field_changes(
        &self,
        actor: &Actor,
        held: &Evaluation,
        maker: &mut LineMaker,
        row: &HistoryEntry,
    ) -> Result<Option<HistoryWhat>, AppError> {
        let mask = held.field_mask();
        let mut fields = Vec::new();
        for change in visible_changes(&|field| mask.can_see(field), &row.changes) {
            let member = Member::of(&change.field);
            let before = self
                .change_value(actor, held, maker, member, change.before.as_ref())
                .await?;
            let after = self
                .change_value(actor, held, maker, member, change.after.as_ref())
                .await?;
            // A change of an issue that the reader may not see leaves nothing to show.
            if member == Member::Issue && before.is_none() && after.is_none() {
                continue;
            }
            fields.push(FieldChangeView {
                field: change.field.clone(),
                before,
                after,
            });
        }
        Ok((!fields.is_empty()).then_some(HistoryWhat::Changes { fields }))
    }

    /// A relation added or taken away, as the issue it is shown on reads it; `None` when the
    /// reader may not see the issue at the other end.
    async fn relation_change(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        issue: IssueId,
        row: &HistoryEntry,
    ) -> Result<Option<HistoryWhat>, AppError> {
        let id_in = |field: &str| -> Option<(String, bool)> {
            let change = row.changes.iter().find(|change| change.field == field)?;
            match (&change.after, &change.before) {
                (Some(value), _) => Some((plain_text(value), true)),
                (None, Some(value)) => Some((plain_text(value), false)),
                (None, None) => None,
            }
        };
        let (Some((from, added)), Some((to, _)), Some((kind, _))) = (
            id_in(relation_member::FROM),
            id_in(relation_member::TO),
            id_in(relation_member::KIND),
        ) else {
            return Ok(None);
        };
        let (direction, other) = if from == issue.to_string() {
            (Direction::Forward, to)
        } else {
            (Direction::Backward, from)
        };
        let Some(other) = other.parse::<IssueId>().ok() else {
            return Ok(None);
        };
        let Some(kind) = self
            .store()
            .relation_kinds()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .find(|found| found.id.to_string() == kind)
        else {
            return Ok(None);
        };
        let Some(other) = self.issues_named(&[other]).await?.into_iter().next() else {
            return Ok(None);
        };
        let Some(line) = self.line_of(actor, maker, &other).await? else {
            return Ok(None);
        };
        Ok(Some(HistoryWhat::Relation {
            added,
            direction,
            name: kind.name(direction).to_owned(),
            issue: line,
        }))
    }

    /// One row of the history as the reader sees it; `None` when there is nothing in it for them.
    async fn history_item(
        &self,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
        maker: &mut LineMaker,
        comments: &BTreeMap<CommentId, Comment>,
        row: &HistoryEntry,
    ) -> Result<Option<HistoryItem>, AppError> {
        let what = match row.kind {
            HistoryKind::Comment => {
                let Some(comment) = CommentId::from_bytes(row.record.to_bytes())
                    .ok()
                    .and_then(|id| comments.get(&id))
                    .filter(|comment| comment.deleted_at.is_none())
                else {
                    return Ok(None);
                };
                let view: CommentView = self
                    .comment_view(actor, held, issue.project, comment)
                    .await?;
                Some(HistoryWhat::Comment { comment: view })
            }
            HistoryKind::Change => {
                if row.record_kind.as_str() == "relation" {
                    self.relation_change(actor, maker, issue.id, row).await?
                } else {
                    self.field_changes(actor, held, maker, row).await?
                }
            }
        };
        let Some(what) = what else {
            return Ok(None);
        };
        let by = match row.by {
            Some(person) => Some(
                self.person_ref(held, PeopleKind::Authors, PeopleContext::Ordinary, person)
                    .await?,
            ),
            None => None,
        };
        Ok(Some(HistoryItem {
            event: row.event,
            at: row.at,
            by,
            what,
        }))
    }
}

/// The history of an issue, newest first, a page at a time, in three tabs: *Everything*,
/// *Comments*, *Changes*. The history is caught up with the events first, so it is never behind
/// what the reader just did.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn history<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HistoryQuery,
) -> Result<Authorized<HistoryPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    catch_up(ctx.store(), ctx.now())
        .await
        .map_err(|error| internal::<S>(&error))?;
    let before = match input.cursor.as_deref() {
        Some(text) => Some(
            text.parse::<i64>()
                .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?,
        ),
        None => None,
    };
    let kind = match input.tab {
        HistoryTab::Everything => None,
        HistoryTab::Comments => Some(HistoryKind::Comment),
        HistoryTab::Changes => Some(HistoryKind::Change),
    };
    let limit = page_len(Some(input.limit));
    let read = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut maker = ctx.line_maker().await?;
    let mut items: Vec<(i64, HistoryItem)> = Vec::new();
    let mut before = before;
    let mut more = false;
    // Rows the reader may not see are skipped, so more rows are read until the page is full or the
    // history ends.
    'pages: loop {
        let rows = ctx
            .store()
            .history_page(issue.id, kind, before, read)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let exhausted = rows.len() < usize::try_from(read).unwrap_or(usize::MAX);
        let comment_ids: Vec<CommentId> = rows
            .iter()
            .filter(|row| row.kind == HistoryKind::Comment)
            .filter_map(|row| CommentId::from_bytes(row.record.to_bytes()).ok())
            .collect();
        let comments: BTreeMap<CommentId, Comment> = ctx
            .store()
            .comments_by_ids(&comment_ids)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|comment| (comment.id, comment))
            .collect();
        let count = rows.len();
        for (at, row) in rows.iter().enumerate() {
            if let Some(item) = ctx
                .history_item(&actor, &held, &issue, &mut maker, &comments, row)
                .await?
            {
                if items.len() == limit {
                    more = true;
                    break 'pages;
                }
                items.push((row.position, item));
            }
            before = Some(row.position);
            if at + 1 == count && exhausted {
                break 'pages;
            }
        }
        if exhausted {
            break;
        }
    }
    let next_cursor = more
        .then(|| items.last().map(|(position, _)| position.to_string()))
        .flatten();
    let place_ok = ctx
        .evaluation_of(&actor, Place::Project(issue.project))
        .await?;
    Ok(place_ok.authorize(HistoryPage {
        items: items.into_iter().map(|(_, item)| item).collect(),
        next_cursor,
    }))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;

    fn change(field: &str) -> FieldChange {
        FieldChange {
            field: field.to_owned(),
            before: None,
            after: Some(json!("x")),
        }
    }

    proptest! {
        /// iss_06_history_leaves_out_changes_to_fields_the_reader_cannot_see: whatever the mask,
        /// every change that is shown is to a field the reader may see (or to a member that is no
        /// field and is open), and every change to a field the reader may see is shown.
        #[test]
        fn iss_06_history_leaves_out_changes_to_fields_the_reader_cannot_see(
            hidden in proptest::sample::subsequence(Field::ALL.to_vec(), 0..=Field::ALL.len()),
            asked in proptest::sample::subsequence(Field::ALL.to_vec(), 0..=Field::ALL.len()),
        ) {
            let can_see = |field: Field| !hidden.contains(&field);
            let mut changes: Vec<FieldChange> =
                asked.iter().map(|field| change(field.code())).collect();
            // Members that are no field of the catalogue: the version, the trash and the key never
            // show; the type and the parent always do.
            for name in [
                bb_core::version::FIELD,
                issue_member::DELETED_AT,
                issue_member::KEY,
                issue_member::ISSUE_TYPE,
                issue_member::PARENT,
            ] {
                changes.push(change(name));
            }
            let shown: Vec<&str> = visible_changes(&can_see, &changes)
                .iter()
                .map(|found| found.field.as_str())
                .collect();
            for field in &asked {
                prop_assert_eq!(
                    shown.contains(&field.code()),
                    !hidden.contains(field),
                    "{}", field.code()
                );
            }
            prop_assert!(!shown.contains(&bb_core::version::FIELD));
            prop_assert!(!shown.contains(&issue_member::DELETED_AT));
            prop_assert!(!shown.contains(&issue_member::KEY));
            prop_assert!(shown.contains(&issue_member::ISSUE_TYPE));
            prop_assert!(shown.contains(&issue_member::PARENT));
        }
    }
}
