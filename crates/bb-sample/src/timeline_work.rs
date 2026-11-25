//! A goal with people in the sample (M8 part 7, PLAN-01, PLAN-11 to PLAN-15): *Mobile app 2.0* with
//! a due date, its issues with start and due dates, estimates and dependencies of *blocks*, three
//! people who work on it, the usual work days and work hours of one of them, her share in the
//! project, the public holidays of one German state, no away time yet and one issue with her own
//! estimate (*Takes me ...*).
//!
//! All of it goes through the use cases the screens call, one change a second of the sample's clock.
//! Days are counted from the first Monday after the end of the sample, so the plan is ahead of the
//! day the sample was made on and starts on a work day.

use bb_app::issues::{CreateIssue, IssueQuery, create_issue, issue};
use bb_app::plan::{FactorGiven, SetTimeFactor, set_time_factor};
use bb_app::relations::{AddRelation, add_relation};
use bb_app::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_app::work_times::{ChangeUsualDays, SetShare, change_usual_days, set_share};
use bb_core::id::{IssueId, IssueTypeId, UserId};
use bb_core::issue::IssueIcon;
use bb_core::relation::Direction;
use bb_core::settings::SettingValue;
use bb_core::settings::work_times::WORK_TIMES_HOLIDAYS;
use bb_core::time::Minutes;
use bb_core::work_times::UsualDay;
use bb_dev_words::sample::{PlannedIssue, TimelinePlan};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use crate::error::{SampleError, Step};
use crate::generate::ADMINISTRATOR;
use crate::issues::{MadeIssue, Maker};
use crate::issues_plan::{kind_of, starting_name};
use crate::issues_setup::json;

/// The place of the starting kind *blocks* among the kinds, which are offered in this order.
const BLOCKS: usize = 0;
/// The days of a week.
const WEEK_DAYS: usize = 7;
/// The same, to count dates with.
const DAYS_IN_WEEK: i64 = 7;

/// The first Monday after the end of the sample: the day the plan counts its days from.
fn first_monday_after(end: Timestamp) -> Result<Date, SampleError> {
    let today = end.to_zoned(TimeZone::UTC).date();
    let ahead = DAYS_IN_WEEK - i64::from(today.weekday().to_monday_zero_offset());
    today
        .checked_add(Span::new().days(ahead))
        .map_err(|_| SampleError::Start(ahead))
}

/// A day of the plan as a date.
fn day_of(monday: Date, day: i64) -> Result<Date, SampleError> {
    monday
        .checked_add(Span::new().days(day))
        .map_err(|_| SampleError::Start(day))
}

fn failed(step: Step, error: &dyn std::fmt::Display) -> SampleError {
    SampleError::step(step, &error)
}

impl Maker<'_, '_> {
    /// The place of a person in the sample, by name.
    fn person_named(&self, name: &str) -> Result<usize, SampleError> {
        self.world
            .people
            .iter()
            .position(|made| made.person.name == name)
            .ok_or_else(|| SampleError::NotMade(name.to_owned()))
    }

    /// The ID of the type with this name.
    fn type_named(&self, name: &str) -> Result<IssueTypeId, SampleError> {
        kind_of(&self.names.types, name)
            .and_then(|at| self.setup.types.get(at).copied())
            .ok_or_else(|| SampleError::NotMade(name.to_owned()))
    }

    /// Makes the goal with its people, dates, dependencies and work times (see the module).
    pub(crate) async fn plan_the_goal(&mut self, plan: &TimelinePlan) -> Result<(), SampleError> {
        let administrator = self.world.session_of(ADMINISTRATOR).await?;
        let lea = self.person_named(&plan.person)?;
        let monday = first_monday_after(self.world.start)?;
        self.holidays_of(&administrator, &plan.holidays).await?;
        let goal_due = day_of(monday, plan.goal.due_day)?;
        let goal = self
            .create_one(
                &administrator,
                &plan.project,
                self.type_named(&starting_name(IssueIcon::Flag))?,
                (&plan.goal.title, None, Some(goal_due)),
                None,
            )
            .await?;
        let task = self.type_named(&starting_name(IssueIcon::SquareCheck))?;
        let mut made = Vec::new();
        for planned in &plan.issues {
            let person = self.world.people[self.person_named(&planned.person)?].id;
            let issue = self
                .create_planned(&administrator, plan, planned, (task, person, monday), &goal)
                .await?;
            made.push((planned.title.as_str(), issue));
        }
        for [first, waiting] in &plan.blocks {
            let find = |title: &str| {
                made.iter()
                    .find(|(each, _)| *each == title)
                    .map(|(_, issue)| issue.id)
                    .ok_or_else(|| SampleError::NotMade(title.to_owned()))
            };
            self.block(&administrator, find(first)?, find(waiting)?)
                .await?;
        }
        self.work_times_of(&administrator, plan, lea).await?;
        let lea_token = self.world.session_of(lea).await?;
        let own = made
            .iter()
            .find(|(title, _)| *title == plan.own_estimate.issue)
            .map(|(_, issue)| issue.id)
            .ok_or_else(|| SampleError::NotMade(plan.own_estimate.issue.clone()))?;
        self.takes_me(&lea_token, own, plan.own_estimate.takes_minutes)
            .await
    }

