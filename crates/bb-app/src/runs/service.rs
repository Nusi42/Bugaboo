//! The real `RunStore` (AI-38, ADR-0029 decision 7): runs kept on `change_run`, their changes read
//! from the events that carry them, and *Undo the run* done through the use case of a single
//! change, as the person who undoes it, so each change is judged again.
//!
//! The preview and the undo share the one evaluation of `weigh.rs`. A large run is undone the
//! same way as a change to many issues: issue by issue, each its own change with its own event.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::event::{Actor as EventActor, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::id::{EventId, IssueId, RunId};
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus, SkipReason};
use bb_core::run::{
    RunActor, RunChange, RunEnd, RunKind, RunSummary, UndoOutcome, UndoPreview, UndoResult,
    UndoSkip, UndoSkipKind, UndoVersion,
};
use bb_policy::FieldMask;
use jiff::Timestamp;
use serde_json::json;

use super::RunStore;
use super::restore::{issue_change, restorable};
use super::weigh::{Evaluation, Net, NetOfRun, weigh};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, entity, internal, set_to};
use crate::history::visible_changes;
use crate::issues::change_issue;
use crate::many_changes::skip_of;

/// How many events of a run are read at once.
const PAGE: u64 = 500;
/// How many skipped fields a preview or an answer names; the number says how many there are.
pub(super) const SKIPS_NAMED_MAX: usize = 50;

/// The runs of one person, kept and undone through the context of a request.
pub struct RunsOf<'a, 'c, S: AuthStore> {
    ctx: &'a Ctx<'c, S>,
    actor: &'a Actor,
    token: &'a str,
    /// The error that stopped the last undo half way, for the use case to record what went back
    /// first and then give the error up.
    interrupted: std::sync::Mutex<Option<AppError>>,
}

impl<'a, 'c, S: AuthStore> RunsOf<'a, 'c, S> {
    /// The runs, for the person who is `actor`; `token` is the secret of the session they undo with.
    pub(crate) const fn new(ctx: &'a Ctx<'c, S>, actor: &'a Actor, token: &'a str) -> Self {
        Self {
            ctx,
            actor,
            token,
            interrupted: std::sync::Mutex::new(None),
        }
    }

    /// Everything the run did, field by field.
    async fn net_of(&self, run: RunId) -> Result<NetOfRun, AppError> {
        let mut net = NetOfRun::default();
        let mut after: Option<EventId> = None;
        loop {
            let page = self
                .ctx
                .store()
                .run_events(run, after, PAGE)
                .await
                .map_err(|error| internal::<S>(&error))?;
            for (place, event) in &page {
                net.add(*place, event);
            }
            match page.last() {
                Some((_, last)) if page.len() as u64 == PAGE => after = Some(last.id),
                _ => break,
            }
        }
        Ok(net)
    }

    /// What the run did against the events outside it that came after: read after the versions of
    /// the issues, so a change between the two is a conflict and is never written over.
    async fn evaluation(&self, run: RunId, net: &NetOfRun) -> Result<Evaluation, AppError> {
        let mut later = Vec::new();
        if let Some(first) = net.first_event() {
            for (kind, ids) in net.records() {
                let kind = EntityKind::new(&kind)
                    .map_err(|error| AppError::Internal(error.to_string()))?;
                later.extend(
                    self.ctx
                        .store()
                        .events_outside_run_after(run, first, &kind, &ids)
                        .await
                        .map_err(|error| internal::<S>(&error))?,
                );
            }
            later.sort_by_key(|(place, _)| *place);
        }
        Ok(weigh(net, &later))
    }

    /// What the undo does, counting and naming only what the reader may see (PRIV-13): the changes
    /// to issues they cannot see are left alone and are in no number, and neither are the fields
    /// of an issue that they may not read.
    pub(crate) async fn plan_for_reader(&self, run: RunId) -> Result<Plan, AppError> {
        let net = self.net_of(run).await?;
        // The issues first, with the version each has now, and only then the events that came
        // after the run: a change made in between shows in the events and conflicts at the write.
        let mut masks: BTreeMap<IssueId, Option<FieldMask>> = BTreeMap::new();
        let mut versions: BTreeMap<IssueId, i64> = BTreeMap::new();
        let mut candidates: Vec<IssueId> = net
            .changed()
            .filter_map(|each| super::issue_of(&each.entity))
            .collect();
        candidates.extend(comment_issues_of(net.changed()).into_values());
        for issue in candidates {
            if let std::collections::btree_map::Entry::Vacant(slot) = masks.entry(issue) {
                let found = self.ctx.seen_issue_by_id(self.actor, issue).await?;
                if let Some((issue, _)) = &found {
                    versions.insert(issue.id, issue.version);
                }
                slot.insert(found.map(|(issue, held)| held.field_mask_for(Some(issue.issue_type))));
            }
        }
        let mut plan = Self::plan(self.evaluation(run, &net).await?, &net);
        plan.versions = versions;
        let readable = |issue: &IssueId, field: &str, kind: UndoSkipKind| {
            masks
                .get(issue)
                .and_then(Option::as_ref)
                .is_some_and(|mask| kind == UndoSkipKind::Comment || may_read(mask, field))
        };
        for (issue, nets) in &mut plan.by_issue {
            nets.retain(|net| readable(issue, &net.field, UndoSkipKind::NotUndoable));
        }
        plan.by_issue.retain(|_, nets| !nets.is_empty());
        plan.skips.retain(|skip| {
            super::issue_of(&skip.entity)
                .is_some_and(|issue| readable(&issue, &skip.field, skip.kind))
        });
        Ok(plan)
    }

