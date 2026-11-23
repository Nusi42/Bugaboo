//! The job `privacy.finish-removal` (PRIV-04, ADR-0044, decision 6, M9-F51): the work of a removal
//! that grows with the organization. The change of the removal records the decision, closes the
//! account, hides the comments that go, writes the row of the removal that is not finished
//! (`PendingRemoval`) with the person's words sealed as a secret of their own, and queues this job;
//! the job does the rest in steps, each a short change of its own, so a person who writes meanwhile
//! waits for one step at most:
//!
//! 1. the files of the person's exports leave the folder of the exports (a file that is gone
//!    already is no error);
//! 2. when the comments go, [`FINISH_REMOVAL_COMMENTS`] of them a step, and the full-text index
//!    forgets them and makes the rows of their issues again;
//! 3. the changes of the events, the history and the notifications are scrubbed of the person's
//!    name, handles and address, a page of each table a step (`RemovalStore::scrub_step`): only
//!    reading every row finds them (the changes about the person's records were scrubbed in the
//!    change of the removal, through those records, before they went). The words come from the
//!    secret of the removal (`RemovalStore::removal_words`); the payload names only the person and
//!    the step. Before the history and the notifications are scrubbed, their readers catch up with
//!    the events, which are scrubbed by then, so no row of them is made behind the scrub;
//! 4. after the last step ([`FinishStep::Last`]) the database lets go of the old text (on SQLite
//!    the index merges its segments and the write-ahead log is emptied), and the row of the removal
//!    and the secret of its words go with their key, which ends *removing…*
//!    (`RemovalStore::pending_removal`).
//!
//! Each step queues the next in the same change that gives the row the next step and its job
//! (`RemovalStore::move_removal`), and only the job the row names does a step: a step that runs
//! twice, because its worker lost the lease after it was done, finds another job named and queues
//! nothing, so the chain never forks. Every step is safe to run again: a file that is gone, a
//! comment that is gone and a row scrubbed once leave nothing to do the second time.
//!
//! A job that gives up stays on the dead-letter list while its removal is not finished (the purge
//! of dead jobs leaves it), the status page counts the removal as stopped, and the daily job
//! `privacy.resume-removals` gives it a new job from the step of its row ([`Ctx::resume_removals`]):
//! a removal is a duty under the law, so it goes on without an administrator, and a fault that
//! stays shows on the status page every day.
//!
//! A job queued before the removal worked in steps (`{comments, issues, exports}`) is still done the
//! way it was then ([`EarlierFinish`]).

use bb_core::id::{CommentId, DataExportId, IssueId, JobId, UserId};
use bb_core::job::Job;
use bb_core::removal::PendingRemoval;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::store::{Scrub, ScrubPlace, ScrubbedTable};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::jobs::JobKind;

/// The most comments one step of the job deletes: few enough that a step is a short write.
pub const FINISH_REMOVAL_COMMENTS: u64 = 200;

/// The most issues one step of a job queued before the removal worked in steps reindexes.
const EARLIER_FINISH_ISSUES: usize = 500;

/// What a step of the job does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinishStep {
    /// Delete the files of these exports of the person.
    Files(Vec<DataExportId>),
    /// Delete the next comments of the person.
    Comments,
    /// Scrub the changes from this place on.
    Scrub(ScrubPlace),
    /// Let the database go of the old text and end the removal.
    Last,
}

/// The payload of the job: the person and the step, nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishRemoval {
    /// The person.
    pub person: UserId,
    /// What this step does.
    pub step: FinishStep,
}

/// The payload of a job queued before the removal worked in steps: the comments it deleted, the
/// issues they were on and the exports whose files go. Its job is done the way it was then.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierFinish {
    /// The comments the removal deleted, whose rows of the index go.
    #[serde(default)]
    pub comments: Vec<CommentId>,
    /// The issues whose texts changed, whose rows of the index are made again.
    #[serde(default)]
    pub issues: Vec<IssueId>,
    /// The exports of the person the removal deleted, whose files go.
    #[serde(default)]
    pub exports: Vec<DataExportId>,
}

/// A payload of the job, in either shape.
#[derive(Deserialize)]
#[serde(untagged)]
enum Payload {
    Steps(FinishRemoval),
    Earlier(EarlierFinish),
}

/// The error of a value that cannot be written as JSON (IDs and places always can).
fn unwritten(error: &serde_json::Error) -> AppError {
    AppError::Internal(error.to_string())
}

impl FinishRemoval {
    /// The payload of the job.
    ///
    /// # Errors
    /// An internal error when the IDs cannot be written as JSON (they always can).
    pub fn payload(&self) -> Result<Value, AppError> {
        serde_json::to_value(self).map_err(|error| unwritten(&error))
    }

    /// The step as the row of the removal keeps it.
    ///
    /// # Errors
    /// An internal error when the step cannot be written as JSON (it always can).
    pub fn step_text(&self) -> Result<String, AppError> {
        serde_json::to_string(&self.step).map_err(|error| unwritten(&error))
    }