    /// The public holidays the organization counts.
    async fn holidays_of(&self, token: &str, holidays: &str) -> Result<(), SampleError> {
        self.world.tick();
        change_setting(
            &self.world.services.ctx(self.world.request()),
            ChangeSetting {
                version: None,
                session_token: token.to_owned(),
                scope: SettingsScope::Organization,
                key: WORK_TIMES_HOLIDAYS.definition().key().to_owned(),
                value: SettingValue::Text(holidays.to_owned()),
            },
        )
        .await
        .map(drop)
        .map_err(|error| failed(Step::Settings, &error))
    }

    /// Makes an issue: its title, start date and due date, and for an issue of the goal the person it is
    /// assigned to, its estimate and the goal it is under.
    async fn create_one(
        &self,
        token: &str,
        project: &str,
        issue_type: IssueTypeId,
        (title, start, due): (&str, Option<Date>, Option<Date>),
        more: Option<(UserId, Minutes, &MadeIssue)>,
    ) -> Result<MadeIssue, SampleError> {
        self.world.tick();
        let (assigned_to, estimated_time, parent) = match more {
            Some((person, estimate, goal)) => (
                Some(vec![person]),
                Some(estimate),
                Some(goal.id.to_string()),
            ),
            None => (None, None, None),
        };
        let made = json(
            create_issue(
                &self.world.services.ctx(self.world.request()),
                CreateIssue {
                    session_token: token.to_owned(),
                    project: project.to_owned(),
                    issue_type,
                    title: title.to_owned(),
                    description: None,
                    status: None,
                    priority: None,
                    assigned_to,
                    start_date: start,
                    due_date: due,
                    estimated_time,
                    fields: std::collections::BTreeMap::new(),
                    parent,
                    found_in_release: None,
                    planned_for_release: None,
                },
            )
            .await,
            Step::Issue,
        )?;
        MadeIssue::of(&made)
    }

    /// An issue of the plan, under the goal.
    async fn create_planned(
        &self,
        token: &str,
        plan: &TimelinePlan,
        planned: &PlannedIssue,
        (task, person, monday): (IssueTypeId, UserId, Date),
        goal: &MadeIssue,
    ) -> Result<MadeIssue, SampleError> {
        self.create_one(
            token,
            &plan.project,
            task,
            (
                &planned.title,
                Some(day_of(monday, planned.start_day)?),
                planned.due_day.map(|day| day_of(monday, day)).transpose()?,
            ),
            Some((person, Minutes::new(planned.estimate), goal)),
        )
        .await
    }

    /// Makes `first` block `waiting`: `first` must finish first.
    async fn block(
        &self,
        token: &str,
        first: IssueId,
        waiting: IssueId,
    ) -> Result<(), SampleError> {
        let kind = *self
            .setup
            .kinds
            .get(BLOCKS)
            .ok_or_else(|| SampleError::NotMade("blocks".to_owned()))?;
        self.world.tick();
        json(
            add_relation(
                &self.world.services.ctx(self.world.request()),
                AddRelation {
                    session_token: token.to_owned(),
                    issue: first.to_string(),
                    other: waiting.to_string(),
                    kind,
                    direction: Direction::Forward,
                },
            )
            .await,
            Step::Relation,
        )
        .map(drop)
    }

    /// The usual work days and work hours of the person who plans, and her share in the project.
    async fn work_times_of(
        &self,
        administrator: &str,
        plan: &TimelinePlan,
        person: usize,
    ) -> Result<(), SampleError> {
        let token = self.world.session_of(person).await?;
        let days: [UsualDay; WEEK_DAYS] = std::array::from_fn(|at| UsualDay {
            works: plan.usual_days.get(at).copied().unwrap_or(false),
            minutes: plan.usual_minutes,
        });
        self.world.tick();
        change_usual_days(
            &self.world.services.ctx(self.world.request()),
            ChangeUsualDays {
                session_token: token,
                person: None,
                version: 0,
                days,
            },
        )
        .await
        .map_err(|error| failed(Step::Time, &error))?;
        self.world.tick();
        set_share(
            &self.world.services.ctx(self.world.request()),
            SetShare {
                session_token: administrator.to_owned(),
                project: plan.project.clone(),
                person: self.world.people[person].id,
                percent: plan.share_percent,
            },
        )
        .await
        .map(drop)
        .map_err(|error| failed(Step::Time, &error))
    }

    /// The person on the bar says how long the issue takes her.
    async fn takes_me(
        &self,
        token: &str,
        issue_id: IssueId,
        minutes: u32,
    ) -> Result<(), SampleError> {
        let current = json(
            issue(
                &self.world.services.ctx(self.world.request()),
                IssueQuery {
                    session_token: token.to_owned(),
                    issue: issue_id.to_string(),
                },
            )
            .await,
            Step::Change,
        )?;
        self.world.tick();
        set_time_factor(
            &self.world.services.ctx(self.world.request()),
            SetTimeFactor {
                session_token: token.to_owned(),
                issue: issue_id.to_string(),
                version: current["issue"]["version"]
                    .as_i64()
                    .ok_or_else(|| SampleError::NotMade(issue_id.to_string()))?,
                given: FactorGiven::Takes(Minutes::new(minutes)),
            },
        )
        .await
        .map(drop)
        .map_err(|error| failed(Step::Time, &error))
    }
}
