//! Reading the history of an issue (ISS-06, SEC-06): the rows that `build.rs` made, shaped for one
//! reader.
//!
//! The one rule of the screen is that **a change to a field the reader may not see is not in the
//! history**: it is left out whole, not shown as hidden, and a row whose changes are all left out
//! is left out too. What a reader may see of a change of an issue is decided by the names of the
//! members of the events (`issues::member`): a field of the catalogue or an own field by the mask of
//! the reader in the project and for the issue type of the issue, the issue type, the parent and
//! the sub-issues as open, everything else never. A value of an own field is shown as the issue page
//! shows it (`Ctx::display_value`); the parent and the sub-issues are a line that the reader may
//! see, or nothing.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::history::{FileEvent, HistoryEntry, HistoryKind, HistoryTab};
use bb_core::id::{AttachmentId, CommentId, FieldId, IssueId, IssueTypeId, UserId};
use bb_core::own_fields::FieldType;
use bb_core::paging::page_len;
use bb_core::people_kinds::PeopleKind;
use bb_core::relation::{Direction, RelationKind};
use bb_policy::{Authorized, Evaluation, PeopleContext, PersonRef};
use serde_json::Value;

use super::names::{KnownNames, leaves_nothing};
use super::types::{
    ChangeValue, FieldChangeView, HistoryItem, HistoryPage, HistoryQuery, HistoryWhat,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::comments::CommentView;
use crate::fields::Definition;
use crate::files::member as file_member;
use crate::issues::member as issue_member;
use crate::member::relation as relation_member;
use crate::projects::ProjectRef;
use crate::relations::LineMaker;
use bb_core::comment::Comment;
use bb_core::files::Attachment;
use bb_core::issue::Issue;

/// How a member of the event of an issue is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Member {
    /// A field of the catalogue: shown when the reader's mask lets them see it.
    Catalogue(Field),
    /// An own field: shown when the reader's mask for the issue type lets them see it.
    Own(FieldId),
    /// The issue type: shown to everyone who sees the issue.
    IssueType,
    /// The project the issue was in or went to: shown where the reader sees that project.
    Project,
    /// The issue it sits under, or an issue that sits under it: shown as a line the reader may
    /// see.
    Issue,
    /// Never shown: the version, the project, the readable ID, the trash.
    Never,
}

impl Member {
    /// The way the member called `name` is shown. The one mapping from the names of the events to
    /// what a reader sees: a field is known by its key (`FieldKey`), the rest is listed here.
    pub(crate) fn of(name: &str) -> Self {
        match FieldKey::from_code(name) {
            Some(FieldKey::BuiltIn(field)) => return Self::Catalogue(field),
            Some(FieldKey::Own(field)) => return Self::Own(field),
            None => {}
        }
        match name {
            issue_member::ISSUE_TYPE => Self::IssueType,
            issue_member::PROJECT => Self::Project,
            issue_member::PARENT | issue_member::SUB_ISSUE => Self::Issue,
            _ => Self::Never,
        }
    }

    /// Whether a reader may see a change of the member, given which fields they may see (their
    /// mask).
    pub(crate) fn may_show(self, can_see: &impl Fn(FieldKey) -> bool) -> bool {
        match self {
            Self::Catalogue(field) => can_see(field.into()),
            Self::Own(field) => can_see(field.into()),
            Self::IssueType | Self::Issue | Self::Project => true,
            Self::Never => false,
        }
    }
}

/// The changes of an event that a reader may see, in the order of the event, given which fields
/// they may see (their mask).
pub(crate) fn visible_changes<'a>(
    can_see: &impl Fn(FieldKey) -> bool,
    changes: &'a [FieldChange],
) -> Vec<&'a FieldChange> {
    changes
        .iter()
        .filter(|change| Member::of(&change.field).may_show(can_see))
        .collect()
}

/// Most rows read for one request, as a multiple of the page size: a reader who may see few of
/// them still gets a cursor back and asks again, so one request never reads a whole long history.
const ROWS_PER_ITEM_MAX: usize = 10;