    /// The step the row of `pending` keeps, as a payload.
    fn of(pending: &PendingRemoval) -> Result<Self, AppError> {
        Ok(Self {
            person: pending.person,
            step: serde_json::from_str(&pending.step).map_err(|_| {
                AppError::Internal("the step of a removal is unreadable".to_owned())
            })?,
        })
    }
}

/// The row of a removal that is not finished and the payload of its first job, for the change of
/// the removal.
///
/// # Errors
/// An internal error when the step cannot be written as JSON (it always can).
pub(crate) fn first_step(
    person: UserId,
    comments_go: bool,
    exports: Vec<DataExportId>,
    job: JobId,
    at: Timestamp,
) -> Result<(PendingRemoval, Value), AppError> {
    let first = FinishRemoval {
        person,
        step: FinishStep::Files(exports),
    };
    Ok((
        PendingRemoval {
            person,
            comments_go,
            job,
            step: first.step_text()?,
            created_at: at,
            updated_at: at,
        },
        first.payload()?,
    ))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One step of the job `privacy.finish-removal` (see the module).
    ///
    /// # Errors
    /// `request.invalid` for a payload that is not one of this job; otherwise the error of the
    /// store or of the files.
    pub(crate) async fn run_finish_removal(&self, job: &Job) -> Result<(), AppError> {
        match serde_json::from_value(job.payload.clone())
            .map_err(|_| AppError::Refused(bb_core::ErrorCode::InvalidRequest))?
        {
            Payload::Steps(left) => self.finish_step(job, &left).await,
            Payload::Earlier(earlier) => self.earlier_finish(earlier).await,
        }
    }

    /// A step of a removal, done only by the job its row names. The job keeps its lease while the
    /// step works ([`Ctx::while_kept`]): a step that catches up the history or compacts the
    /// full-text index on a large organization may take longer than the lease.
    async fn finish_step(&self, job: &Job, left: &FinishRemoval) -> Result<(), AppError> {
        let person = left.person;
        let Some(pending) = self
            .store()
            .pending_removal(person)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|pending| pending.job == job.id)
        else {
            // The removal is finished, or this is a copy of a step that moved on.
            return Ok(());
        };
        let step = Box::pin(self.do_step(job.id, &pending, left));
        match self.while_kept(job, step).await?? {
            Some(next) => {
                self.queue_step(&FinishRemoval { person, step: next }, job.id)
                    .await
            }
            None => Ok(()),
        }
    }

    /// The work of one step of the removal the row `pending` keeps; the step after it, or `None`
    /// when the removal is finished.
    async fn do_step(
        &self,
        job: JobId,
        pending: &PendingRemoval,
        left: &FinishRemoval,
    ) -> Result<Option<FinishStep>, AppError> {
        let failed = |error| internal::<S>(&error);
        let person = left.person;
        let scrub_from_the_start = FinishStep::Scrub(ScrubPlace::default());
        let next = match &left.step {
            FinishStep::Files(exports) => {
                for export in exports {
                    self.exports()
                        .delete(*export)
                        .await
                        .map_err(|error| AppError::Internal(error.to_string()))?;
                }
                if pending.comments_go {
                    FinishStep::Comments
                } else {
                    scrub_from_the_start
                }
            }
            FinishStep::Comments => {
                if self.delete_comments_step(person).await? < FINISH_REMOVAL_COMMENTS {
                    scrub_from_the_start
                } else {
                    FinishStep::Comments
                }
            }
            FinishStep::Scrub(place) => {
                self.caught_up_before(place).await?;
                let words = self
                    .store()
                    .removal_words(person)
                    .await
                    .map_err(failed)?
                    .ok_or_else(|| {
                        AppError::Internal("the words of a removal are gone".to_owned())
                    })?;
                let scrub = Scrub {
                    names: vec![words.name],
                    handles: words.handles,
                    addresses: vec![words.address],
                    ..Scrub::default()
                };
                match self
                    .store()
                    .scrub_step(&scrub, place)
                    .await
                    .map_err(failed)?
                {
                    Some(next) => FinishStep::Scrub(next),
                    None => FinishStep::Last,
                }
            }
            FinishStep::Last => {
                self.store().compact_removed_texts().await.map_err(failed)?;
                let ended = self
                    .store()
                    .record_changes(&[], move |transaction| {
                        Box::pin(async move { S::end_removal(transaction, person, job).await })
                    })
                    .await
                    .map_err(failed)?;
                if ended {
                    tracing::info!("a removal of a person is finished");
                }
                return Ok(None);
            }
        };
        Ok(Some(next))
    }

    /// Before the scrub of the history or of the notifications starts, their readers catch up with
    /// the events, which are scrubbed by then: a row made afterwards is made from a scrubbed event,
    /// and a row made from an event read before its scrub is in the table when the scrub reads it.
    async fn caught_up_before(&self, place: &ScrubPlace) -> Result<(), AppError> {
        if !place.after.is_empty() {
            return Ok(());
        }
        match place.table {
            ScrubbedTable::Events => {}
            ScrubbedTable::History => {
                self.catch_up_history().await?;
            }
            ScrubbedTable::Notifications => {
                self.catch_up_notifications().await?;
            }
        }
        Ok(())
    }

    /// Deletes the next comments of `person` in a change of their own, then lets the full-text
    /// index forget them; how many went.
    async fn delete_comments_step(&self, person: UserId) -> Result<u64, AppError> {
        let failed = |error| internal::<S>(&error);
        let gone = self
            .store()
            .record_changes(&[], move |transaction| {
                Box::pin(async move {
                    S::delete_comments_of(transaction, person, FINISH_REMOVAL_COMMENTS).await
                })
            })
            .await
            .map_err(failed)?;
        let comments: Vec<_> = gone.iter().map(|(comment, _)| *comment).collect();
        let mut issues: Vec<_> = gone.iter().map(|(_, issue)| *issue).collect();
        issues.sort_unstable();
        issues.dedup();
        self.store()
            .reindex_removed_texts(&issues, &comments)
            .await
            .map_err(failed)?;
        Ok(u64::try_from(gone.len()).unwrap_or(u64::MAX))
    }

    /// Queues the job of `next` and names it in the row of the removal, in one change, while
    /// `from` is the job the row names; nothing when another job is named by now.
    async fn queue_step(&self, next: &FinishRemoval, from: JobId) -> Result<(), AppError> {
        let id = JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let kind = JobKind::FinishRemoval.name()?;
        let payload = next.payload()?;
        let step = next.step_text()?;
        let (person, now) = (next.person, self.now());
        self.store()
            .record_changes(&[], move |transaction| {
                Box::pin(async move {
                    if S::move_removal(transaction, person, from, id, &step, now).await? {
                        S::enqueue_job(transaction, id, &kind, &payload, now, now).await?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Gives every removal that stopped (its job gave up or is gone) a new job from the step its row
    /// keeps: the work of the daily job `privacy.resume-removals`. How many it gave a new job.
    ///
    /// # Errors
    /// The error of the store.
    pub(crate) async fn resume_removals(&self) -> Result<usize, AppError> {
        let stopped = self
            .store()
            .stopped_removals()
            .await
            .map_err(|error| internal::<S>(&error))?;
        for pending in &stopped {
            self.queue_step(&FinishRemoval::of(pending)?, pending.job)
                .await?;
        }
        Ok(stopped.len())
    }

    /// A job queued before the removal worked in steps: the files go, the full-text index forgets
    /// the deleted comments and makes their issues again, [`EARLIER_FINISH_ISSUES`] a step, and
    /// after the last the database lets go of the old text.
    async fn earlier_finish(&self, mut left: EarlierFinish) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        for export in &left.exports {
            self.exports()
                .delete(*export)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        let rest = (left.issues.len() > EARLIER_FINISH_ISSUES).then(|| EarlierFinish {
            issues: left.issues.split_off(EARLIER_FINISH_ISSUES),
            ..EarlierFinish::default()
        });
        self.store()
            .reindex_removed_texts(&left.issues, &left.comments)
            .await
            .map_err(failed)?;
        match rest {
            Some(rest) => {
                let id = JobId::generate(&mut self.ids())
                    .map_err(|error| AppError::Internal(error.to_string()))?;
                let payload = serde_json::to_value(&rest).map_err(|error| unwritten(&error))?;
                self.store()
                    .queue_job(
                        id,
                        &JobKind::FinishRemoval.name()?,
                        &payload,
                        self.now(),
                        self.now(),
                    )
                    .await
                    .map_err(failed)
            }
            None => self.store().compact_removed_texts().await.map_err(failed),
        }
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::SequentialIds;

    use super::*;
    use crate::privacy::removal::ScrubbedTable;

    #[test]
    fn priv_04_the_payload_of_a_step_names_the_person_and_the_step_and_no_word() {
        let mut ids = SequentialIds::default();
        let left = FinishRemoval {
            person: UserId::generate(&mut ids),
            step: FinishStep::Scrub(ScrubPlace {
                table: ScrubbedTable::History,
                after: vec!["00ff".to_owned(), "a0".to_owned()],
            }),
        };
        let payload = left.payload().unwrap_or_default();
        assert_eq!(payload["step"]["scrub"]["table"], "history");
        let members: Vec<&String> = payload
            .as_object()
            .map(|object| object.keys().collect())
            .unwrap_or_default();
        assert_eq!(members, ["person", "step"]);
        let Ok(Payload::Steps(read)) = serde_json::from_value(payload) else {
            unreachable!()
        };
        assert_eq!(read, left);
    }

    #[test]
    fn priv_04_a_payload_from_before_the_steps_is_read_as_one() {
        let mut ids = SequentialIds::default();
        let comment = CommentId::generate(&mut ids).unwrap_or_else(|_| unreachable!());
        let issue = IssueId::generate(&mut ids).unwrap_or_else(|_| unreachable!());
        let earlier =
            serde_json::json!({ "comments": [comment], "issues": [issue], "exports": [] });
        let Ok(Payload::Earlier(read)) = serde_json::from_value(earlier) else {
            unreachable!()
        };
        assert_eq!(read.comments, [comment]);
        assert_eq!(read.issues, [issue]);
        assert!(serde_json::from_value::<Payload>(serde_json::json!({ "other": 1 })).is_err());
    }
}
