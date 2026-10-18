//! Reading the notifications of a person and marking them as read (NTF-01, SEC-06, ADR-0033 §2,
//! §4).
//!
//! The row was checked when it was built; it is checked again here, every time it is shown: a
//! person who lost access to the issue since gets nothing of it (the row is left out whole, and
//! is not counted), the fields are read with the mask the person has now, and the people are
//! shown as the kinds of people information say for this person now.

use std::collections::{HashMap, HashSet};

use bb_core::ErrorCode;
use bb_core::comment::Comment;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{CommentId, EntityId, IssueId, NotificationId, ProjectId, UserId};
use bb_core::issue::Issue;
use bb_core::many_changes::ManyChangeStatus;
use bb_core::notification::{
    AT_ONCE_MAX, LOOKED_AT_MAX, Notification, NotificationFilter, NotificationKind,
};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Evaluation, PeopleContext, PersonRef, Place};
use serde_json::json;

use super::plan::{edit_note_of, renewal_failed, run_status_of};
use super::types::{
    NotificationItem, NotificationPage, NotificationWhat, NotificationsQuery, ReadNotifications,
    Unread, UnreadQuery, Which,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity, internal};
use crate::history::ChangeReader;
use crate::member::notifications as member;
use crate::relations::LineMaker;

/// The comment a row is about, if it is about one.
fn comment_of(row: &Notification) -> Option<CommentId> {
    row.record
        .and_then(|record| CommentId::from_bytes(record.to_bytes()).ok())
}