/// What the rows of one batch need to be shaped, read once for the whole batch.
struct Batch<'a> {
    comments: BTreeMap<CommentId, Comment>,
    issues: BTreeMap<IssueId, Issue>,
    kinds: &'a [RelationKind],
    /// The files the rows of files name, also the ones in the trash.
    files: BTreeMap<AttachmentId, Attachment>,
    /// The own fields, to name them and to read their values.
    fields: Arc<Vec<Definition>>,
}

impl Batch<'_> {
    /// The own field with this ID, if there is one.
    fn field(&self, id: FieldId) -> Option<&Definition> {
        self.fields
            .iter()
            .find(|definition| definition.field.id == id)
    }
}

/// Who reads, and what is built up while the rows of a page are shaped for them: the people shown so
/// far (one cache for the page) and the issue lines.
pub(super) struct Reading<'a> {
    pub(super) actor: &'a Actor,
    held: &'a Evaluation,
    people: HashMap<(PeopleKind, UserId), PersonRef>,
    maker: LineMaker,
    /// The names of types and labels the reader may know, read when first needed.
    pub(super) names: KnownNames,
}

/// What shows changes to a reader (see `Ctx::change_reader`).
pub(crate) struct ChangeReader {
    batch: Batch<'static>,
    people: HashMap<(PeopleKind, UserId), PersonRef>,
    maker: LineMaker,
    names: KnownNames,
}

/// Whether the row is about a relation (the rest are about an issue).
fn is_relation(row: &HistoryEntry) -> bool {
    row.record_kind.as_str() == RecordKind::Relation.code()
}

/// Whether the row is about a file of the issue.
fn is_file(row: &HistoryEntry) -> bool {
    row.record_kind.as_str() == RecordKind::Attachment.code()
}

