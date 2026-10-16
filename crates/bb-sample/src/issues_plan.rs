//! The plan of the issues (architecture §11, *Projects*): which issues there are, where, of which
//! type, with which text, and everything that happens to them afterwards, as a list of steps in
//! the order of time. It has no database in it, so its tests are quick; [`crate::issues`] carries
//! it out through the use cases.
//!
//! The plan is worked out from the counts of `tools/dev/sample.json` scaled to the number of
//! issues asked for, and from the seed: the same numbers give the same plan. Time is counted in
//! working seconds ([`Calendar`]).
//!
//! The file is longer than 400 lines (coding-standards §2) because the plan is one thing worked
//! out in one pass (the issues, then what happens to them in time); its words, its tests and the
//! text of an issue are already in files of their own.

use std::collections::BTreeMap;

use bb_core::issue::{IssueIcon, Priority, STARTING_ISSUE_TYPES, Status};
use bb_core::message::Message;
use bb_dev_words::sample::{IssuePlan, TypeShare};

use crate::comments::{PlannedComment, plan_comments};
use crate::error::SampleError;
use crate::fields::labels;
use crate::issues_text::{describe, field_pick, priority};
use crate::issues_time::{Calendar, WORK_DAY};
use crate::issues_words::IssueWords;
use crate::random::SeededRandom;
use crate::relations::{PlannedRelation, plan_relations};

/// The name the starting issue type with this icon has, as the sample's plan writes it (`Bug`):
/// the bug has steps to reproduce, the one with sparkles is the feature, the one with a flag is the
/// goal whose issues take sub-issues.
fn starting_name(icon: IssueIcon) -> String {
    STARTING_ISSUE_TYPES
        .iter()
        .find(|starting| starting.icon == icon)
        .map(|starting| Message::from(starting.name).render())
        .unwrap_or_default()
}
/// A pick that the maker turns into a person: the plan does not know the people.
const PICKS: usize = 1 << 20;

/// The value an issue gets for one own field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldPick {
    /// The places of the choices of the field.
    Choices(Vec<usize>),
    /// A text.
    Text(String),
}

/// An issue as planned.
#[derive(Clone, Debug)]
pub(crate) struct PlannedIssue {
    /// The short name of the project it is made in.
    pub(crate) place: String,
    /// The list of things its title is made from.
    pub(crate) area: String,
    /// The place of its top-level project in the plan.
    pub(crate) family: usize,
    /// The place of its type in the plan.
    pub(crate) kind: usize,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) priority: Priority,
    /// Who it is assigned to, as picks that the maker turns into people.
    pub(crate) assigned_people: Vec<u32>,
    /// The value for each own field of the plan, in its order.
    pub(crate) fields: Vec<Option<FieldPick>>,
    /// The goal it is a sub-issue of.
    pub(crate) parent: Option<usize>,
    /// The tags, by their place in the plan.
    pub(crate) tags: Vec<usize>,
    /// The labels, by their place in [`crate::fields::labels`].
    pub(crate) labels: Vec<usize>,
}

/// What happens at a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Create(usize),
    Parent(usize),
    Tag(usize, usize),
    Label(usize, usize),
    Status(usize, Status),
    Comment(usize),
    Edit(usize),
    Relate(usize),
}

/// One thing that happens, at a working second, by a person the maker picks with `lane`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    pub(crate) at: i64,
    pub(crate) lane: u32,
    pub(crate) action: Action,
}

/// Everything the sample does to its issues.
#[derive(Debug)]
pub(crate) struct Plan {
    pub(crate) issues: Vec<PlannedIssue>,
    pub(crate) comments: Vec<PlannedComment>,
    pub(crate) relations: Vec<PlannedRelation>,
    /// In the order of time (the order they were planned in, for the same second).
    pub(crate) steps: Vec<Step>,
}

/// `total` shared out by `weights`, each share a whole number, together exactly `total` (the
/// largest remainders get the rest; the earlier one on a tie).
pub(crate) fn allocate(total: u32, weights: &[u32]) -> Vec<u32> {
    let sum: u64 = weights.iter().map(|weight| u64::from(*weight)).sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let mut shares: Vec<(u32, u64)> = weights
        .iter()
        .map(|weight| {
            let exact = u64::from(total) * u64::from(*weight);
            (u32::try_from(exact / sum).unwrap_or(0), exact % sum)
        })
        .collect();
    let given: u32 = shares.iter().map(|(share, _)| *share).sum();
    let mut order: Vec<usize> = (0..shares.len()).collect();
    order.sort_by_key(|at| std::cmp::Reverse(shares[*at].1));
    let rest = usize::try_from(total.saturating_sub(given)).unwrap_or(0);
    for at in order.into_iter().take(rest) {
        shares[at].0 += 1;
    }
    shares.into_iter().map(|(share, _)| share).collect()
}

/// `count` of the full sample (`full` issues) scaled to a sample of `wanted` issues, rounded.
pub(crate) fn scaled(count: u32, full: u32, wanted: u32) -> usize {
    if full == 0 {
        return 0;
    }
    let exact = u64::from(count) * u64::from(wanted) + u64::from(full) / 2;
    usize::try_from(exact / u64::from(full)).unwrap_or(0)
}