/// What a page of notifications is shaped with: the person, the issue lines and the people shown
/// so far, which are looked up once for the page.
pub(super) struct Reading<'a> {
    actor: &'a Actor,
    maker: LineMaker,
    changes: ChangeReader,
    /// The people shown so far, by the kind they were shown as.
    people: HashMap<UserId, PersonRef>,
    /// The issues and comments the rows of a batch name, read in one statement each (PERF-05).
    issues: HashMap<IssueId, Issue>,
    comments: HashMap<CommentId, Comment>,
    /// Everything that was looked for, found or not: an issue or a comment that is gone (purged) is
    /// not looked for again by every row that names it.
    looked_for: HashSet<EntityId>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What shapes notifications for `actor`: the issue types, the fields and the people are read
    /// once for all of them.
    pub(super) async fn reading_for<'a>(&self, actor: &'a Actor) -> Result<Reading<'a>, AppError> {
        Ok(Reading {
            actor,
            maker: self.line_maker().await?,
            changes: self.change_reader().await?,
            people: HashMap::new(),
            issues: HashMap::new(),
            comments: HashMap::new(),
            looked_for: HashSet::new(),
        })
    }

    /// Reads the issues and the comments that these rows name, one statement for each kind, so a
    /// page costs the same however many rows it has. A row that names something not read here is
    /// read on its own by `notification_item`.
    pub(super) async fn read_what_rows_name(
        &self,
        reading: &mut Reading<'_>,
        rows: &[Notification],
    ) -> Result<(), AppError> {
        let mut issues: Vec<IssueId> = rows
            .iter()
            .filter_map(|row| row.issue)
            .filter(|id| !reading.looked_for.contains(&EntityId::from(*id)))
            .collect();
        issues.sort();
        issues.dedup();
        let mut comments: Vec<CommentId> = rows
            .iter()
            .filter_map(comment_of)
            .filter(|id| !reading.looked_for.contains(&EntityId::from(*id)))
            .collect();
        comments.sort();
        comments.dedup();
        reading
            .looked_for
            .extend(issues.iter().copied().map(EntityId::from));
        reading
            .looked_for
            .extend(comments.iter().copied().map(EntityId::from));
        if !issues.is_empty() {
            for issue in self.issues_named(&issues).await? {
                reading.issues.insert(issue.id, issue);
            }
        }
        if !comments.is_empty() {
            for comment in self
                .store()
                .comments_by_ids(&comments)
                .await
                .map_err(|error| internal::<S>(&error))?
            {
                reading.comments.insert(comment.id, comment);
            }
        }
        Ok(())
    }

    /// One notification as the person reads it now; `None` when there is nothing in it for them
    /// any more: they may not see the issue, the comment is gone, or every field is hidden.
    pub(super) async fn notification_item(
        &self,
        reading: &mut Reading<'_>,
        row: &Notification,
    ) -> Result<Option<NotificationItem>, AppError> {
        let actor = reading.actor;
        // What a batch did not read first is read for this row alone.
        self.read_what_rows_name(reading, std::slice::from_ref(row))
            .await?;
        let (issue_line, held) = match row.issue {
            Some(id) => {
                let Some(issue) = reading.issues.get(&id).cloned() else {
                    return Ok(None);
                };
                let Some(line) = self.line_of(actor, &mut reading.maker, &issue).await? else {
                    return Ok(None);
                };
                let held = self
                    .evaluation_of(actor, Place::Project(issue.project))
                    .await?;
                (Some((line, issue)), held)
            }
            None => (None, self.evaluation_of(actor, Place::Organization).await?),
        };
        let Some(what) = self
            .notification_what(reading, &held, issue_line.as_ref(), row)
            .await?
        else {
            return Ok(None);
        };
        let by = match row.by {
            Some(person) => Some(self.notification_person(reading, &held, person).await?),
            None => None,
        };
        Ok(Some(NotificationItem {
            id: row.id,
            kind: row.kind,
            at: row.at,
            read: row.read_at.is_some(),
            comment: comment_of(row).filter(|_| matches!(what, NotificationWhat::Comment { .. })),
            issue: issue_line.map(|(line, _)| line),
            by,
            what,
        }))
    }

    /// Who did it, as the kind *Authors* says for the reader.
    async fn notification_person(
        &self,
        reading: &mut Reading<'_>,
        held: &Evaluation,
        person: UserId,
    ) -> Result<PersonRef, AppError> {
        if let Some(known) = reading.people.get(&person) {
            return Ok(known.clone());
        }
        let shown = self
            .person_ref(held, PeopleKind::Authors, PeopleContext::Ordinary, person)
            .await?;
        reading.people.insert(person, shown.clone());
        Ok(shown)
    }

    /// What the row says, read now.
    async fn notification_what(
        &self,
        reading: &mut Reading<'_>,
        held: &Evaluation,
        issue: Option<&(crate::relations::IssueLine, Issue)>,
        row: &Notification,
    ) -> Result<Option<NotificationWhat>, AppError> {
        let actor = reading.actor;
        match row.kind {
            NotificationKind::EditNotice => {
                Ok(
                    edit_note_of(&row.changes).map(|note| NotificationWhat::EditNotice {
                        notice: note.notice,
                        part: note.part,
                    }),
                )
            }
            NotificationKind::RunEnded if renewal_failed(&row.changes) => {
                let Some(project) = row
                    .record
                    .and_then(|record| ProjectId::from_bytes(record.to_bytes()).ok())
                else {
                    return Ok(None);
                };
                Ok(Some(NotificationWhat::IssueIdsFailed {
                    project: self.project_label(actor, project).await?,
                }))
            }
            NotificationKind::RunEnded => Ok(row.record.map(|run| NotificationWhat::RunEnded {
                run,
                // Only a run that says it is done ended well: one that failed, one that was
                // stopped and work that gave up are not.
                done: run_status_of(&row.changes) == Some(ManyChangeStatus::Done),
            })),
            NotificationKind::Mentioned
            | NotificationKind::Assigned
            | NotificationKind::FollowedActivity
            | NotificationKind::FollowedFields => {
                let Some((_, issue)) = issue else {
                    return Ok(None);
                };
                if let Some(comment) = comment_of(row) {
                    let found = reading
                        .comments
                        .get(&comment)
                        .filter(|found| found.deleted_at.is_none());
                    let Some(found) = found else {
                        return Ok(None);
                    };
                    let view = self.comment_view(actor, held, issue, found).await?;
                    return Ok(Some(NotificationWhat::Comment { comment: view }));
                }
                // The fields as the person may see them now: what they lost sight of is left out.
                let fields = self
                    .changes_as_read(&mut reading.changes, actor, held, issue, &row.changes)
                    .await?;
                if fields.is_empty() {
                    // An assignment is still news without the fields; a change of fields that the
                    // person cannot see any more is not.
                    return Ok(
                        (row.kind == NotificationKind::Assigned).then_some(NotificationWhat::Plain)
                    );
                }
                Ok(Some(NotificationWhat::Changes { fields }))
            }
        }
    }

    /// How many unread notifications the person has that they may still see, at most
    /// `AT_ONCE_MAX`: no total ever counts what the person cannot see (ARCH-NO-COUNTERS).
    async fn count_unread(&self, reading: &mut Reading<'_>) -> Result<usize, AppError> {
        let rows = self
            .store()
            .notifications_of(
                reading.actor.person.id,
                NotificationFilter::Unread,
                None,
                u64::try_from(AT_ONCE_MAX).unwrap_or(u64::MAX),
            )
            .await
            .map_err(|error| internal::<S>(&error))?;
        self.read_what_rows_name(reading, &rows).await?;
        let mut count = 0;
        for row in &rows {
            if self.notification_item(reading, row).await?.is_some() {
                count += 1;
            }
        }
        Ok(count)
    }
}

