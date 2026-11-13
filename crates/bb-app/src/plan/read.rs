//! A page of the timeline (PLAN-01, PLAN-04, PLAN-08, PLAN-10, PLAN-13, ADR-0030 amendment M8,
//! ADR-0043): the issues of a search that have a date the reader may read, a page at a time, each
//! with its bar, the dependencies between them and their warnings, and the count of *Not scheduled
//! yet*, whose issues come from the same use case.
//!
//! The issues are read through the compiled search, so restricted issues are left out entirely: no
//! bar, no arrow, no warning, no count. Issues around the page (the other end of a dependency, the
//! goal of a sub-issue, the sub-issues of a goal) count for the warnings of the page only when the
//! reader may see them.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::fields::{ASSIGNED_TO_PEOPLE, Field, word_key};
use bb_core::id::IssueId;
use bb_core::issue::Issue;
use bb_core::own_fields::FieldType;
use bb_core::people_kinds::PeopleKind;
use bb_core::plan::Days;
use bb_core::plan::warning::Warning;
use bb_core::saved_search::TimelineOptions;
use bb_policy::{Authorized, Evaluation, PeopleContext, PersonRef, Place as Where};
use bb_searchlang::{Expr, FieldRef, Span, Test};

use super::build::{DateTargets, Plan, PlanAsk, window_of};
use super::factor::names_person;
use super::person_days::hatched_of;
use super::types::{
    AcceptedView, BarView, EffortShown, FactorShown, GoalView, HatchedDay, TimelinePage,
    TimelinePart, TimelineQuery, TimelineRow, TimelineSearch, WarningView, WorkTimeShown,
};
use crate::access::{Actor, place_of_address};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::ReadOnRequest;
use crate::search::{
    FieldTarget, GroupKey, Looked, Prepared, SearchScope, SearchStore, ShownKey, stopped, target_of,
};

/// The two date fields of a view by their search words: its own, or *Start date* and *Due date*.
pub(crate) fn date_words(options: &TimelineOptions) -> (String, String) {
    options
        .dates
        .clone()
        .unwrap_or_else(|| (Field::StartDate.search_word(), Field::DueDate.search_word()))
}

/// The date fields of a view as the search reads them for the reader.
pub(crate) fn date_targets(looked: &Looked, words: &(String, String)) -> DateTargets {
    DateTargets {
        start: DateTargets::of_word(&looked.fields, &words.0),
        end: DateTargets::of_word(&looked.fields, &words.1),
    }
}

/// `<word> is not empty`, or `<word> is empty`.
fn filled(word: &str, has_value: bool) -> Expr {
    let span = Span::default();
    Expr::Field {
        field: FieldRef {
            word: word.to_owned(),
            span,
        },
        test: if has_value {
            Test::IsNotEmpty { span }
        } else {
            Test::IsEmpty { span }
        },
        span,
    }
}

/// The filter of a part of the timeline over the readable date fields: a row has one of them, an
/// issue of *Not scheduled yet* has none. `None` for no filter; `nothing` is set when no row can
/// have a date the reader may read.
fn narrowing(
    looked: &Looked,
    words: &(String, String),
    part: TimelinePart,
    nothing: &AtomicBool,
) -> Option<Expr> {
    let targets = date_targets(looked, words);
    let readable: Vec<&str> = [(targets.start, &words.0), (targets.end, &words.1)]
        .into_iter()
        .filter_map(|(target, word)| target.map(|_| word.as_str()))
        .collect();
    let has_value = part == TimelinePart::Rows;
    let mut tests: Vec<Expr> = readable
        .iter()
        .map(|word| filled(word, has_value))
        .collect();
    match (tests.len(), has_value) {
        (0, true) => {
            nothing.store(true, Ordering::Relaxed);
            None
        }
        (0, false) => None,
        (1, _) => tests.pop(),
        (_, true) => Some(Expr::Or {
            items: tests,
            span: Span::default(),
        }),
        (_, false) => Some(Expr::And {
            items: tests,
            span: Span::default(),
        }),
    }
}

/// The hatched days of a bar inside the span, as *Who sees people* (the kind *Work times*) allows:
/// none at all for a reader who may not know the person's work times or may not read who the issue
/// is assigned to, and without a word for a reader who does not see who the person is (`named`).
fn hatched_days(
    plan: &Plan,
    issue: &Issue,
    (days, span): (Days, Days),
    named: bool,
) -> Vec<HatchedDay> {
    if !plan.work_times_known(issue) || !plan.reads(issue, Field::AssignedTo) {
        return Vec::new();
    }
    plan.days_of(issue)
        .iter()
        .filter(|day| days.contains(day.date) && span.contains(day.date))
        .filter_map(|day| hatched_of(day, true, named))
        .collect()
}

