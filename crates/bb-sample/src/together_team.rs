//! The Team members change statuses under the workflows.

use bb_app::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use bb_core::id::IssueId;
use bb_core::workflow::StatusKey;

use crate::error::{SampleError, Step};
use crate::issues::{MadeIssue, Maker};
use crate::issues_setup::json;
use crate::together::{FOLLOWED_PERCENT, REPORTED_PERCENT, TRIES_PER_CHANGE, Work};

impl Maker<'_, '_> {
    /// The Team members change statuses under the workflows, mostly of issues that Viewers follow
    /// and some of the issues that Contributors and Guests reported, so those people are told too.
    pub(crate) async fn team_changes(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let wanted = usize::try_from(work.counts.team_status_changes).unwrap_or(usize::MAX);
        let (mut done, mut tries) = (0_usize, 0_usize);
        while done < wanted && tries < wanted.saturating_mul(TRIES_PER_CHANGE) {
            tries += 1;
            let id = if work.random.chance(REPORTED_PERCENT) && !work.reported.is_empty() {
                work.reported[work.random.below(work.reported.len())].id
            } else {
                let at = if work.random.chance(FOLLOWED_PERCENT) && !work.followed.is_empty() {
                    work.followed[work.random.below(work.followed.len())]
                } else {
                    work.random.below(self.issues.len())
                };
                let Some(Some(made)) = self.issues.get(at) else {
                    continue;
                };
                made.id
            };
            let person = self
                .hands
                .anyone(u32::try_from(work.random.below(1 << 16)).unwrap_or(0));
            if self.change_status(work, person, id).await? {
                done += 1;
            }
        }
        Ok(())
    }

    /// One status change of an issue by a person, if the workflow offers them one: read the issue as
    /// the person to see what they may do (a change that asks for the person who does the work
    /// asks for them), then make one at random. `false` when there is none.
    pub(crate) async fn change_status(
        &mut self,
        work: &mut Work,
        person: usize,
        issue_id: IssueId,
    ) -> Result<bool, SampleError> {
        let token = self.hands.token(person).await?;
        let ctx = self.world.services.ctx(self.world.request());
        let detail = json(
            issue(
                &ctx,
                IssueQuery {
                    session_token: token.clone(),
                    issue: issue_id.to_string(),
                },
            )
            .await,
            Step::Change,
        )?;
        let read = MadeIssue::of(&detail["issue"])?;
        let Some(option) = work.random.pick(&read.changes) else {
            return Ok(false);
        };
        self.world.tick();
        json(
            change_issue(
                &ctx,
                ChangeIssue {
                    session_token: token,
                    issue: issue_id.to_string(),
                    version: read.version,
                    issue_type: None,
                    title: None,
                    status: Some(
                        StatusKey::new(&option.key)
                            .map_err(|error| SampleError::NotMade(error.to_string()))?,
                    ),
                    priority: None,
                    assigned_to: read.assigned_for(option, self.world.people[person].id),
                    fields: std::collections::BTreeMap::new(),
                },
            )
            .await,
            Step::Change,
        )?;
        Ok(true)
    }
}
