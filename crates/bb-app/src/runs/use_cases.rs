//! The use cases of runs (AI-38): reading a run and its changes as the reader may see them, the
//! preview of *Undo the run*, and the undo itself. Only the person a run belongs to reads it or undoes
//! it; for anybody else it is not there. (An attempt is also undone by the reviewer through
//! *Throw away*, which agent work adds.)

use std::collections::HashMap;

use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{EventId, IssueId, RunId};
use bb_core::many_changes::ManyChangeStatus;
use bb_core::run::{RunActor, RunKind, UndoOutcome, UndoSkip, UndoSkipKind, UndoVersion};
use bb_policy::{Authorized, Masked, Place};
use jiff::Timestamp;
use serde::Serialize;

use super::{RunStore, RunsOf};
use crate::auth::{AppError, AuthStore, Ctx, by as by_person, entity, internal, set_to};
use crate::history::FieldChangeView;
use crate::relations::IssueLine;

/// How many changes of a run one page holds when the request does not say.
const PAGE_DEFAULT: u64 = 50;
/// The most changes one page holds.
const PAGE_MAX: u64 = 100;

/// Read a run.
pub struct RunQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The run.
    pub run: RunId,
}

/// Read the changes of a run, a page at a time.
pub struct RunChangesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The run.
    pub run: RunId,
    /// The event the page starts after (the last of the page before).
    pub after: Option<EventId>,
    /// How many changes at most.
    pub limit: Option<u64>,
}

/// Undo a run with the version of the preview the person looked at.
pub struct UndoRun {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The run.
    pub run: RunId,
    /// The version of the preview.
    pub version: UndoVersion,
}

/// A run, as its person sees it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunView {
    /// The run.
    pub id: RunId,
    /// What it is of.
    pub kind: RunKind,
    /// Where it is.
    pub status: ManyChangeStatus,
    /// The name of the AI app, for a session of an app.
    pub app: Option<String>,
    /// When it began.
    pub started_at: Timestamp,
    /// When it ended.
    pub ended_at: Option<Timestamp>,
}

impl Masked for RunView {}

/// One change of a run: an issue and what changed on it, in the words of the history.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunChangeView {
    /// The event.
    pub event: EventId,
    /// When it was made.
    pub at: Timestamp,
    /// The issue.
    pub issue: IssueLine,
    /// What changed, leaving out what the reader may not see.
    pub changes: Vec<FieldChangeView>,
}

/// A page of the changes of a run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunChangesPage {
    /// The run.
    pub run: RunView,
    /// The changes the reader may see, oldest first.
    pub items: Vec<RunChangeView>,
    /// Where the next page starts after, when there is more.
    pub next: Option<EventId>,
}

impl Masked for RunChangesPage {}

/// A field *Undo the run* leaves as it is.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UndoSkipView {
    /// The issue, when the reader may see it.
    pub issue: Option<IssueLine>,
    /// The field, in the words of the history.
    pub field: String,
    /// Why it stays.
    pub kind: UndoSkipKind,
    /// Somebody changed it afterwards and it was the reader.
    pub by_you: bool,
    /// When it was changed afterwards.
    pub at: Option<Timestamp>,
}

/// What *Undo the run* would do or did.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UndoView {
    /// The version to apply with.
    pub version: UndoVersion,
    /// How many field changes go back (or went back).
    pub goes_back: u64,
    /// The first fields that stay.
    pub skips: Vec<UndoSkipView>,
    /// How many fields stay.
    pub skipped: u64,
    /// For an apply: the preview was out of date, nothing was written and this is the newer one.
    pub changed_since_preview: bool,
}

impl Masked for UndoView {}

impl<S: AuthStore> Ctx<'_, S> {
    /// The run, when it is the actor's own; for anybody else it is not there.
    async fn run_of(&self, actor: &crate::access::Actor, run: RunId) -> Result<RunView, AppError> {
        let found = self.own_run(actor, run).await?;
        Ok(RunView {
            id: found.id,
            kind: found.kind,
            status: found.status,
            app: found.app,
            started_at: found.created_at,
            ended_at: found.finished_at,
        })
    }

    /// The names of the skips as the actor reads them.
    async fn skip_views(
        &self,
        actor: &crate::access::Actor,
        skips: &[UndoSkip],
    ) -> Result<Vec<UndoSkipView>, AppError> {
        let mut maker = self.line_maker().await?;
        let mut views = Vec::new();
        for skip in skips {
            let issue = match super::issue_of(&skip.entity) {
                Some(id) => match self.seen_issue_by_id(actor, id).await? {
                    Some((issue, _)) => self.line_of(actor, &mut maker, &issue).await?,
                    None => None,
                },
                None => None,
            };
            views.push(UndoSkipView {
                issue,
                field: skip.field.clone(),
                kind: skip.kind,
                by_you: skip.by.and_then(EventActor::person) == Some(actor.person.id),
                at: skip.at,
            });
        }
        Ok(views)
    }
}