/// The effort of a bar as its card shows it: the estimate, the effort with the time factor the
/// reader may read, and, when the bars are planned by effort, the work days it takes from the
/// first day of the bar.
fn effort_shown(plan: &Plan, issue: &Issue, days: Days) -> Option<EffortShown> {
    let effort = plan.effort_of(issue)?;
    Some(EffortShown {
        estimate: issue.estimated_time?,
        effort,
        work_days: plan
            .by_effort
            .then(|| bb_core::plan::bar::effort_days(days.first, effort, plan.days_of(issue)))
            .flatten(),
    })
}

/// What a warning tells of its days and its ways out.
struct Told {
    days: Days,
    ways_out: Vec<bb_core::plan::warning::WayOut>,
    move_after: Option<jiff::civil::Date>,
    due_needed: Option<jiff::civil::Date>,
}

/// What a warning may tell the reader: all of it, or for *Away* about a person the reader does not
/// see by name only that the assigned person is away (UX §4.7, *People on the bars*). Then its days
/// are the bar's, not the away days, and no way out tells them either: *Move it after* would start
/// after the away days and *Make it longer* would end around them, so only *Keep and accept* is left.
fn told_of(plan: &Plan, issue: &Issue, warning: &Warning, (days, named): (Days, bool)) -> Told {
    if named || !matches!(warning, Warning::Away { .. }) {
        return Told {
            days,
            ways_out: warning.ways_out(),
            move_after: warning.move_after(),
            due_needed: warning.due_needed(),
        };
    }
    Told {
        days: plan.bars.get(&issue.id).map_or(days, |bar| bar.days),
        ways_out: vec![bb_core::plan::warning::WayOut::KeepAndAccept],
        move_after: None,
        due_needed: None,
    }
}

/// The days a warning is about.
fn warning_days(plan: &Plan, issue: &Issue, warning: &Warning) -> Option<Days> {
    use bb_core::plan::warning::Delay;
    match warning {
        Warning::Away { days, .. } | Warning::TooMuchAtOnce { days, .. } => Some(*days),
        Warning::Delay(Delay::SubIssue { due_needed, .. }) => {
            issue.due_date.and_then(|due| Days::new(due, *due_needed))
        }
        Warning::NotEnoughDays(_) | Warning::Delay(Delay::Dependency { .. }) => {
            plan.bars.get(&issue.id).map(|bar| bar.days)
        }
    }
}

/// The field the rows are grouped by. *Assigned to* is no field of the search for a reader who sees
/// people by their role or not at all: then nobody is told apart, and every row is in the group of
/// no value (ADR-0030, decision 1).
/// A word that is no field the reader may group by is no request.
fn group_target(looked: &Looked, word: &str) -> Result<Option<FieldTarget>, AppError> {
    match target_of(&looked.fields, word) {
        Some(target) if timeline_groups_by(target) => Ok(Some(target)),
        // Without names *Assigned to* is no field of the search: its rows are all in the group of
        // no value (`groups_of`).
        None if word_key(word) == word_key(&Field::AssignedTo.search_word()) => {
            Ok(Some(FieldTarget::AssignedTo))
        }
        Some(_) | None => Err(AppError::Refused(ErrorCode::InvalidRequest)),
    }
}

/// Whether a timeline groups its rows by a field (UX §4.7): the project, the goal, the person, the
/// status, the priority or an own field of choices. The one rule of the timeline's grouping: the
/// readable fields say it to the browser, and a page refuses any other field.
#[must_use]
pub fn timeline_groups_by(target: FieldTarget) -> bool {
    matches!(
        target,
        FieldTarget::Project
            | FieldTarget::Parent
            | FieldTarget::AssignedTo
            | FieldTarget::Status
            | FieldTarget::Priority
            | FieldTarget::Own {
                kind: FieldType::PickOne | FieldType::PickSeveral,
                ..
            }
    )
}