/// The place of a type in the plan, by its name.
fn kind_of(types: &[TypeShare], name: &str) -> Option<usize> {
    types.iter().position(|share| share.name == name)
}

/// A pick for the maker.
fn pick(random: &mut SeededRandom) -> u32 {
    u32::try_from(random.below(PICKS)).unwrap_or(0)
}

impl Plan {
    /// The plan for `wanted` issues over the working time of `calendar`.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when the plan names a type that is not there.
    pub(crate) fn new(
        names: &IssuePlan,
        words: &IssueWords,
        seed: u64,
        wanted: u32,
        calendar: &Calendar,
    ) -> Result<Self, SampleError> {
        for project in &names.projects {
            if !words.has_area(&project.area) {
                return Err(SampleError::Layout(format!(
                    "the project {} has the area {}, which issues.toml has no things for",
                    project.short_name, project.area
                )));
            }
        }
        let mut random = SeededRandom::new(seed ^ 0x155E_5A11);
        let mut issues = Vec::new();
        let per_project = allocate(
            wanted,
            &names
                .projects
                .iter()
                .map(|project| project.issues)
                .collect::<Vec<_>>(),
        );
        for (family, (project, count)) in names.projects.iter().zip(per_project).enumerate() {
            let places = allocate(
                count,
                &project
                    .places
                    .iter()
                    .map(|place| place.percent)
                    .collect::<Vec<_>>(),
            );
            let mut spots: Vec<&str> = Vec::new();
            for (place, number) in project.places.iter().zip(&places) {
                spots.extend(std::iter::repeat_n(
                    place.short_name.as_str(),
                    *number as usize,
                ));
            }
            random.shuffle(&mut spots);
            let shares = project.types.as_deref().unwrap_or(&names.types);
            let kinds = allocate(
                count,
                &shares.iter().map(|share| share.percent).collect::<Vec<_>>(),
            );
            let mut typed: Vec<usize> = Vec::new();
            for (share, number) in shares.iter().zip(&kinds) {
                let kind = kind_of(&names.types, &share.name).ok_or_else(|| {
                    SampleError::Layout(format!("the type {} is not in the plan", share.name))
                })?;
                typed.extend(std::iter::repeat_n(kind, *number as usize));
            }
            random.shuffle(&mut typed);
            for (place, kind) in spots.into_iter().zip(typed) {
                issues.push(PlannedIssue {
                    place: place.to_owned(),
                    area: project.area.clone(),
                    family,
                    kind,
                    title: String::new(),
                    description: String::new(),
                    priority: Priority::NEW,
                    assigned_people: Vec::new(),
                    fields: Vec::new(),
                    parent: None,
                    tags: Vec::new(),
                    labels: Vec::new(),
                });
            }
        }
        // The order of the issues is the order they are made in.
        random.shuffle(&mut issues);
        let mut plan = Self {
            issues,
            comments: Vec::new(),
            relations: Vec::new(),
            steps: Vec::new(),
        };
        plan.describe(names, words, &mut random);
        plan.arrange(names, words, calendar, wanted, &mut random);
        // Stable: the steps of one second stay in the order they were planned in.
        plan.steps.sort_by_key(|step| step.at);
        Ok(plan)
    }

    /// Gives every issue its text, priority, people, own fields, tags and labels.
    fn describe(&mut self, names: &IssuePlan, words: &IssueWords, random: &mut SeededRandom) {
        let all_labels = labels(names);
        let (bug_name, feature_name, goal_name) = (
            starting_name(IssueIcon::Bug),
            starting_name(IssueIcon::Sparkles),
            starting_name(IssueIcon::Flag),
        );
        for issue in &mut self.issues {
            let type_name = names
                .types
                .get(issue.kind)
                .map(|share| share.name.as_str())
                .unwrap_or_default();
            let family = names
                .projects
                .get(issue.family)
                .map(|project| project.short_name.as_str())
                .unwrap_or_default();
            let bug = type_name == bug_name;
            let templates = if bug {
                &words.bug
            } else if type_name == feature_name {
                &words.feature
            } else if type_name == goal_name {
                &words.goal
            } else {
                &words.task
            };
            let title = random.pick(&templates.titles).cloned().unwrap_or_default();
            issue.title = words.fill(&title, &issue.area, random);
            issue.description = describe(words, templates, bug, &issue.area, random);
            issue.priority = priority(bug, random);
            issue.assigned_people = match random.below(100) {
                0..=74 => vec![pick(random)],
                75..=84 => vec![pick(random), pick(random)],
                _ => Vec::new(),
            };
            issue.fields = names
                .fields
                .iter()
                .map(|field| field_pick(field, type_name, family, words, random))
                .collect();
            if random.chance(40) && !names.tags.is_empty() {
                let first = random.below(names.tags.len());
                issue.tags.push(first);
                let second = random.below(names.tags.len());
                if random.chance(30) && second != first {
                    issue.tags.push(second);
                }
            }
            // A label of each group, with a chance that gets smaller for the later groups, and
            // now and then one that is in no group.
            for group in 0..names.label_groups.len() {
                let chance =
                    u32::try_from(55_usize.saturating_sub(group * 25).max(15)).unwrap_or(15);
                if random.chance(chance) {
                    let of_group: Vec<usize> = (0..all_labels.len())
                        .filter(|at| all_labels[*at].group == Some(group))
                        .collect();
                    issue.labels.extend(random.pick(&of_group));
                }
            }
            if random.chance(10) {
                let loose: Vec<usize> = (0..all_labels.len())
                    .filter(|at| all_labels[*at].group.is_none())
                    .collect();
                issue.labels.extend(random.pick(&loose));
            }
        }
    }

