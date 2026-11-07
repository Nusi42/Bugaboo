//! What goes with the start and the end of an attempt (AI-33, AI-35, ADR-0029, decisions 4, 6
//! and 7): its run, the move of the status after an accept by itself, and the one short comment
//! that says the result is ready.

use bb_core::agent_records::Attempt;
use bb_core::app_paths;
use bb_core::id::RunId;
use bb_core::issue::StatusGroup;
use bb_core::message;
use bb_core::run::{RunActor, RunEnd, RunKind};

use super::attempt::Mine;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};
use crate::comments::{AddComment, add_comment};
use crate::issues::{ChangeIssue, change_issue};
use crate::runs::{RunStore, RunsOf};

impl<S: AuthStore> Ctx<'_, S> {
    /// The actor of a run of this agent's attempt.
    fn attempt_actor(mine: &Mine) -> RunActor {
        RunActor {
            person: mine.work.person,
            app: None,
            agent: Some(mine.work.agent),
        }
    }

    /// Opens the run of a new attempt: everything the agent changes on the issue meanwhile belongs
    /// to it, and *Throw away* undoes it in one step.
    pub(super) async fn open_attempt_run(
        &self,
        token: &str,
        mine: &Mine,
    ) -> Result<RunId, AppError> {
        let run = RunId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        RunsOf::new(self, &mine.actor, token)
            .open_run(
                run,
                RunKind::Attempt,
                &Self::attempt_actor(mine),
                self.now(),
            )
            .await?;
        Ok(run)
    }

    /// Closes the run of an attempt that ended.
    pub(super) async fn close_attempt_run(
        &self,
        token: &str,
        mine: &Mine,
        attempt: &Attempt,
        ended: RunEnd,
    ) -> Result<(), AppError> {
        if let Some(run) = attempt.run {
            RunsOf::new(self, &mine.actor, token)
                .close_run(run, ended, self.now())
                .await?;
        }
        Ok(())
    }

    /// After an attempt was accepted by itself the issue moves to the one status of the *Done*
    /// group that the workflow allows from its current status. With none, or with several, the
    /// status stays and a person chooses (ADR-0029, decision 6, amendment M7). Says whether it
    /// moved.
    pub(super) async fn move_after_accept(
        &self,
        token: &str,
        mine: &Mine,
    ) -> Result<bool, AppError> {
        let failed = |error| internal::<S>(&error);
        let Some(issue) = self.store().issue(mine.work.issue).await.map_err(failed)? else {
            return Ok(false);
        };
        let chain = self.chain_of(issue.project).await?;
        let workflow = self
            .workflow_directory()
            .await?
            .in_force(&chain, Some(issue.issue_type));
        let mut done = workflow
            .changes_from(&issue.status)
            .filter(|change| {
                workflow
                    .status(&change.to)
                    .is_some_and(|status| status.group == StatusGroup::Done)
            })
            .map(|change| change.to.clone());
        let (Some(only), None) = (done.next(), done.next()) else {
            return Ok(false);
        };
        *locked(&self.accepting) = Some(issue.id);
        let moved = change_issue(
            self,
            ChangeIssue::status(token, issue.id.to_string(), issue.version, only),
        )
        .await;
        *locked(&self.accepting) = None;
        // A change the workflow does not let through (a rule of the change) leaves the status.
        Ok(moved.is_ok())
    }

    /// The one short comment, written as the agent and so marked *Written by an agent*, that says
    /// an attempt is ready and where to read it.
    pub(super) async fn write_summary(
        &self,
        token: &str,
        mine: &Mine,
        attempt: &Attempt,
    ) -> Result<(), AppError> {
        let key = self
            .readable_id_for(&mine.actor, mine.work.issue)
            .await?
            .unwrap_or_else(|| mine.work.issue.to_string());
        let address = self.app_address(&app_paths::ISSUE.replace("$issue", &key));
        let text = message::agent_work_summary(attempt.number, address).render();
        add_comment(
            self,
            AddComment {
                session_token: token.to_owned(),
                issue: mine.work.issue.to_string(),
                text,
            },
        )
        .await
        .map(drop)
    }
}