    /// The fields that go back, grouped by issue, and the fields that stay.
    fn plan(evaluation: Evaluation, net: &NetOfRun) -> Plan {
        let mut by_issue: BTreeMap<IssueId, Vec<Net>> = BTreeMap::new();
        let mut skips = evaluation.skips;
        // A comment is named under its issue, which the event of its making says.
        let comment_issues = comment_issues_of(net.changed());
        let mut named_issues = std::collections::BTreeSet::new();
        for net in evaluation.back {
            if net.entity.kind.as_str() == RecordKind::Comment.code() {
                if let Some(issue) = comment_issues.get(&net.entity.id)
                    && named_issues.insert(*issue)
                {
                    skips.push(UndoSkip {
                        entity: EntityRef {
                            kind: EntityKind::from(RecordKind::Issue),
                            id: (*issue).into(),
                        },
                        field: COMMENT_FIELD.to_owned(),
                        kind: UndoSkipKind::Comment,
                        by: None,
                        at: None,
                    });
                }
                continue;
            }
            let issue = (net.entity.kind.as_str() == RecordKind::Issue.code())
                .then(|| IssueId::from_bytes(net.entity.id.to_bytes()).ok())
                .flatten()
                .filter(|_| restorable(&net));
            match issue {
                Some(issue) => by_issue.entry(issue).or_default().push(net),
                None => skips.push(UndoSkip {
                    entity: net.entity,
                    field: net.field,
                    kind: UndoSkipKind::NotUndoable,
                    by: None,
                    at: None,
                }),
            }
        }
        Plan {
            by_issue,
            versions: BTreeMap::new(),
            skips,
            version: evaluation.version,
        }
    }
}

/// The field that names a comment among the fields a run changed: a comment cannot be put back yet
/// (M7-F60), so it is one skip under its issue.
pub(crate) const COMMENT_FIELD: &str = "comment";

/// The issue of each comment of the run, from the event that made it.
fn comment_issues_of<'a>(
    nets: impl Iterator<Item = &'a Net>,
) -> BTreeMap<bb_core::id::EntityId, IssueId> {
    nets.filter(|net| net.entity.kind.as_str() == RecordKind::Comment.code())
        .filter(|net| net.field == crate::member::comment::ISSUE)
        .filter_map(|net| {
            let id = net.after.as_ref()?.as_str()?.parse::<IssueId>().ok()?;
            Some((net.entity.id, id))
        })
        .collect()
}

/// Whether the reader may read the field of an issue: the one rule of the history.
fn may_read(mask: &FieldMask, field: &str) -> bool {
    let probe = [FieldChange {
        field: field.to_owned(),
        before: None,
        after: None,
    }];
    !visible_changes(&|key| mask.can_see(key), &probe).is_empty()
}

/// What an undo would do, by issue.
pub(crate) struct Plan {
    by_issue: BTreeMap<IssueId, Vec<Net>>,
    /// The version each issue had when the plan looked at it: the undo writes on it.
    versions: BTreeMap<IssueId, i64>,
    skips: Vec<UndoSkip>,
    version: UndoVersion,
}

impl Plan {
    fn goes_back(&self) -> u64 {
        self.by_issue.values().map(|nets| nets.len() as u64).sum()
    }

    fn preview(self) -> UndoPreview {
        let goes_back = self.goes_back();
        let skipped = self.skips.len() as u64;
        let mut skips = self.skips;
        skips.truncate(SKIPS_NAMED_MAX);
        UndoPreview {
            version: self.version,
            goes_back,
            skips,
            skipped,
        }
    }
}

impl<S: AuthStore> RunStore for RunsOf<'_, '_, S> {
    type Error = AppError;

