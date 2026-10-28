//! The people work together (M4 WP-10, M3-F111): after the Team members made the issues, the
//! Viewers follow the ones they care about, Contributors and Guests report issues of their own and
//! comment on them, Contributors change what they reported, the Team members change statuses under
//! the workflows (a change that asks for something gets it), files are added to issues and comments,
//! and the people who only read mark some of their notifications as read.
//!
//! All of it goes through the use cases, one change a second of the sample's clock, by the people
//! whose role allows it, so the workflows by role are the real ones. The counts are those of
//! [`WorkPlan::counts`], which the judge of the dump reads too.

use std::collections::BTreeSet;

use bb_app::access::StartingRole;
use bb_app::follow::{FollowIssue, follow_issue};
use bb_core::id::IssueId;
use bb_dev_words::WorkCounts;
use bb_dev_words::sample::WorkPlan;

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_words::IssueWords;
use crate::people_plan::SamplePerson;
use crate::random::SeededRandom;

/// A different stream of random numbers than the issues use, whatever the seed.
const SALT: u64 = 0x57_4F_52_4B;
/// How many tries it takes to find an issue that offers a status change a person may make, per
/// change wanted, before the sample gives up (a sample of a few issues may have none).
pub(crate) const TRIES_PER_CHANGE: usize = 40;
/// How many notifications a page holds when a person reads their own.
pub(crate) const NOTIFICATIONS_PAGE: u32 = 100;
/// Of every hundred issues a Team member changes, this many are ones that a Viewer follows, so the
/// Viewers have something to be told about.
pub(crate) const FOLLOWED_PERCENT: u32 = 60;
/// Of every hundred status changes of Team members, this many are of issues that a Contributor or
/// a Guest reported.
pub(crate) const REPORTED_PERCENT: u32 = 25;
/// Of every hundred reports, this many get a comment from the person who reported them.
pub(crate) const COMMENTED_PERCENT: u32 = 60;

/// An issue that a Contributor or a Guest reported.
#[derive(Clone, Debug)]
pub(crate) struct Reported {
    pub(crate) id: IssueId,
    /// The person (a place in the plan of the people) who reported it.
    pub(crate) author: usize,
}

/// What the stage keeps while it runs.
pub(crate) struct Work {
    pub(crate) random: SeededRandom,
    pub(crate) counts: WorkCounts,
    pub(crate) plan: WorkPlan,
    pub(crate) words: IssueWords,
    /// The places in the plan of the people, by role.
    pub(crate) contributors: Vec<usize>,
    pub(crate) guests: Vec<usize>,
    pub(crate) viewers: Vec<usize>,
    /// The issues (places in the plan of the issues) that a Viewer follows.
    pub(crate) followed: Vec<usize>,
    pub(crate) reported: Vec<Reported>,
}

impl Work {
    fn new(
        people: &[SamplePerson],
        seed: u64,
        plan: &WorkPlan,
        counts: WorkCounts,
    ) -> Result<Self, SampleError> {
        let of_role = |role| {
            people
                .iter()
                .enumerate()
                .filter(|(_, person)| person.role == role && !person.code_app)
                .map(|(at, _)| at)
                .collect::<Vec<_>>()
        };
        Ok(Self {
            random: SeededRandom::new(seed ^ SALT),
            counts,
            plan: plan.clone(),
            words: IssueWords::bundled()?,
            contributors: of_role(StartingRole::Contributor),
            guests: of_role(StartingRole::Guest),
            viewers: of_role(StartingRole::Viewer),
            followed: Vec::new(),
            reported: Vec::new(),
        })
    }
}

impl Maker<'_, '_> {
    /// The people work together; the first moment it happens at is the moment the clock stands at.
    pub(crate) async fn work_together(&mut self, asked: u32, full: u32) -> Result<(), SampleError> {
        let plan = self.names.work.clone();
        let counts = plan.counts(asked, full);
        let people: Vec<SamplePerson> = self
            .world
            .people
            .iter()
            .map(|made| made.person.clone())
            .collect();
        let mut work = Work::new(&people, self.world.seed, &plan, counts)?;
        self.viewers_follow(&mut work).await?;
        self.people_report(&mut work, &people).await?;
        self.team_changes(&mut work).await?;
        self.contributors_change(&mut work).await?;
        self.add_files(&mut work).await?;
        self.run_the_jobs().await?;
        self.people_read(&mut work).await?;
        self.save_searches().await
    }

    /// The Viewers follow issues: together as many as the plan says, each one different. The issue
    /// is one the Team members made, so the Viewer sees it.
    async fn viewers_follow(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let made: Vec<usize> = (0..self.issues.len())
            .filter(|at| matches!(self.issues.get(*at), Some(Some(_))))
            .collect();
        if work.viewers.is_empty() || made.is_empty() {
            return Err(SampleError::NotMade("viewers".to_owned()));
        }
        let wanted = usize::try_from(work.counts.viewer_follows).unwrap_or(usize::MAX);
        let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut tries = 0_usize;
        while seen.len() < wanted && tries < wanted.saturating_mul(TRIES_PER_CHANGE) {
            tries += 1;
            let viewer = work.viewers[seen.len() % work.viewers.len()];
            let at = made[work.random.below(made.len())];
            if !seen.insert((viewer, at)) {
                continue;
            }
            let token = self.hands.token(viewer).await?;
            let id = self.issue(at)?.id;
            self.world.tick();
            follow_issue(
                &self.world.services.ctx(self.world.request()),
                FollowIssue {
                    session_token: token,
                    issue: id.to_string(),
                },
            )
            .await
            .map_err(|error| SampleError::step(Step::Following, &error))?;
            if !work.followed.contains(&at) {
                work.followed.push(at);
            }
        }
        Ok(())
    }
}
