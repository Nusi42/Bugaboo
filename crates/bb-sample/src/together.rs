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
use bb_app::comments::{AddComment, add_comment};
use bb_app::follow::{FollowIssue, follow_issue};
use bb_app::issues::{ChangeIssue, CreateIssue, IssueQuery, change_issue, create_issue, issue};
use bb_app::notifications::{
    NotificationsQuery, ReadNotifications, Which, notifications, read_notifications,
};
use bb_app::tags::{AddTag, add_tag};
use bb_core::id::{IssueId, NotificationId};
use bb_core::notification::NotificationFilter;
use bb_core::workflow::StatusKey;
use bb_dev_words::WorkCounts;
use bb_dev_words::sample::{Layout, WorkPlan};

use crate::error::{SampleError, Step};
use crate::issues::{MadeIssue, Maker};
use crate::issues_setup::{id_in, json};
use crate::issues_text::describe;
use crate::issues_words::IssueWords;
use crate::organization::projects_of;
use crate::people_plan::SamplePerson;
use crate::random::SeededRandom;

/// A different stream of random numbers than the issues use, whatever the seed.
const SALT: u64 = 0x57_4F_52_4B;
/// How many tries it takes to find an issue that offers a status change a person may make, per
/// change wanted, before the sample gives up (a sample of a few issues may have none).
const TRIES_PER_CHANGE: usize = 40;
/// How many notifications a page holds when a person reads their own.
const NOTIFICATIONS_PAGE: u32 = 100;
/// Of every hundred issues a Team member changes, this many are ones that a Viewer follows, so the
/// Viewers have something to be told about.
const FOLLOWED_PERCENT: u32 = 60;
/// Of every hundred status changes of Team members, this many are of issues that a Contributor or
/// a Guest reported.
const REPORTED_PERCENT: u32 = 25;
/// Of every hundred reports, this many get a comment from the person who reported them.
const COMMENTED_PERCENT: u32 = 60;

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
    contributors: Vec<usize>,
    guests: Vec<usize>,
    viewers: Vec<usize>,
    /// The issues (places in the plan of the issues) that a Viewer follows.
    followed: Vec<usize>,
    reported: Vec<Reported>,
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
        self.people_read(&mut work).await
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

    /// Contributors and Guests report issues in the projects they have a role on, and most of them
    /// write a comment under their own report.
    async fn people_report(
        &mut self,
        work: &mut Work,
        people: &[SamplePerson],
    ) -> Result<(), SampleError> {
        let organization = crate::organization::plan(people, &Layout::bundled()?.organization);
        for (who, total) in [
            (work.contributors.clone(), work.counts.contributor_reports),
            (work.guests.clone(), work.counts.guest_reports),
        ] {
            let reporters: Vec<(usize, Vec<String>)> = who
                .iter()
                .map(|person| (*person, projects_of(&organization, *person)))
                .filter(|(_, projects)| !projects.is_empty())
                .collect();
            if reporters.is_empty() {
                return Err(SampleError::NotMade("reporters".to_owned()));
            }
            for number in 0..usize::try_from(total).unwrap_or(0) {
                let (person, projects) = &reporters[number % reporters.len()];
                let project = &projects[(number / reporters.len()) % projects.len()];
                self.report(work, *person, project).await?;
            }
        }
        Ok(())
    }

    /// One report: a bug in a place of the project, and sometimes a comment under it.
    async fn report(
        &mut self,
        work: &mut Work,
        person: usize,
        project: &str,
    ) -> Result<(), SampleError> {
        let family = self
            .names
            .projects
            .iter()
            .find(|each| each.short_name == project)
            .ok_or_else(|| SampleError::NotMade(project.to_owned()))?;
        let place = work
            .random
            .pick(&family.places)
            .map_or(project, |place| place.short_name.as_str())
            .to_owned();
        let template = work
            .random
            .pick(&work.words.bug.titles)
            .cloned()
            .unwrap_or_default();
        let title = work.words.fill(&template, &family.area, &mut work.random);
        let description = describe(
            &work.words,
            &work.words.bug,
            true,
            &family.area,
            &mut work.random,
        );
        let token = self.hands.token(person).await?;
        self.world.tick();
        let ctx = self.world.services.ctx(self.world.request());
        let made = json(
            create_issue(
                &ctx,
                CreateIssue {
                    session_token: token.clone(),
                    project: place,
                    issue_type: *self
                        .setup
                        .types
                        .first()
                        .ok_or_else(|| SampleError::NotMade("issue type".to_owned()))?,
                    title,
                    description: Some(description),
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::new(),
                },
            )
            .await,
            Step::Issue,
        )?;
        let id: IssueId = id_in(&made, "id")?;
        work.reported.push(Reported { id, author: person });
        if work.random.chance(COMMENTED_PERCENT) {
            let text = work
                .random
                .pick(&work.words.comments.first)
                .cloned()
                .unwrap_or_default();
            self.world.tick();
            json(
                add_comment(
                    &ctx,
                    AddComment {
                        session_token: token,
                        issue: id.to_string(),
                        text: work.words.fill(&text, "", &mut work.random),
                    },
                )
                .await,
                Step::Comment,
            )?;
        }
        Ok(())
    }

    /// The Team members change statuses under the workflows, mostly of issues that Viewers follow
    /// and some of the issues that Contributors and Guests reported, so those people are told too.
    async fn team_changes(&mut self, work: &mut Work) -> Result<(), SampleError> {
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

    /// Contributors change what they reported: a Contributor may change the title and the tags of
    /// their own issues, not the status.
    async fn contributors_change(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let wanted = usize::try_from(work.counts.contributor_changes).unwrap_or(usize::MAX);
        let mut done = 0_usize;
        let mine: Vec<Reported> = work
            .reported
            .iter()
            .filter(|each| work.contributors.contains(&each.author))
            .cloned()
            .collect();
        if mine.is_empty() {
            return Err(SampleError::NotMade("issues of Contributors".to_owned()));
        }
        while done < wanted {
            let own = mine[done % mine.len()].clone();
            let token = self.hands.token(own.author).await?;
            let ctx = self.world.services.ctx(self.world.request());
            self.world.tick();
            if done.is_multiple_of(2) {
                self.retitle(work, &ctx, &token, own.id).await?;
            } else {
                let tag = work
                    .random
                    .pick(&self.names.tags)
                    .cloned()
                    .unwrap_or_default();
                add_tag(
                    &ctx,
                    AddTag {
                        session_token: token,
                        issue: own.id.to_string(),
                        name: tag,
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::Tags, &error))?;
            }
            done += 1;
        }
        Ok(())
    }

    /// A new title for an issue, made by the person whose session it is.
    async fn retitle(
        &mut self,
        work: &mut Work,
        ctx: &bb_app::auth::Ctx<'_, bb_store::AuthStorage>,
        token: &str,
        issue_id: IssueId,
    ) -> Result<(), SampleError> {
        let detail = json(
            issue(
                ctx,
                IssueQuery {
                    session_token: token.to_owned(),
                    issue: issue_id.to_string(),
                },
            )
            .await,
            Step::Change,
        )?;
        let version = MadeIssue::of(&detail["issue"])?.version;
        let template = work
            .random
            .pick(&work.words.bug.titles)
            .cloned()
            .unwrap_or_default();
        let title = work.words.fill(&template, "app", &mut work.random);
        json(
            change_issue(
                ctx,
                ChangeIssue {
                    session_token: token.to_owned(),
                    issue: issue_id.to_string(),
                    version,
                    issue_type: None,
                    title: Some(title),
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::new(),
                },
            )
            .await,
            Step::Change,
        )?;
        Ok(())
    }

    /// One status change of an issue by a person, if the workflow offers them one: read the issue as
    /// the person to see what they may do (a change that asks for the person who does the work
    /// asks for them), then make one at random. `false` when there is none.
    async fn change_status(
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

    /// The people who only read open their notifications and mark some of them as read.
    async fn people_read(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let readers: Vec<usize> = work
            .viewers
            .iter()
            .chain(&work.guests)
            .chain(&work.contributors)
            .copied()
            .collect();
        for person in readers {
            let token = self.hands.token(person).await?;
            let ctx = self.world.services.ctx(self.world.request());
            let mut to_mark: Vec<NotificationId> = Vec::new();
            let mut cursor = None;
            loop {
                let page = json(
                    notifications(
                        &ctx,
                        NotificationsQuery {
                            session_token: token.clone(),
                            filter: NotificationFilter::All,
                            cursor,
                            limit: NOTIFICATIONS_PAGE,
                        },
                    )
                    .await,
                    Step::Notifications,
                )?;
                for item in page["items"].as_array().map_or(&[][..], Vec::as_slice) {
                    if work.random.chance(work.plan.read_percent) {
                        to_mark.push(id_in(item, "id")?);
                    }
                }
                cursor = page["next_cursor"].as_str().map(str::to_owned);
                if cursor.is_none() {
                    break;
                }
            }
            for chunk in to_mark.chunks(bb_core::notification::AT_ONCE_MAX) {
                self.world.tick();
                read_notifications(
                    &ctx,
                    ReadNotifications {
                        session_token: token.clone(),
                        which: Which::These(chunk.to_vec()),
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::Notifications, &error))?;
            }
        }
        Ok(())
    }
}