/// The groups of an issue by an own field of choices: one for each choice it has, or the group
/// without a value.
fn choice_groups(
    view: &crate::issues::IssueView,
    field: bb_core::id::FieldId,
) -> Vec<(Option<ShownKey>, bool)> {
    let chosen: Vec<GroupKey> = match view.own_fields.0.get(&field.to_string()) {
        Some(serde_json::Value::String(id)) => vec![id.as_str()],
        Some(serde_json::Value::Array(ids)) => ids.iter().filter_map(|id| id.as_str()).collect(),
        _ => Vec::new(),
    }
    .into_iter()
    .filter_map(|id| id.parse().ok().map(GroupKey::Choice))
    .collect();
    if chosen.is_empty() {
        return vec![(Some(ShownKey::Other(GroupKey::Empty)), false)];
    }
    chosen
        .into_iter()
        .map(|key| (Some(ShownKey::Other(key)), false))
        .collect()
}

/// The goals of the page: the goals of its issues and the issues of it that are goals, each with
/// its due date as the reader may read it (only *Due date* is a goal's due date, whatever date
/// fields the view uses) and the span of the bars of its sub-issues that the plan knows. The plan
/// holds only issues the reader sees, so a hidden goal or sub-issue counts for nothing.
fn goals_of(plan: &Plan, page: &HashSet<IssueId>) -> Vec<GoalView> {
    let mut goals: Vec<IssueId> = plan
        .parents
        .iter()
        .filter(|(child, parent)| page.contains(child) || page.contains(parent))
        .map(|(_, parent)| *parent)
        .collect();
    goals.sort_unstable();
    goals.dedup();
    goals
        .into_iter()
        .filter_map(|id| {
            let goal = plan.issues.get(&id)?;
            let span = plan
                .parents
                .iter()
                .filter(|(_, parent)| *parent == id)
                .filter_map(|(child, _)| plan.bars.get(child).map(|bar| bar.days))
                .reduce(|all, days| Days {
                    first: all.first.min(days.first),
                    last: all.last.max(days.last),
                });
            Some(GoalView {
                issue: id,
                title: plan.reads(goal, Field::Title).then(|| goal.title.clone()),
                due: goal.due_date.filter(|_| plan.reads(goal, Field::DueDate)),
                span,
            })
        })
        .collect()
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The text and the place of a timeline's search.
    async fn timeline_search(
        &self,
        actor: &Actor,
        query: &TimelineQuery,
    ) -> Result<(String, SearchScope), AppError> {
        let text = match &query.search {
            TimelineSearch::Text(text) => text.clone(),
            TimelineSearch::Saved(id) => self.open_search(actor, *id).await?.stored.search.text,
        };
        let scope = match &query.project {
            None => SearchScope::Everywhere,
            Some(address) => {
                let found = self.find_project(address).await?;
                match place_of_address(found.as_ref().map(|found| found.project.id))? {
                    Where::Project(project) => SearchScope::Within(project),
                    Where::Organization => return Err(AppError::Refused(ErrorCode::NotFound)),
                }
            }
        };
        Ok((text, scope))
    }

    /// The search of one part of the timeline, checked and narrowed; `None` when no issue can be
    /// in it for the reader.
    async fn part_search(
        &self,
        actor: &Actor,
        (text, scope): (&str, SearchScope),
        words: &(String, String),
        part: TimelinePart,
    ) -> Result<Option<Prepared>, AppError> {
        let nothing = AtomicBool::new(false);
        let prepared = self
            .prepare_search_narrowed(actor, text, scope, None, |looked| {
                narrowing(looked, words, part, &nothing)
            })
            .await?;
        // What the search read of the person's rights decides every project of the plan too.
        self.remember_projects(&prepared.looked.seen);
        Ok((!nothing.load(Ordering::Relaxed)).then_some(prepared))
    }

    /// The groups of a row of `issue`, with whether each is a copy of the bar of the person on the
    /// bar (*Group by person*). One row with no group when the rows are not grouped, or when the
    /// reader may not see people by name (no grouping by person with *Role only*). An own field of
    /// choices gives a row for each choice the issue has, as the reader sees its values.
    async fn groups_of(
        &self,
        plan: &Plan,
        (held, view): (&Evaluation, &crate::issues::IssueView),
        issue: &Issue,
        by: Option<FieldTarget>,
    ) -> Result<Vec<(Option<ShownKey>, bool)>, AppError> {
        let one = |key: Option<GroupKey>| vec![(key.map(ShownKey::Other), false)];
        Ok(match by {
            Some(FieldTarget::Own {
                field,
                kind: FieldType::PickOne | FieldType::PickSeveral,
            }) => choice_groups(view, field),
            None => one(None),
            Some(FieldTarget::Project) => one(Some(GroupKey::Project(issue.project))),
            // A goal the reader may not see is no goal: the plan holds only issues they see.
            Some(FieldTarget::Parent) => one(Some(
                plan.parents
                    .iter()
                    .find(|(child, _)| *child == issue.id)
                    .map_or(GroupKey::Empty, |(_, parent)| GroupKey::Issue(*parent)),
            )),
            Some(FieldTarget::Priority) => one(Some(GroupKey::Priority(issue.priority))),
            Some(FieldTarget::Status) => {
                one(Some(GroupKey::Text(issue.status.as_str().to_owned())))
            }
            Some(FieldTarget::AssignedTo) => {
                let people = plan.assigned.get(&issue.id).cloned().unwrap_or_default();
                let by_name = held.people_level(ASSIGNED_TO_PEOPLE, PeopleContext::Ordinary)
                    == PeopleLevel::Names;
                // Without names (*Role only*) nobody can be told apart: every row is in the group
                // of no value, as a row without a person.
                if people.is_empty() || !by_name {
                    return Ok(one(Some(GroupKey::Empty)));
                }
                let mut rows = Vec::with_capacity(people.len());
                for (at, person) in people.iter().enumerate() {
                    let shown = self
                        .person_ref(held, ASSIGNED_TO_PEOPLE, PeopleContext::Ordinary, *person)
                        .await?;
                    rows.push((Some(ShownKey::Person(shown)), at > 0));
                }
                rows
            }
            Some(_) => return Err(AppError::Refused(ErrorCode::InvalidRequest)),
        })
    }

    /// The rows of the issues of a page: each with its bar (none in *Not scheduled yet*), once for
    /// each of its groups, every row showing the issue as the reader may see it.
    async fn timeline_rows(
        &self,
        plan: &Plan,
        issues: &[Issue],
        items: Vec<(std::sync::Arc<Evaluation>, crate::issues::IssueView)>,
        (group_by, dates): (Option<FieldTarget>, DateTargets),
        (part, span): (TimelinePart, Days),
    ) -> Result<Vec<TimelineRow>, AppError> {
        let mut rows = Vec::with_capacity(items.len());
        for (issue, (held, view)) in issues.iter().zip(items) {
            let bar = if part == TimelinePart::Rows {
                self.bar_view(plan, &held, issue, &view, span).await?
            } else {
                None
            };
            // Own date fields need the scopes of the own fields; built-in ones only the levels.
            let changeable = if dates.own().is_empty() {
                crate::issues::changeable_built_in(&held, issue)
            } else {
                self.changeable_fields(&held, issue).await?
            };
            let may_move = dates.changeable(&changeable);
            for (group, copy) in self
                .groups_of(plan, (&held, &view), issue, group_by)
                .await?
            {
                rows.push(TimelineRow {
                    group,
                    issue: held.authorize(view.clone()),
                    bar: bar.clone(),
                    copy,
                    may_move,
                });
            }
        }
        Ok(rows)
    }

    /// The bar of a row as the reader may see it.
    async fn bar_view(
        &self,
        plan: &Plan,
        held: &Evaluation,
        issue: &Issue,
        view: &crate::issues::IssueView,
        span: Days,
    ) -> Result<Option<BarView>, AppError> {
        let Some(bar) = plan.bars.get(&issue.id) else {
            return Ok(None);
        };
        let reads = |field| plan.reads(issue, field);
        let done_group =
            reads(Field::Status) && view.status.group == bb_core::issue::StatusGroup::Done;
        let done = bb_core::plan::bar::done_part(
            issue.estimated_time.filter(|_| reads(Field::EstimatedTime)),
            view.time_left
                .map(bb_core::time::TimeLeft::minutes)
                .filter(|_| reads(Field::TimeLeft)),
            done_group,
        );
        let person = match plan
            .person_on_bar(issue.id)
            .filter(|_| reads(Field::AssignedTo))
        {
            Some(person) => self.shown_person(held, person).await?,
            None => None,
        };
        // One rule for the bar, its warnings and its factor: the reader sees the person by name or
        // is the person; by role only or not at all, every day without work looks the same.
        let named = names_person(held, plan.person_on_bar(issue.id), person.as_ref());
        let factor = plan.factor_of(issue).map(|kept| FactorShown {
            percent: kept.factor.percent(),
            // Whose estimate it is is told only as openly as the person on the bar.
            own_estimate: named && kept.set_by == kept.person,
        });
        Ok(Some(BarView {
            days: bar.days,
            worked_out: bar.worked_out,
            done,
            person,
            factor,
            hatched: hatched_days(plan, issue, (bar.days, span), named),
            effort: effort_shown(plan, issue, bar.days),
            // The person's time tells whose it is: only to a reader who may read *Assigned to*.
            work_time: plan
                .time_of(issue)
                .filter(|_| reads(Field::AssignedTo))
                .map(|(usual, share)| WorkTimeShown { usual, share }),
        }))
    }

    /// The accepted warnings of the issues of the page, with who accepted them and the role labels
    /// of those the reader sees by their role, each read at once for the page.
    async fn accepted_on_page(
        &self,
        plan: &Plan,
        page: &HashSet<IssueId>,
        seen: &crate::projects::Leveled,
    ) -> Result<Vec<bb_core::plan::record::AcceptedWarning>, AppError> {
        let accepted = self
            .store()
            .accepted_warnings_of(page.iter().copied().collect())
            .await
            .map_err(|error| internal::<S>(&error))?;
        let accepters: Vec<_> = accepted.iter().map(|kept| kept.accepted_by).collect();
        self.preload_people(&accepters).await?;
        let by_role: Vec<(bb_core::id::UserId, Where)> = accepted
            .iter()
            .filter_map(|kept| {
                let project = plan.issues.get(&kept.issue)?.project;
                plan.held
                    .get(&project)?
                    .shows_role_label(
                        PeopleKind::Authors,
                        PeopleContext::Ordinary,
                        kept.accepted_by,
                    )
                    .then_some((kept.accepted_by, Where::Project(project)))
            })
            .collect();
        self.preload_role_labels(seen, &by_role).await?;
        Ok(accepted)
    }

    /// The warnings of the issues of the page, each with what it names as the reader may know it,
    /// and faded when it was accepted for the dates its issue has.
    async fn warning_views(
        &self,
        plan: &Plan,
        page: &HashSet<IssueId>,
        seen: &crate::projects::Leveled,
    ) -> Result<Vec<WarningView>, AppError> {
        let accepted = self.accepted_on_page(plan, page, seen).await?;
        let mut views = Vec::new();
        for (id, warning) in plan.warnings.iter().filter(|(id, _)| page.contains(id)) {
            let (Some(issue), Some(days)) = (
                plan.issues.get(id),
                plan.issues
                    .get(id)
                    .and_then(|issue| warning_days(plan, issue, warning)),
            ) else {
                continue;
            };
            let Some(held) = plan.held.get(&issue.project) else {
                continue;
            };
            let names_person_on_bar = matches!(
                warning,
                Warning::Away { .. } | Warning::TooMuchAtOnce { .. }
            );
            // A warning about the person on the bar tells that the issue has one: not to a reader
            // who may not read *Assigned to*, so neither the list nor its count says so.
            if names_person_on_bar && !plan.reads(issue, Field::AssignedTo) {
                continue;
            }
            let person = match plan
                .person_on_bar(*id)
                .filter(|_| names_person_on_bar && plan.reads(issue, Field::AssignedTo))
            {
                Some(person) => self.shown_person(held, person).await?,
                None => None,
            };
            let first = plan.person_on_bar(*id);
            let named = names_person(held, first, person.as_ref());
            // How much a person not named can work is not the reader's to know (UX §4.7).
            if matches!(warning, Warning::TooMuchAtOnce { .. }) && !named {
                continue;
            }
            let told = told_of(plan, issue, warning, (days, named));
            let accepted_by = match accepted.iter().find(|kept| {
                kept.issue == *id
                    && kept.kind == warning.kind()
                    && kept.holds_for(issue.start_date, issue.due_date)
            }) {
                // The person on the bar who accepted a warning that does not name them is shown no
                // more openly than the warning shows them.
                Some(kept) if names_person_on_bar && !named && Some(kept.accepted_by) == first => {
                    Some(AcceptedView {
                        by: person.clone().unwrap_or(PersonRef::Someone),
                        at: kept.accepted_at,
                    })
                }
                Some(kept) => Some(AcceptedView {
                    by: self
                        .person_ref(
                            held,
                            PeopleKind::Authors,
                            PeopleContext::Ordinary,
                            kept.accepted_by,
                        )
                        .await?,
                    at: kept.accepted_at,
                }),
                None => None,
            };
            // The other issue is one the reader sees (only those are in the plan), named by its
            // readable ID from the tree the search read.
            let other = warning
                .other_issue()
                .and_then(|other| plan.issues.get(&other))
                .and_then(|found| {
                    seen.stored(found.project).map(|project| {
                        let key = bb_core::readable_id::ReadableId::new(
                            project.short_name.clone(),
                            found.code.clone(),
                        );
                        (found.id, key.to_string())
                    })
                });
            views.push(WarningView {
                issue: *id,
                kind: warning.kind(),
                sentence: warning.sentence(named),
                days: told.days,
                what: *warning,
                other,
                person,
                ways_out: told.ways_out,
                move_after: told.move_after,
                due_needed: told.due_needed,
                accepted: accepted_by,
            });
        }
        Ok(views)
    }
}

