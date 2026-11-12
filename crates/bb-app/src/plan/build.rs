//! The plan of a set of issues the reader may see: their bars, worked out with the rules of
//! `bb_core::plan`, and their warnings. The timeline works out the plan of a page and the issues
//! around it; *Move it after* the plan of an issue and those that wait for it. Everything is read
//! once for the whole set (PERF-N1), and only what the reader may read counts: a date field they
//! may not read is empty, a time factor they may not read is ×1, an estimate they may not read
//! plans nothing.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::{IssueId, ProjectId, UserId};
use bb_core::issue::Issue;
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::{FieldValue, date_of};
use bb_core::plan::bar::{Bar, BarInput, bar, load, shortfall};
use bb_core::plan::record::IssueTimeFactor;
use bb_core::plan::warning::{Warning, away, delays, sub_issue_delays, too_much_at_once};
use bb_core::plan::{Days, TimeFactor};
use bb_core::relation::Dependency;
use bb_core::work_times::DayMinutes;
use bb_policy::{Evaluation, Place};
use jiff::ToSpan as _;
use jiff::civil::Date;

use super::factor::person_on_bar;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::Where;
use crate::relations::effect_kinds;
use crate::search::FieldTarget;
use crate::work_times::{PLAN_PEOPLE_MAX, RANGE_DAYS_MAX, WorkMinutesQuery};

/// The date fields the bars take their days from, as the reader may read them: a field the reader
/// may not read is none, and counts as empty (ADR-0043, decision 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DateTargets {
    /// Where the bars start.
    pub(crate) start: Option<FieldTarget>,
    /// Where they end.
    pub(crate) end: Option<FieldTarget>,
}

impl DateTargets {
    /// Whether a field the search reads can give a bar its day.
    pub(crate) const fn is_date(target: FieldTarget) -> bool {
        matches!(
            target,
            FieldTarget::StartDate
                | FieldTarget::DueDate
                | FieldTarget::Own {
                    kind: FieldType::Date,
                    ..
                }
        )
    }

    /// The own fields among them, whose values are read for the set.
    fn own(self) -> Vec<bb_core::id::FieldId> {
        [self.start, self.end]
            .into_iter()
            .flatten()
            .filter_map(|target| match target {
                FieldTarget::Own { field, .. } => Some(field),
                _ => None,
            })
            .collect()
    }
}

/// What a plan is worked out from.
pub(crate) struct PlanAsk<'a> {
    /// The issues the reader may see, each once.
    pub(crate) issues: Vec<Issue>,
    /// The date fields.
    pub(crate) dates: DateTargets,
    /// Whether *Plan by effort* is on.
    pub(crate) by_effort: bool,
    /// The days of the work times to read: bars are worked out inside them.
    pub(crate) window: Days,
    /// Who asks.
    pub(crate) actor: &'a Actor,
}

/// The days of work times one plan reads: from 120 days before the span, so a bar that started
/// before the screen is worked out too, up to a year and a day in all.
pub(crate) fn window_of(span: Days) -> Result<Days, AppError> {
    let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
    let length = Days::between(span.first, span.last);
    if length >= RANGE_DAYS_MAX {
        return Err(invalid());
    }
    let before = (RANGE_DAYS_MAX - 1 - length).min(LOOK_BACK_DAYS);
    let first = span
        .first
        .checked_sub(before.days())
        .map_err(|_| invalid())?;
    let last = first
        .checked_add((RANGE_DAYS_MAX - 1).days())
        .map_err(|_| invalid())?;
    Ok(Days { first, last })
}

/// How many days before the span the work times are read.
const LOOK_BACK_DAYS: i64 = 120;