/// The notifications of the person, newest first, a page at a time. The reader is caught up with
/// the events first, so the list is never behind what just happened.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn notifications<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: NotificationsQuery,
) -> Result<Authorized<NotificationPage>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    ctx.catch_up_notifications().await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let mut before = match input.cursor.as_deref() {
        Some(text) => Some(
            text.parse::<NotificationId>()
                .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?,
        ),
        None => None,
    };
    let len = page_len(Some(input.limit));
    let mut reading = ctx.reading_for(&actor).await?;
    let mut items: Vec<NotificationItem> = Vec::new();
    let mut more = false;
    // Rows the person may not see any more are skipped, so more rows are read until the page is
    // full, there are no more, or the request has looked at as many as one may (then the page is
    // shorter and the next one goes on from the last row looked at).
    let mut looked_at = 0_usize;
    let mut resume = None;
    'pages: loop {
        let rows = ctx
            .store()
            .notifications_of(actor.person.id, input.filter, before, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let exhausted = rows.len() < usize::try_from(read_len(len)).unwrap_or(usize::MAX);
        ctx.read_what_rows_name(&mut reading, &rows).await?;
        for row in &rows {
            if looked_at == LOOKED_AT_MAX {
                more = true;
                resume = before;
                break 'pages;
            }
            looked_at += 1;
            if let Some(item) = ctx.notification_item(&mut reading, row).await? {
                if items.len() == len {
                    more = true;
                    break 'pages;
                }
                items.push(item);
            }
            before = Some(row.id);
        }
        if exhausted {
            break;
        }
    }
    let cut = Cut { items, more };
    let next_cursor = match resume {
        Some(last) => Some(last.to_string()),
        None => cut.next_cursor(|item| item.id.to_string()),
    };
    Ok(held.authorize(NotificationPage {
        items: cut.items,
        next_cursor,
    }))
}

/// How many unread notifications the person has that they may still see, at most
/// `AT_ONCE_MAX`.
///
/// # Errors
/// None besides a failing store.
pub async fn unread_notifications<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UnreadQuery,
) -> Result<Authorized<Unread>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    ctx.catch_up_notifications().await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let mut reading = ctx.reading_for(&actor).await?;
    let unread = ctx.count_unread(&mut reading).await?;
    Ok(held.authorize(Unread { unread }))
}

/// Marks notifications of the person as read (*Mark as read*): the ones named, or every unread
/// one up to the newest the person has seen (*Mark all as read*). Marking one that is read already changes nothing.
///
/// # Errors
/// `request.invalid` when no notification is named, or too many.
pub async fn read_notifications<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReadNotifications,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    if let Which::These(ids) = &input.which
        && (ids.is_empty() || ids.len() > AT_ONCE_MAX)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let person = actor.person.id;
    let named = match &input.which {
        Which::These(ids) => json!(ids.iter().map(ToString::to_string).collect::<Vec<_>>()),
        Which::UpTo(id) => json!({ "up_to": id.to_string() }),
    };
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![FieldChange {
            field: member::READ.to_owned(),
            before: None,
            after: Some(named),
        }],
    )?;
    let (which, now) = (input.which, ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                match which {
                    Which::These(ids) => {
                        S::mark_notifications_read(transaction, person, &ids, now).await
                    }
                    Which::UpTo(id) => {
                        S::mark_notifications_read_up_to(transaction, person, id, now).await
                    }
                }
                .map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(Done)
}