/// The dependencies between issues of the page.
fn page_dependencies(plan: &Plan, page: &HashSet<IssueId>) -> Vec<(IssueId, IssueId)> {
    plan.dependencies
        .iter()
        .copied()
        .filter(|(first, then)| page.contains(first) && page.contains(then))
        .collect()
}

/// A page of the timeline: the rows of a search that have a date the reader may read, or the
/// issues of *Not scheduled yet*, with the count of those.
///
/// # Errors
/// The mistakes of the text; `resource.not-found` for a saved search or a project the person may
/// not open; `request.invalid` for a span that is backwards or longer than a year, a grouping the
/// timeline does not offer, or a cursor that is not one; `search.too-slow` for a search that takes
/// longer than the limit.
pub async fn timeline<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: TimelineQuery,
) -> Result<Authorized<TimelinePage>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let span =
        Days::new(query.first, query.last).ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let window = window_of(span)?;
    let (text, scope) = ctx.timeline_search(&actor, &query).await?;
    let words = date_words(&query.options);
    let tray = ctx
        .part_search(&actor, (&text, scope), &words, TimelinePart::NotScheduled)
        .await?;
    let not_scheduled = match &tray {
        Some(prepared) => ctx
            .store()
            .search_count(&prepared.request)
            .await
            .map_err(stopped)?,
        None => 0,
    };
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    let wanted = match query.part {
        TimelinePart::Rows => {
            ctx.part_search(&actor, (&text, scope), &words, TimelinePart::Rows)
                .await?
        }
        TimelinePart::NotScheduled => tray,
    };
    let Some(mut prepared) = wanted else {
        let by_effort = query.options.plan_by_effort && actor.key.is_none();
        return Ok(organization.authorize(TimelinePage::empty(not_scheduled, by_effort)));
    };
    let targets = date_targets(&prepared.looked, &words);
    let group_by = match &query.group_by {
        Some(word) => group_target(&prepared.looked, word)?,
        None => None,
    };
    let found = ctx
        .issues_page(
            &actor,
            &mut prepared,
            query.cursor.as_deref(),
            query.limit,
            ReadOnRequest {
                people: true,
                labels: false,
                agent: crate::issues::WithAgent::No,
                time: true,
            },
        )
        .await?;
    let page: HashSet<IssueId> = found.issues.iter().map(|issue| issue.id).collect();
    let ask = PlanAsk {
        issues: found.issues.clone(),
        dates: targets,
        by_effort: query.options.plan_by_effort,
        window,
        actor: &actor,
    };
    let plan = match query.part {
        TimelinePart::Rows => ctx.plan_with_surroundings(ask).await?,
        TimelinePart::NotScheduled => ctx.work_out_plan(ask, &[], &[]).await?,
    };
    let rows = ctx
        .timeline_rows(
            &plan,
            &found.issues,
            found.items,
            (group_by, targets),
            (query.part, span),
        )
        .await?;
    let dependencies = page_dependencies(&plan, &page);
    let (warnings, goals) = if query.part == TimelinePart::Rows {
        (
            ctx.warning_views(&plan, &page, &prepared.looked.seen)
                .await?,
            goals_of(&plan, &page),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    // Away days and public holidays are a person's in every project: only the minutes of a day
    // depend on the share in a project, and a row hatches no minutes. One project of the page is
    // enough to read them through the port of work times.
    let project = found.issues.first().map(|issue| issue.project);
    let people_days = ctx
        .people_days(&actor, &rows, (group_by, query.part), project, span)
        .await?;
    Ok(organization.authorize(TimelinePage {
        rows,
        people_days,
        next_cursor: found.next_cursor,
        dependencies,
        warnings,
        goals,
        not_scheduled,
        planned_by_effort: plan.by_effort,
    }))
}