/// The issues that the rows of a batch name: the parent and the sub-issues of a change, and the
/// two ends of a relation.
fn issues_named_by(rows: &[HistoryEntry]) -> Vec<IssueId> {
    let mut ids = Vec::new();
    for row in rows.iter().filter(|row| row.kind == HistoryKind::Change) {
        for change in &row.changes {
            let names_issues = if is_relation(row) {
                change.field == relation_member::FROM || change.field == relation_member::TO
            } else {
                Member::of(&change.field) == Member::Issue
            };
            if names_issues {
                ids.extend(
                    [&change.before, &change.after]
                        .into_iter()
                        .flatten()
                        .filter_map(|value| value.as_str()?.parse::<IssueId>().ok()),
                );
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
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

    /// One value of a change of an own field as the reader may see it. The one definition of how
    /// an own-field value is shown to a viewer is `display_value` (the issue page and the project
    /// page use it too); this only puts its answer into the words of a change: the names of the
    /// choices, the people, a group by its name and an issue by its readable ID (when the reader
    /// may see them), the values that a screen words itself (yes or no, a length of time, a
    /// number, a day) as they are with the kind of the field. `None` for a long text and for
    /// what the reader may not see.
    async fn own_value(
        &self,
        actor: &Actor,
        held: &Evaluation,
        people: &mut HashMap<(PeopleKind, UserId), PersonRef>,
        definition: &Definition,
        value: &Value,
    ) -> Result<Option<ChangeValue>, AppError> {
        let kind = definition.field.kind;
        if kind.is_long_text() {
            return Ok(None);
        }
        if matches!(kind, FieldType::PickOne | FieldType::PickSeveral) {
            let names = definition.choice_names(value);
            return Ok((!names.is_empty()).then(|| ChangeValue::Names {
                names: names.into_iter().map(str::to_owned).collect(),
            }));
        }
        let shown = self
            .display_value(actor, held, people, kind, value.clone())
            .await?;
        let text = |text: &str| ChangeValue::Text {
            text: text.to_owned(),
        };
        Ok(match (kind, shown) {
            (_, Value::Null) => None,
            (FieldType::Person | FieldType::People, shown) => {
                let people = match shown {
                    Value::Array(_) => serde_json::from_value(shown),
                    one => serde_json::from_value(one).map(|one| vec![one]),
                };
                if let Ok(people) = people {
                    Some(ChangeValue::People { people })
                } else {
                    // A stored value that cannot be read is no value for the reader. The error of
                    // the reader can hold the value itself, which names a person, so it is not
                    // logged (DATA-LOGS).
                    tracing::warn!("a value of a person field in the history cannot be read");
                    None
                }
            }
            (FieldType::Group, Value::Object(found)) => {
                found.get("name").and_then(Value::as_str).map(text)
            }
            (FieldType::Issue, Value::Object(found)) => {
                found.get("readable_id").and_then(Value::as_str).map(text)
            }
            (
                FieldType::YesNo
                | FieldType::Duration
                | FieldType::WholeNumber
                | FieldType::Decimal
                | FieldType::Date
                | FieldType::DateTime,
                value,
            ) => Some(ChangeValue::Typed { kind, value }),
            (_, Value::String(words)) => Some(text(&words)),
            (_, other) => Some(text(&plain_text(&other))),
        })
    }

    /// One value of a change as the reader may see it; `None` when there is no value, the value
    /// is a long text, or it names an issue the reader may not see.
    async fn change_value(
        &self,
        reading: &mut Reading<'_>,
        batch: &Batch<'_>,
        member: Member,
        value: Option<&Value>,
    ) -> Result<Option<ChangeValue>, AppError> {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return Ok(None);
        };
        Ok(match member {
            Member::Catalogue(Field::Description) | Member::Never => None,
            Member::Own(id) => match batch.field(id) {
                Some(definition) => {
                    self.own_value(
                        reading.actor,
                        reading.held,
                        &mut reading.people,
                        definition,
                        value,
                    )
                    .await?
                }
                None => None,
            },
            Member::Catalogue(Field::Tags) => self.tags_value(reading, value).await?,
            Member::Catalogue(Field::Labels) => self.labels_value(reading, value).await?,
            Member::Catalogue(field) => match field.person_kind() {
                Some(kind) => Some(self.people_in(reading.held, kind, value).await?),
                None => Some(ChangeValue::Text {
                    text: plain_text(value),
                }),
            },
            Member::IssueType => self.type_value(reading, value).await?,
            Member::Project => match value
                .as_str()
                .and_then(|text| text.parse::<bb_core::id::ProjectId>().ok())
            {
                Some(id) => self
                    .project_seen(reading.actor, id)
                    .await?
                    .map(|project| ChangeValue::Project { project }),
                None => None,
            },
            Member::Issue => {
                let Some(id) = value.as_str().and_then(|text| text.parse::<IssueId>().ok()) else {
                    return Ok(None);
                };
                match batch.issues.get(&id) {
                    Some(issue) => self
                        .line_of(reading.actor, &mut reading.maker, issue)
                        .await?
                        .map(|issue| ChangeValue::Issue { issue }),
                    None => None,
                }
            }
        })
    }

    /// The fields of a change that the reader may see; `None` when none of them is.
    async fn field_changes(
        &self,
        reading: &mut Reading<'_>,
        issue_type: IssueTypeId,
        batch: &Batch<'_>,
        changes: &[FieldChange],
    ) -> Result<Option<HistoryWhat>, AppError> {
        // The mask for the type of the issue now, built-in and own fields alike, so a field the
        // reader may not see on this type is in no row.
        let mask = reading.held.field_mask_for(Some(issue_type));
        let mut fields = Vec::new();
        for change in visible_changes(&|field| mask.can_see(field), changes) {
            let member = Member::of(&change.field);
            let before = self
                .change_value(reading, batch, member, change.before.as_ref())
                .await?;
            let after = self
                .change_value(reading, batch, member, change.after.as_ref())
                .await?;
            // A change of an issue or project that the reader may not see leaves nothing to show,
            // also in an own field of the kind *Issue* (M3-F79: as for the parent).
            let points_at_a_record = match member {
                Member::Issue | Member::Project => true,
                Member::Own(id) => batch
                    .field(id)
                    .is_some_and(|found| found.field.kind == FieldType::Issue),
                _ => false,
            };
            if points_at_a_record && before.is_none() && after.is_none() {
                continue;
            }
            if leaves_nothing(member, before.as_ref(), after.as_ref()) {
                continue;
            }
            let name = match member {
                Member::Own(id) => batch.field(id).map(|found| found.field.name.clone()),
                _ => None,
            };
            fields.push(FieldChangeView {
                field: change.field.clone(),
                name,
                before,
                after,
            });
        }
        Ok((!fields.is_empty()).then_some(HistoryWhat::Changes { fields }))
    }

    /// What reads changes that have not happened yet, as the history will show them: made once for
    /// a preview, so the definitions of the fields and the people are read once for all of it.
    pub(crate) async fn change_reader(&self) -> Result<ChangeReader, AppError> {
        Ok(ChangeReader {
            batch: self.batch_of(&[], &[]).await?,
            people: HashMap::new(),
            maker: self.line_maker().await?,
            names: KnownNames::default(),
        })
    }

    /// Changes that have not happened yet, shown as the history will show them to the actor: the
    /// preview of a change to many issues says what will change in the words of the history.
    pub(crate) async fn changes_as_read(
        &self,
        reader: &mut ChangeReader,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
        changes: &[FieldChange],
    ) -> Result<Vec<FieldChangeView>, AppError> {
        let mut reading = Reading {
            actor,
            held,
            people: std::mem::take(&mut reader.people),
            maker: std::mem::take(&mut reader.maker),
            names: std::mem::take(&mut reader.names),
        };
        let read = self
            .field_changes(&mut reading, issue.issue_type, &reader.batch, changes)
            .await;
        reader.people = reading.people;
        reader.maker = reading.maker;
        reader.names = reading.names;
        Ok(match read? {
            Some(HistoryWhat::Changes { fields }) => fields,
            _ => Vec::new(),
        })
    }

    /// A relation added or taken away, as the issue it is shown on reads it; `None` when the
    /// reader may not see the issue at the other end.
    async fn relation_change(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        issue: IssueId,
        batch: &Batch<'_>,
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
        let Some(kind) = batch
            .kinds
            .iter()
            .find(|found| found.id.to_string() == kind)
        else {
            return Ok(None);
        };
        let Some(other) = batch.issues.get(&other) else {
            return Ok(None);
        };
        let Some(line) = self.line_of(actor, maker, other).await? else {
            return Ok(None);
        };
        Ok(Some(HistoryWhat::Relation {
            added,
            direction,
            name: kind.name(direction).to_owned(),
            issue: line,
        }))
    }

    /// A file of the issue that was added, put into the trash or brought back; `None` for a file of
    /// a comment (it belongs to the comment). The name is the one the file has now, from the batch;
    /// a file that is gone for good keeps its row, as "a file" with no name.
    fn file_change(batch: &Batch<'_>, row: &HistoryEntry) -> Option<HistoryWhat> {
        let file = AttachmentId::from_bytes(row.record.to_bytes())
            .ok()
            .and_then(|id| batch.files.get(&id));
        if file.is_some_and(|file| file.comment.is_some()) {
            return None;
        }
        let deleted = row
            .changes
            .iter()
            .find(|change| change.field == file_member::DELETED_AT);
        let event = match deleted {
            None => FileEvent::Added,
            Some(change) if change.after.is_some() => FileEvent::Deleted,
            Some(_) => FileEvent::Restored,
        };
        Some(HistoryWhat::File {
            event,
            name: file.map(|file| file.name.clone()),
        })
    }

    /// The project, when the reader sees it.
    async fn project_seen(
        &self,
        actor: &Actor,
        id: bb_core::id::ProjectId,
    ) -> Result<Option<ProjectRef>, AppError> {
        let Some(found) = self
            .store()
            .project(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        let held = self
            .evaluation_of(actor, bb_policy::Place::Project(id))
            .await?;
        Ok(held.sees_project().then(|| ProjectRef::from(&found)))
    }

    /// The comments and the issues that the rows of a batch name, read once for the batch.
    async fn batch_of<'k>(
        &self,
        rows: &[HistoryEntry],
        kinds: &'k [RelationKind],
    ) -> Result<Batch<'k>, AppError> {
        let comment_ids: Vec<CommentId> = rows
            .iter()
            .filter(|row| row.kind == HistoryKind::Comment)
            .filter_map(|row| CommentId::from_bytes(row.record.to_bytes()).ok())
            .collect();
        let comments = self
            .store()
            .comments_by_ids(&comment_ids)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|comment| (comment.id, comment))
            .collect();
        let issues = self
            .issues_named(&issues_named_by(rows))
            .await?
            .into_iter()
            .map(|issue| (issue.id, issue))
            .collect();
        let file_ids: Vec<AttachmentId> = rows
            .iter()
            .filter(|row| is_file(row))
            .filter_map(|row| AttachmentId::from_bytes(row.record.to_bytes()).ok())
            .collect();
        let files = self
            .store()
            .attachments_by_ids(&file_ids)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|file| (file.id, file))
            .collect();
        Ok(Batch {
            comments,
            issues,
            kinds,
            files,
            fields: self.field_catalogue().await?,
        })
    }

    /// One row of the history as the reader sees it; `None` when there is nothing in it for them.
    async fn history_item(
        &self,
        reading: &mut Reading<'_>,
        issue: &Issue,
        batch: &Batch<'_>,
        row: &HistoryEntry,
    ) -> Result<Option<HistoryItem>, AppError> {
        let what = match row.kind {
            HistoryKind::Comment => {
                let Some(comment) = CommentId::from_bytes(row.record.to_bytes())
                    .ok()
                    .and_then(|id| batch.comments.get(&id))
                    .filter(|comment| comment.deleted_at.is_none())
                else {
                    return Ok(None);
                };
                let view: CommentView = self
                    .comment_view(reading.actor, reading.held, issue, comment)
                    .await?;
                Some(HistoryWhat::Comment { comment: view })
            }
            HistoryKind::Change => {
                if is_file(row) {
                    Self::file_change(batch, row)
                } else if is_relation(row) {
                    self.relation_change(reading.actor, &mut reading.maker, issue.id, batch, row)
                        .await?
                } else {
                    self.field_changes(reading, issue.issue_type, batch, &row.changes)
                        .await?
                }
            }
        };
        let Some(what) = what else {
            return Ok(None);
        };
        let by = match row.by {
            Some(person) => Some(
                self.person_ref(
                    reading.held,
                    PeopleKind::Authors,
                    PeopleContext::Ordinary,
                    person,
                )
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
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn history<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HistoryQuery,
) -> Result<Authorized<HistoryPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    ctx.catch_up_history().await?;
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
    // The people of the values of the page are resolved once for every row of it.
    let mut reading = Reading {
        actor: &actor,
        held: &held,
        people: HashMap::new(),
        maker: ctx.line_maker().await?,
        names: KnownNames::default(),
    };
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut items: Vec<(i64, HistoryItem)> = Vec::new();
    let mut resume = None;
    let mut before = before;
    let mut rows_read = 0_usize;
    // Rows the reader may not see are skipped, so more rows are read until the page is full, the
    // history ends or the most rows of one request are read; then the cursor says where to go on.
    'pages: loop {
        let rows = ctx
            .store()
            .history_page(issue.id, kind, before, read)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let exhausted = rows.len() < usize::try_from(read).unwrap_or(usize::MAX);
        let batch = ctx.batch_of(&rows, &kinds).await?;
        for row in &rows {
            if let Some(item) = ctx.history_item(&mut reading, &issue, &batch, row).await? {
                if items.len() == limit {
                    resume = items.last().map(|(position, _)| *position);
                    break 'pages;
                }
                items.push((row.position, item));
            }
            before = Some(row.position);
            rows_read += 1;
            if rows_read >= limit.saturating_mul(ROWS_PER_ITEM_MAX) {
                resume = before;
                break 'pages;
            }
        }
        if exhausted {
            break;
        }
    }
    Ok(held.authorize(HistoryPage {
        items: items.into_iter().map(|(_, item)| item).collect(),
        next_cursor: resume.map(|position| position.to_string()),
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
            let can_see = |field: FieldKey| match field {
                FieldKey::BuiltIn(field) => !hidden.contains(&field),
                FieldKey::Own(_) => false,
            };
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