    /// Puts everything in time: when each issue is made and what happens to it next.
    fn arrange(
        &mut self,
        names: &IssuePlan,
        words: &IssueWords,
        calendar: &Calendar,
        wanted: u32,
        random: &mut SeededRandom,
    ) {
        let count = i64::try_from(self.issues.len()).unwrap_or(1).max(1);
        let end = calendar.length() - 1;
        let slot = (calendar.length() / count).max(1);
        let made_at: Vec<i64> = (0..count)
            .map(|at| at * slot + random.below_i64(slot))
            .collect();
        self.give_sub_issues(names, random);

        for (at, issue) in self.issues.iter().enumerate() {
            let when = made_at[at];
            let mut steps = Vec::new();
            let mut offset = 0;
            let mut push = |offset: i64, action: Action, random: &mut SeededRandom| {
                steps.push(Step {
                    at: (when + offset).min(end),
                    lane: pick(random),
                    action,
                });
            };
            push(offset, Action::Create(at), random);
            if issue.parent.is_some() {
                offset += 1;
                push(offset, Action::Parent(at), random);
            }
            for tag in &issue.tags {
                offset += 1;
                push(offset, Action::Tag(at, *tag), random);
            }
            for label in &issue.labels {
                offset += 1;
                push(offset, Action::Label(at, *label), random);
            }
            // How far it got: the older an issue is, the likelier it is done.
            let age = ((count - 1 - i64::try_from(at).unwrap_or(0)) * 55) / count;
            let roll = random.below_i64(100);
            let room = (end - when - 60).max(0);
            if roll < 20 + age {
                if random.chance(55) {
                    let started = when + 30 + random.skewed(room.min(3 * WORK_DAY));
                    push(
                        started - when,
                        Action::Status(at, Status::InProgress),
                        random,
                    );
                    let after = (end - started - 60).clamp(0, 20 * WORK_DAY);
                    let done = started + 60 + random.skewed(after);
                    push(done - when, Action::Status(at, Status::Done), random);
                } else {
                    let done = when + 30 + random.skewed(room.min(20 * WORK_DAY));
                    push(done - when, Action::Status(at, Status::Done), random);
                }
            } else if roll < 35 + age {
                let started = when + 30 + random.skewed(room.min(5 * WORK_DAY));
                push(
                    started - when,
                    Action::Status(at, Status::InProgress),
                    random,
                );
            }
            self.steps.extend(steps);
        }

        let comments = scaled(names.comments, names.total, wanted);
        self.comments = plan_comments(
            comments,
            names.edited_comments_percent,
            &self.issues,
            &made_at,
            end,
            words,
            random,
        );
        for (at, comment) in self.comments.iter().enumerate() {
            self.steps.push(Step {
                at: comment.at,
                lane: comment.lane,
                action: Action::Comment(at),
            });
            if let Some(edit) = &comment.edit {
                self.steps.push(Step {
                    at: edit.at,
                    lane: comment.lane,
                    action: Action::Edit(at),
                });
            }
        }

        let relations = scaled(names.relations, names.total, wanted);
        self.relations = plan_relations(relations, &self.issues, random);
        for (at, relation) in self.relations.iter().enumerate() {
            let after = made_at[relation.from].max(made_at[relation.to]) + 5;
            let room = (end - after).clamp(0, 30 * WORK_DAY);
            self.steps.push(Step {
                at: (after + random.skewed(room)).min(end),
                lane: pick(random),
                action: Action::Relate(at),
            });
        }
    }

    /// Sub-issues: each goal takes a few of the issues made after it in the same project.
    fn give_sub_issues(&mut self, names: &IssuePlan, random: &mut SeededRandom) {
        let goal = kind_of(&names.types, &starting_name(IssueIcon::Flag));
        let mut places: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (at, issue) in self.issues.iter().enumerate() {
            places.entry(issue.place.clone()).or_default().push(at);
        }
        for members in places.values() {
            for (position, &at) in members.iter().enumerate() {
                if Some(self.issues[at].kind) != goal {
                    continue;
                }
                let wanted = 2 + random.below(4);
                let mut taken = 0;
                for &child in members.iter().skip(position + 1).take(400) {
                    if taken == wanted {
                        break;
                    }
                    let free = Some(self.issues[child].kind) != goal
                        && self.issues[child].parent.is_none();
                    if free && random.chance(15) {
                        self.issues[child].parent = Some(at);
                        taken += 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