/// The plan of a set of issues for the reader.
pub(crate) struct Plan {
    /// The issues, by ID.
    pub(crate) issues: HashMap<IssueId, Issue>,
    /// What the reader holds in the project of each issue.
    pub(crate) held: HashMap<ProjectId, Arc<Evaluation>>,
    /// The people each issue is assigned to, in order.
    pub(crate) assigned: HashMap<IssueId, Vec<UserId>>,
    /// The time factors that are not ×1.
    pub(crate) factors: HashMap<IssueId, IssueTimeFactor>,
    /// The bars.
    pub(crate) bars: HashMap<IssueId, Bar>,
    /// The days of each person on the bar in each project, as the reader may know them.
    pub(crate) days: HashMap<(ProjectId, UserId), Vec<DayMinutes>>,
    /// The people on a bar whose work times the reader may know in the project (the kind *Work
    /// times* is not *Hidden* for them): only theirs are hatched or named in a warning about how
    /// much they can work.
    pub(crate) work_times_known: HashSet<(ProjectId, UserId)>,
    /// The dependencies between the issues of the set.
    pub(crate) dependencies: Vec<Dependency>,
    /// Which issue of the set sits under which.
    pub(crate) parents: Vec<(IssueId, IssueId)>,
    /// The warnings, each on its issue.
    pub(crate) warnings: Vec<(IssueId, Warning)>,
}

impl Plan {
    /// Whether the reader may read `field` of `issue`.
    pub(crate) fn reads(&self, issue: &Issue, field: Field) -> bool {
        self.held
            .get(&issue.project)
            .is_some_and(|held| held.field_mask_for(Some(issue.issue_type)).can_see(field))
    }

    /// The person on the bar of an issue.
    pub(crate) fn person_on_bar(&self, issue: IssueId) -> Option<UserId> {
        self.assigned
            .get(&issue)
            .and_then(|people| person_on_bar(people))
    }

    /// Whether the reader may know the work times of the person on the bar of an issue.
    pub(crate) fn work_times_known(&self, issue: &Issue) -> bool {
        self.person_on_bar(issue.id)
            .is_some_and(|person| self.work_times_known.contains(&(issue.project, person)))
    }

    /// The days of the person on the bar of an issue.
    pub(crate) fn days_of(&self, issue: &Issue) -> &[DayMinutes] {
        self.person_on_bar(issue.id)
            .and_then(|person| self.days.get(&(issue.project, person)))
            .map_or(&[], Vec::as_slice)
    }

    /// The time factor of an issue as the reader may read it: ×1 when it is hidden from them.
    pub(crate) fn factor_of(&self, issue: &Issue) -> Option<&IssueTimeFactor> {
        self.factors
            .get(&issue.id)
            .filter(|_| self.reads(issue, Field::TimeFactor))
    }

    /// The effort of an issue under *Plan by effort*: its estimate times its time factor, when it
    /// has a person on the bar and an estimate the reader may read.
    fn effort_of(&self, issue: &Issue) -> Option<bb_core::time::Minutes> {
        self.person_on_bar(issue.id)?;
        let estimate = issue
            .estimated_time
            .filter(|_| self.reads(issue, Field::EstimatedTime))?;
        let factor = self
            .factor_of(issue)
            .map_or(TimeFactor::ONE, |kept| kept.factor);
        Some(factor.effort(estimate))
    }
}

