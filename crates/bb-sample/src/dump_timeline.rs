//! The part of the canonical dump that is about the plan of the goal (M8 WP-12, part 7): the goal
//! with its dated issues and dependencies, the public holidays of the organization, the work times
//! of the person who plans and the own estimates. Only counts, names and days before the due date
//! of the goal, so the dump does not change with the day the sample was made on; IDs are random by
//! design.
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`
//! (`bb_dev_words::sample::TimelinePlan`).

use std::collections::BTreeSet;

use bb_app::issues::IssueStore as _;
use bb_app::plan::TimelineStore as _;
use bb_app::relations::RelationStore as _;
use bb_app::settings::SettingsStore as _;
use bb_app::work_times::WorkTimesStore as _;
use bb_core::id::{IssueId, UserId};
use bb_core::issue::Issue;
use bb_core::message::{self, Message};
use bb_core::settings::work_times::WORK_TIMES_HOLIDAYS;
use bb_dev_words::sample::Layout;
use jiff::civil::Date;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// The goal.
#[derive(Debug, Default, Serialize)]
struct DumpGoal {
    title: String,
    /// The issues under it that have both dates.
    sub_issues: u64,
}

/// A dated issue under the goal.
#[derive(Debug, Serialize)]
struct DumpPlanned {
    title: String,
    /// The first person it is assigned to.
    person: String,
    starts_before_due: i64,
    ends_before_due: i64,
    estimate: u64,
}

/// The person who plans.
#[derive(Debug, Default, Serialize)]
struct DumpPerson {
    name: String,
    /// Whether she works on the day, and the work hours of the day, Monday first.
    usual_days: Vec<(bool, u16)>,
    share_percent: u8,
    away_times: u64,
}

/// An own estimate: the issue it is on, and whether the person on the bar set it.
#[derive(Debug, Serialize)]
struct DumpOwnEstimate {
    issue: String,
    set_by_the_person: bool,
}

/// The plan of the goal in the sample.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpTimeline {
    holidays: String,
    goal: DumpGoal,
    issues: Vec<DumpPlanned>,
    /// The title of the issue that must finish first, and of the one that waits.
    blocks: Vec<(String, String)>,
    person: DumpPerson,
    /// How many different people the issues of the goal are assigned to.
    people_on_the_goal: u64,
    own_estimates: Vec<DumpOwnEstimate>,
}

/// The days from `first` to `last`.
fn days_between(first: Date, last: Date) -> i64 {
    i64::from(first.until(last).map_or(0, |span| span.get_days()))
}

impl Sample {
    /// Reads the plan of the goal from the store.
    pub(crate) async fn timeline_of(&self, issues: &[Issue]) -> Result<DumpTimeline, SampleError> {
        let plan = Layout::bundled()?.issues.work.timeline;
        let mut dump = DumpTimeline::default();
        let organization = self.storage.setting_layers(None, None).await?;
        dump.holidays = organization.text(&WORK_TIMES_HOLIDAYS);
        dump.goal.title.clone_from(&plan.goal.title);
        let Some(goal) = issues.iter().find(|issue| issue.title == plan.goal.title) else {
            return Ok(dump);
        };
        let due = goal.due_date.unwrap_or_default();
        let mut people = BTreeSet::new();
        let mut titles = Vec::new();
        for parent in self.storage.sub_issues_of(goal.id).await? {
            let Some(sub) = issues.iter().find(|issue| issue.id == parent.child) else {
                continue;
            };
            let (Some(start), Some(end)) = (sub.start_date, sub.due_date) else {
                continue;
            };
            let first = self
                .storage
                .assigned_people(sub.id)
                .await?
                .into_iter()
                .next()
                .map(|assigned| assigned.person);
            people.extend(first);
            dump.issues.push(DumpPlanned {
                title: sub.title.clone(),
                person: first.map(|person| self.name_of(person)).unwrap_or_default(),
                starts_before_due: days_between(start, due),
                ends_before_due: days_between(end, due),
                estimate: sub
                    .estimated_time
                    .map_or(0, |minutes| u64::from(minutes.get())),
            });
            titles.push((sub.id, sub.title.clone()));
        }
        dump.goal.sub_issues = dump.issues.len() as u64;
        dump.issues
            .sort_by(|one, other| one.title.cmp(&other.title));
        dump.people_on_the_goal = people.len() as u64;
        self.blocks_among(&titles, &mut dump).await?;
        self.work_times_of(&plan.person, &mut dump).await?;
        self.own_estimates_among(&titles, &mut dump).await?;
        Ok(dump)
    }

    /// The name of a person of the sample.
    fn name_of(&self, id: UserId) -> String {
        self.people
            .iter()
            .find(|made| made.id == id)
            .map(|made| made.person.name.clone())
            .unwrap_or_default()
    }

    /// The dependencies among the issues of the goal.
    async fn blocks_among(
        &self,
        titles: &[(IssueId, String)],
        dump: &mut DumpTimeline,
    ) -> Result<(), SampleError> {
        let name = Message::from(message::RELATION_KIND_BLOCKS).render();
        let blocks = self
            .storage
            .relation_kinds()
            .await?
            .into_iter()
            .find(|kind| kind.forward == name)
            .map(|kind| kind.id);
        let title_of = |id: IssueId| {
            titles
                .iter()
                .find(|(each, _)| *each == id)
                .map(|(_, title)| title.clone())
        };
        for (issue, _) in titles {
            for relation in self.storage.relations_of(*issue).await? {
                if Some(relation.kind) != blocks || relation.from != *issue {
                    continue;
                }
                if let (Some(first), Some(waiting)) =
                    (title_of(relation.from), title_of(relation.to))
                {
                    dump.blocks.push((first, waiting));
                }
            }
        }
        dump.blocks.sort();
        Ok(())
    }

    /// The usual work days, the share in the project and the away times of the person who plans.
    async fn work_times_of(&self, name: &str, dump: &mut DumpTimeline) -> Result<(), SampleError> {
        name.clone_into(&mut dump.person.name);
        let Some(person) = self.people.iter().find(|made| made.person.name == name) else {
            return Ok(());
        };
        if let Some((_, days)) = self
            .storage
            .usual_days_of(vec![person.id])
            .await?
            .into_iter()
            .next()
        {
            dump.person.usual_days = days
                .days()
                .iter()
                .map(|day| (day.works, day.minutes))
                .collect();
        }
        dump.person.share_percent = self
            .storage
            .shares_of(vec![person.id])
            .await?
            .first()
            .map_or(0, |(_, _, share)| share.percent());
        dump.person.away_times = self
            .storage
            .away_times_of(
                vec![person.id],
                Date::constant(2000, 1, 1),
                Date::constant(2200, 1, 1),
                None,
            )
            .await?
            .len() as u64;
        Ok(())
    }

    /// The time factors of the issues of the goal that the person on the bar set herself.
    async fn own_estimates_among(
        &self,
        titles: &[(IssueId, String)],
        dump: &mut DumpTimeline,
    ) -> Result<(), SampleError> {
        let ids = titles.iter().map(|(id, _)| *id).collect();
        for factor in self.storage.time_factors_of(ids).await? {
            let title = titles
                .iter()
                .find(|(id, _)| *id == factor.issue)
                .map(|(_, title)| title.clone())
                .unwrap_or_default();
            dump.own_estimates.push(DumpOwnEstimate {
                issue: title,
                set_by_the_person: factor.set_by == factor.person,
            });
        }
        Ok(())
    }
}