/// Shows a run to the person it belongs to.
///
/// # Errors
/// `resource.not-found` for a run that is not there or that is somebody else's.
pub async fn get_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RunQuery,
) -> Result<Authorized<RunView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(ctx.run_of(&actor, input.run).await?))
}

/// The changes of a run, oldest first, a page at a time: only issues the reader may see, and on
/// them only the fields they may see.
///
/// # Errors
/// `resource.not-found` for a run that is not there or that is somebody else's.
pub async fn run_changes<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RunChangesQuery,
) -> Result<Authorized<RunChangesPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let run = ctx.run_of(&actor, input.run).await?;
    let limit = input.limit.unwrap_or(PAGE_DEFAULT).clamp(1, PAGE_MAX);
    let events = ctx
        .store()
        .run_events(input.run, input.after, limit)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let next = (events.len() as u64 == limit)
        .then(|| events.last().map(|(_, event)| event.id))
        .flatten();
    let mut reader = ctx.change_reader().await?;
    let mut maker = ctx.line_maker().await?;
    let mut seen: HashMap<IssueId, Option<(bb_core::issue::Issue, _)>> = HashMap::new();
    let mut items = Vec::new();
    for (_, event) in events {
        let Some(id) = super::issue_of(&event.entity) else {
            continue;
        };
        if let std::collections::hash_map::Entry::Vacant(place) = seen.entry(id) {
            place.insert(ctx.seen_issue_by_id(&actor, id).await?);
        }
        let Some(Some((issue, in_project))) = seen.get(&id) else {
            continue;
        };
        let Some(line) = ctx.line_of(&actor, &mut maker, issue).await? else {
            continue;
        };
        let changes = ctx
            .changes_as_read(&mut reader, &actor, in_project, issue, &event.changes)
            .await?;
        if changes.is_empty() {
            continue;
        }
        items.push(RunChangeView {
            event: event.id,
            at: event.time,
            issue: line,
            changes,
        });
    }
    Ok(held.authorize(RunChangesPage { run, items, next }))
}

/// What *Undo the run* would do now: how many field changes go back and which stay, with the
/// version to apply with.
///
/// # Errors
/// `resource.not-found` for a run that is not there or that is somebody else's.
pub async fn preview_undo_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RunQuery,
) -> Result<Authorized<UndoView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let preview = RunsOf::new(ctx, &actor, &input.session_token)
        .preview_undo(input.run)
        .await?;
    let skips = ctx.skip_views(&actor, &preview.skips).await?;
    Ok(held.authorize(UndoView {
        version: preview.version,
        goes_back: preview.goes_back,
        skips,
        skipped: preview.skipped,
        changed_since_preview: false,
    }))
}

/// Undoes a run in one step, as the person who undoes it: each change is judged again, and a field
/// that somebody changed afterwards stays and is named. When anything changed since the preview,
/// nothing is written and the newer preview comes back.
///
/// # Errors
/// `resource.not-found` for a run that is not there or that is somebody else's.
pub async fn undo_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UndoRun,
) -> Result<Authorized<UndoView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    // A key that may only look undoes nothing, and an agent never undoes (it is not the person).
    actor.may_change_own()?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let by = RunActor {
        person: actor.person.id,
        app: None,
        agent: None,
    };
    let outcome = RunsOf::new(ctx, &actor, &input.session_token)
        .undo_run(input.run, &input.version, &by, ctx.now())
        .await?;
    let view = match outcome {
        UndoOutcome::Undone(done) => {
            // The undo is a fact of the run too: the run says it was undone and how far. An undo
            // that put nothing back (another one was first) is no fact.
            if done.undone > 0 {
                let event = ctx.event(
                    by_person(actor.person.id),
                    entity(RecordKind::ChangeRun, input.run),
                    vec![
                        set_to("undone", serde_json::json!(done.undone)),
                        set_to("kept", serde_json::json!(done.skipped)),
                    ],
                )?;
                ctx.store()
                    .record_change(&event, |_transaction| Box::pin(async { Ok(()) }))
                    .await
                    .map_err(|error| internal::<S>(&error))?;
            }
            UndoView {
                version: input.version,
                goes_back: done.undone,
                skips: ctx.skip_views(&actor, &done.skips).await?,
                skipped: done.skipped,
                changed_since_preview: false,
            }
        }
        UndoOutcome::Changed(preview) => UndoView {
            version: preview.version,
            goes_back: preview.goes_back,
            skips: ctx.skip_views(&actor, &preview.skips).await?,
            skipped: preview.skipped,
            changed_since_preview: true,
        },
    };
    Ok(held.authorize(view))
}