    async fn open_run(
        &self,
        run: RunId,
        kind: RunKind,
        actor: &RunActor,
        at: Timestamp,
    ) -> Result<(), AppError> {
        let stored = ManyChangeRun {
            id: run,
            kind,
            person: actor.person,
            app: actor.app.clone(),
            agent: actor.agent,
            action: json!({}),
            items: Vec::new(),
            status: ManyChangeStatus::Running,
            done: 0,
            changed: 0,
            skipped: Vec::new(),
            created_at: at,
            finished_at: None,
        };
        let event = self.ctx.event(
            actor.event_actor(),
            entity(RecordKind::ChangeRun, run),
            vec![
                set_to("kind", json!(kind.code())),
                set_to("status", json!(ManyChangeStatus::Running.code())),
            ],
        )?;
        self.ctx
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { S::insert_many_changes_run(transaction, &stored).await })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }

    async fn close_run(&self, run: RunId, ended: RunEnd, _at: Timestamp) -> Result<bool, AppError> {
        let found = self
            .ctx
            .store()
            .many_changes_run(run)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some(found) = found.filter(|found| !found.status.is_over()) else {
            return Ok(false);
        };
        let who = EventActor::Person(found.person);
        self.ctx
            .end_many_changes_run(run, ended.status(), who)
            .await?;
        Ok(true)
    }

    async fn run(&self, run: RunId) -> Result<Option<RunSummary>, AppError> {
        Ok(self
            .ctx
            .store()
            .many_changes_run(run)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|found| RunSummary {
                id: found.id,
                kind: found.kind,
                actor: RunActor {
                    person: found.person,
                    app: found.app,
                    agent: found.agent,
                },
                status: found.status,
                started_at: found.created_at,
                ended_at: found.finished_at,
            }))
    }

    async fn run_changes(
        &self,
        run: RunId,
        after: Option<EventId>,
        limit: u64,
    ) -> Result<Vec<RunChange>, AppError> {
        Ok(self
            .ctx
            .store()
            .run_events(run, after, limit)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|(_, event)| RunChange {
                event: event.id,
                at: event.time,
                entity: event.entity,
                changes: event.changes,
            })
            .collect())
    }

    async fn preview_undo(&self, run: RunId) -> Result<UndoPreview, AppError> {
        self.ctx.own_run(self.actor, run).await?;
        Ok(self.plan_for_reader(run).await?.preview())
    }

    async fn undo_run(
        &self,
        run: RunId,
        version: &UndoVersion,
        by: &RunActor,
        _at: Timestamp,
    ) -> Result<UndoOutcome, AppError> {
        if by.person != self.actor.person.id {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        self.ctx.own_run(self.actor, run).await?;
        let plan = self.plan_for_reader(run).await?;
        if plan.version != *version {
            return Ok(UndoOutcome::Changed(plan.preview()));
        }
        Ok(UndoOutcome::Undone(self.apply_plan(run, plan).await?))
    }
}

impl<S: AuthStore> RunsOf<'_, '_, S> {
    /// Puts back what the plan says, issue by issue, on the versions the plan read. An error that
    /// stops it half way is kept for [`RunsOf::take_interrupt`]: what went back before it is
    /// answered and recorded first, so the run does not offer to put it back again.
    pub(crate) async fn apply_plan(&self, run: RunId, plan: Plan) -> Result<UndoResult, AppError> {
        let mut skips = plan.skips;
        let mut undone = 0;
        self.ctx.set_reason(Some(bb_core::run::undo_reason(run)));
        for (issue, nets) in &plan.by_issue {
            let version = plan.versions.get(issue).copied().unwrap_or_default();
            match self.restore(*issue, version, nets).await {
                Ok(()) => undone += nets.len() as u64,
                Err(Failed::Kept(kind)) => skips.extend(nets.iter().map(|net| UndoSkip {
                    entity: net.entity.clone(),
                    field: net.field.clone(),
                    kind,
                    by: None,
                    at: None,
                })),
                Err(Failed::Error(error)) => {
                    *self
                        .interrupted
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error);
                    break;
                }
            }
        }
        self.ctx.set_reason(None);
        let skipped = skips.len() as u64;
        Ok(UndoResult {
            undone,
            skips,
            skipped,
        })
    }

    /// The error that stopped the undo half way, if one did.
    pub(crate) fn take_interrupt(&self) -> Option<AppError> {
        self.interrupted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

/// Why the fields of an issue were not put back: a reason to keep them as they are, or an error
/// that is no reason (the database was busy, something broke), which goes up and is no skip.
enum Failed {
    Kept(UndoSkipKind),
    Error(AppError),
}

/// The skip a reason of the judgment of a change is, in the words of an undo.
fn kept(reason: SkipReason) -> UndoSkipKind {
    match reason {
        SkipReason::ChangedSince => UndoSkipKind::ChangedSince,
        SkipReason::NotFound => UndoSkipKind::Gone,
        SkipReason::NotAllowed => UndoSkipKind::NotAllowed,
        _ => UndoSkipKind::NotUndoable,
    }
}

impl<S: AuthStore> RunsOf<'_, '_, S> {
    /// Puts the fields of one issue back by the use case of a single change, as the person who
    /// undoes, on the version the plan saw; the reason it did not is the kind of skip.
    async fn restore(&self, issue: IssueId, version: i64, nets: &[Net]) -> Result<(), Failed> {
        let input = issue_change(self.token, issue, version, nets)
            .ok_or(Failed::Kept(UndoSkipKind::NotUndoable))?;
        match change_issue(self.ctx, input).await {
            Ok(_) => Ok(()),
            Err(error) => match skip_of(error) {
                Ok(reason) => Err(Failed::Kept(kept(reason))),
                Err(error) => Err(Failed::Error(error)),
            },
        }
    }
}