/// A date of an issue from one of the date fields.
fn date_from(
    issue: &Issue,
    target: Option<FieldTarget>,
    values: &crate::fields::ItemValues,
) -> Option<Date> {
    match target? {
        FieldTarget::StartDate => issue.start_date,
        FieldTarget::DueDate => issue.due_date,
        FieldTarget::Own { field, .. } => match values.value(issue.id.into(), field)? {
            FieldValue::Day(days) => date_of(*days),
            _ => None,
        },
        _ => None,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issues with these IDs the reader may see (not in the trash, not restricted from them),
    /// with what they hold in each project: an issue they may not see is left out entirely.
    pub(crate) async fn seen_issues(
        &self,
        actor: &Actor,
        ids: &[IssueId],
    ) -> Result<Vec<(Issue, Arc<Evaluation>)>, AppError> {
        let found = self
            .store()
            .issues_by_ids(ids)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut seen = Vec::with_capacity(found.len());
        for issue in found {
            if let Some(held) = self.seen_in(actor, &issue, Where::Live).await? {
                seen.push((issue, held));
            }
        }
        Ok(seen)
    }

    /// The dependencies and the sub-issues around `issues`, as the IDs of the issues at their
    /// other ends: what a plan reads next to them.
    pub(crate) async fn around(
        &self,
        issues: &[IssueId],
    ) -> Result<(Vec<Dependency>, Vec<(IssueId, IssueId)>), AppError> {
        let failed = |error: S::Error| internal::<S>(&error);
        let kinds = effect_kinds(&self.store().relation_kinds().await.map_err(failed)?);
        let dependencies = self
            .store()
            .dependencies_touching(kinds, issues.to_vec())
            .await
            .map_err(failed)?;
        let parents = self
            .store()
            .parents_touching(issues.to_vec())
            .await
            .map_err(failed)?
            .into_iter()
            .map(|row| (row.child, row.parent))
            .collect();
        Ok((dependencies, parents))
    }

    /// The plan of the issues of `ask`: the people on their bars and their work times, read once
    /// for the whole set; the bars; and the warnings among them. `dependencies` and `parents` may
    /// name issues outside the set: those are not checked (an issue the reader may not see is never
    /// in the set).
    pub(crate) async fn work_out_plan(
        &self,
        ask: PlanAsk<'_>,
        dependencies: &[Dependency],
        parents: &[(IssueId, IssueId)],
    ) -> Result<Plan, AppError> {
        let failed = |error: S::Error| internal::<S>(&error);
        let ids: Vec<IssueId> = ask.issues.iter().map(|issue| issue.id).collect();
        let in_set: HashSet<IssueId> = ids.iter().copied().collect();
        let mut held = HashMap::new();
        for issue in &ask.issues {
            if let std::collections::hash_map::Entry::Vacant(slot) = held.entry(issue.project) {
                slot.insert(
                    self.evaluation_of(ask.actor, Place::Project(issue.project))
                        .await?,
                );
            }
        }
        let mut assigned: HashMap<IssueId, Vec<UserId>> = HashMap::new();
        for row in self
            .store()
            .assigned_people_of(&ids)
            .await
            .map_err(failed)?
        {
            assigned.entry(row.issue).or_default().push(row.person);
        }
        let factors = self
            .store()
            .time_factors_of(ids.clone())
            .await
            .map_err(failed)?
            .into_iter()
            .map(|factor| (factor.issue, factor))
            .collect();
        let own = ask.dates.own();
        let values = if own.is_empty() {
            crate::fields::ItemValues::default()
        } else {
            self.load_values(
                bb_core::event::RecordKind::Issue,
                &ids.iter().map(|id| (*id).into()).collect::<Vec<_>>(),
            )
            .await?
        };
        let mut plan = Plan {
            issues: ask
                .issues
                .into_iter()
                .map(|issue| (issue.id, issue))
                .collect(),
            held,
            assigned,
            factors,
            bars: HashMap::new(),
            days: HashMap::new(),
            work_times_known: HashSet::new(),
            dependencies: dependencies
                .iter()
                .copied()
                .filter(|(first, then)| in_set.contains(first) && in_set.contains(then))
                .collect(),
            parents: parents
                .iter()
                .copied()
                .filter(|(child, parent)| in_set.contains(child) && in_set.contains(parent))
                .collect(),
            warnings: Vec::new(),
        };
        self.read_days(&mut plan, ask.window).await?;
        // The bars.
        let mut bars = HashMap::new();
        for issue in plan.issues.values() {
            let reads = |field| plan.reads(issue, field);
            let start = date_from(issue, ask.dates.start, &values).filter(|_| {
                ask.dates.start != Some(FieldTarget::StartDate) || reads(Field::StartDate)
            });
            let end = date_from(issue, ask.dates.end, &values)
                .filter(|_| ask.dates.end != Some(FieldTarget::DueDate) || reads(Field::DueDate));
            let effort = plan.effort_of(issue).filter(|_| ask.by_effort);
            if let Some(found) = bar(&BarInput {
                start,
                due: end,
                effort,
                days: plan.days_of(issue),
            }) {
                bars.insert(issue.id, found);
            }
        }
        plan.bars = bars;
        plan.warnings = warnings_of(&plan, ask.by_effort);
        Ok(plan)
    }

    /// The days of every person on a bar of the plan, in each project, through the one port of
    /// work times: a fixed number of reads for each project, whatever the number of people.
    async fn read_days(&self, plan: &mut Plan, window: Days) -> Result<(), AppError> {
        let mut wanted: HashMap<ProjectId, Vec<UserId>> = HashMap::new();
        for issue in plan.issues.values() {
            if let Some(person) = plan.person_on_bar(issue.id) {
                let people = wanted.entry(issue.project).or_default();
                if !people.contains(&person) {
                    people.push(person);
                }
            }
        }
        for (project, people) in wanted {
            let Some(reader) = plan.held.get(&project).cloned() else {
                continue;
            };
            for chunk in people.chunks(PLAN_PEOPLE_MAX) {
                let minutes = self
                    .work_minutes(
                        &reader,
                        WorkMinutesQuery {
                            people: chunk.to_vec(),
                            project,
                            first: window.first,
                            last: window.last,
                        },
                    )
                    .await?;
                for person in minutes.people {
                    if person.away_visible {
                        plan.work_times_known.insert((project, person.person));
                    }
                    plan.days.insert((project, person.person), person.days);
                }
            }
        }
        Ok(())
    }
}

/// The minutes each issue of one person on a bar plans on each day.
type Loads = Vec<(IssueId, Vec<(Date, u32)>)>;

/// The warnings of a plan: *Away* and *Not enough days* of each bar, *Delay* along its
/// dependencies and of its goals, and *Too much at once* for each person on a bar in each project.
fn warnings_of(plan: &Plan, by_effort: bool) -> Vec<(IssueId, Warning)> {
    let mut found = Vec::new();
    let mut loads: HashMap<(ProjectId, UserId), Loads> = HashMap::new();
    for (id, bar_of) in &plan.bars {
        let Some(issue) = plan.issues.get(id) else {
            continue;
        };
        let days = plan.days_of(issue);
        found.extend(away(bar_of, days).map(|warning| (*id, warning)));
        if !by_effort {
            continue;
        }
        let Some(effort) = plan.effort_of(issue) else {
            continue;
        };
        found.extend(
            shortfall(bar_of, effort, days).map(|short| (*id, Warning::NotEnoughDays(short))),
        );
        if let Some(person) = plan.person_on_bar(*id) {
            loads
                .entry((issue.project, person))
                .or_default()
                .push((*id, load(bar_of, effort, days)));
        }
    }
    found.extend(delays(&plan.dependencies, &plan.bars));
    // The due date of a goal, as the reader may read it: only the field *Due date* is a goal's due
    // date, whatever date fields the view uses.
    let dues: HashMap<IssueId, Date> = plan
        .issues
        .values()
        .filter(|issue| plan.reads(issue, Field::DueDate))
        .filter_map(|issue| issue.due_date.map(|due| (issue.id, due)))
        .collect();
    found.extend(sub_issue_delays(&plan.parents, &plan.bars, &dues));
    // *Too much at once* says how much a person can work in the project: only to a reader who may
    // know their work times.
    for ((project, person), mut issue_loads) in loads
        .into_iter()
        .filter(|(at, _)| plan.work_times_known.contains(at))
    {
        issue_loads.sort_by_key(|(id, _)| *id);
        let days = plan
            .days
            .get(&(project, person))
            .map_or(&[][..], Vec::as_slice);
        found.extend(too_much_at_once(&issue_loads, days));
    }
    found.sort_by_key(|(id, warning)| (*id, warning.kind()));
    found
}
