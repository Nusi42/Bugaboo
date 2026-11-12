//! Sub-issues and related issues (ISS-03, ISS-04, ADR-0027): the kinds of relation with a name for
//! each direction, the relations themselves, and the rule that a parent never sits under its own
//! sub-issue.
//!
//! A relation is stored once, from one issue to the other, in the *forward* direction of its kind
//! (*A blocks B*). It is read from both sides: A sees the forward name (*blocks B*), B sees the
//! backward name (*is blocked by A*). A kind whose two names are the same (*related to*) reads
//! the same from both sides.

use std::collections::{HashMap, HashSet, VecDeque};

use jiff::Timestamp;

use crate::id::{IssueId, RelationId, RelationKindId, UserId};
use crate::message::{self, MessageKey};
use crate::personal::Entity;

/// The most levels of sub-issues one chain may have. A deeper chain is refused, so the check for
/// circles and the progress of a goal always finish.
pub const SUB_ISSUE_DEPTH_MAX: usize = 32;

/// The most relations one issue may have, so a page of them stays a page.
pub const RELATIONS_MAX: usize = 200;

/// The most sub-issues one issue may have.
pub const SUB_ISSUES_MAX: usize = 500;

words! {
    /// Which end of a relation a person starts from.
    pub enum Direction {
        /// From the issue the relation is stored from: *this issue blocks the other*.
        Forward => "forward",
        /// From the other end: *this issue is blocked by the other*.
        Backward => "backward",
    }
}

words_in_api!(
    Direction,
    "Which end of a relation a person starts from: the forward name or the backward one."
);

impl Direction {
    /// The other end.
    #[must_use]
    pub const fn flipped(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

/// The most kinds of relation that carry a date effect at one time (ADR-0043, decision 4): the
/// starting kind *blocks* and one other. A third is refused.
pub const DATE_EFFECT_KINDS_MOST: usize = 2;

words! {
    /// What a kind of relation does to dates on the timeline (ADR-0043, decision 4). In M8
    /// *Must finish first* is the only effect besides none; the others (*start together*, *finish
    /// together*, *finish when it starts*) are V1 and join as variants here.
    pub enum DateEffect {
        /// The relation says nothing about dates.
        Nothing => "none",
        /// The issue it starts from must finish first; the other one waits. A relation of such a
        /// kind is a dependency.
        MustFinishFirst => "must-finish-first",
    }
    messages name = "relation-kinds.date-effect."
}

words_in_api!(
    DateEffect,
    "What a kind of relation does to dates: nothing, or *Must finish first*."
);

impl DateEffect {
    /// Whether a relation of a kind with this effect is a dependency.
    #[must_use]
    pub const fn is_dependency(self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// A kind of relation that is made at the first start: the message of each name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartingRelationKind {
    /// What the issue it starts from reads (*blocks*).
    pub forward: MessageKey,
    /// What the other issue reads (*is blocked by*).
    pub backward: MessageKey,
    /// What the kind does to dates.
    pub date_effect: DateEffect,
}

/// The kinds of relation of the first start, in order. The one definition; after the first start
/// the kinds are rows that can be changed, and this list is not read again.
pub const STARTING_RELATION_KINDS: [StartingRelationKind; 4] = [
    StartingRelationKind {
        forward: message::RELATION_KIND_BLOCKS,
        backward: message::RELATION_KIND_IS_BLOCKED_BY,
        date_effect: DateEffect::MustFinishFirst,
    },
    StartingRelationKind {
        forward: message::RELATION_KIND_DUPLICATE_OF,
        backward: message::RELATION_KIND_HAS_DUPLICATE,
        date_effect: DateEffect::Nothing,
    },
    StartingRelationKind {
        forward: message::RELATION_KIND_CAUSES,
        backward: message::RELATION_KIND_IS_CAUSED_BY,
        date_effect: DateEffect::Nothing,
    },
    StartingRelationKind {
        forward: message::RELATION_KIND_RELATED_TO,
        backward: message::RELATION_KIND_RELATED_TO,
        date_effect: DateEffect::Nothing,
    },
];

/// What a relation means (ISS-04): *blocks / is blocked by*, *duplicate of / has duplicate*, or a
/// kind the organization made. It is the same in every project.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RelationKind {
    /// The ID of the kind.
    #[not_personal]
    pub id: RelationKindId,
    /// What the issue it starts from reads. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub forward: String,
    /// What the other issue reads. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub backward: String,
    /// What it does to dates (ADR-0043).
    #[not_personal]
    pub date_effect: DateEffect,
    /// The place in the order the kinds are offered in.
    #[not_personal]
    pub position: i64,
    /// When it was retired: no new relations of this kind, while the old ones keep it.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

impl RelationKind {
    /// The name that an issue reads at one end of a relation of this kind.
    #[must_use]
    pub fn name(&self, direction: Direction) -> &str {
        match direction {
            Direction::Forward => &self.forward,
            Direction::Backward => &self.backward,
        }
    }

    /// Whether both ends read the same (*related to*): then it makes no difference which issue a
    /// person started from.
    #[must_use]
    pub fn is_symmetric(&self) -> bool {
        // Not `same_name`: this says how a kind is shown (one name or two), and a kind stored with
        // names that differ in inner spaces has always been shown with two.
        self.forward.to_lowercase() == self.backward.to_lowercase()
    }
}

/// One connection between two issues (ISS-04). Stored once; both issues show it.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Relation {
    /// The ID of the relation.
    #[not_personal]
    pub id: RelationId,
    /// What it means.
    #[not_personal]
    pub kind: RelationKindId,
    /// The issue that reads the forward name (*blocks*).
    #[not_personal]
    pub from: IssueId,
    /// The issue that reads the backward name (*is blocked by*).
    #[not_personal]
    pub to: IssueId,
    /// Who added it.
    #[personal(category = "activity", basis = "contract")]
    pub added_by: UserId,
    /// When it was added.
    #[not_personal]
    pub added_at: Timestamp,
}

impl Relation {
    /// The issue at the other end, seen from `issue`; `None` when `issue` is at neither end.
    #[must_use]
    pub fn other_end(&self, issue: IssueId) -> Option<IssueId> {
        if self.from == issue {
            Some(self.to)
        } else if self.to == issue {
            Some(self.from)
        } else {
            None
        }
    }

    /// Which end `issue` is at; `None` when it is at neither.
    #[must_use]
    pub fn direction_of(&self, issue: IssueId) -> Option<Direction> {
        if self.from == issue {
            Some(Direction::Forward)
        } else if self.to == issue {
            Some(Direction::Backward)
        } else {
            None
        }
    }
}

/// One dependency, as the issues at its two ends: the issue that must finish first, and the one
/// that waits for it. The relation of a kind with a date effect, read from its `from` and `to`.
pub type Dependency = (IssueId, IssueId);

/// The dependencies as a map from the issue that must finish first to the issues that wait for it,
/// built once so a walk, or many, never builds it again.
pub struct DependencyGraph {
    waiting_for: HashMap<IssueId, Vec<IssueId>>,
}

impl DependencyGraph {
    /// The graph of these dependencies.
    #[must_use]
    pub fn new(dependencies: &[Dependency]) -> Self {
        let mut waiting_for: HashMap<IssueId, Vec<IssueId>> = HashMap::new();
        for (first, then) in dependencies {
            waiting_for.entry(*first).or_default().push(*then);
        }
        Self { waiting_for }
    }

    /// The issues that wait for `issue` directly, in the order their dependencies were given. The
    /// step of every walk along dependencies: the loop check and the order of *Move it after*
    /// (`crate::plan`).
    #[must_use]
    pub fn waiting(&self, issue: IssueId) -> &[IssueId] {
        self.waiting_for.get(&issue).map_or(&[], Vec::as_slice)
    }

    /// The issues that wait for `start` over any number of steps, `start` first, in the order of a
    /// walk by steps: each once, and only through issues `through` lets the walk pass (an issue it
    /// refuses is neither answered nor walked past). Lazy, so a caller that needs only so many
    /// stops the walk there. The one walk forward along dependencies: *Move it after* and its
    /// preview both take it.
    pub fn waiting_from<'a>(
        &'a self,
        start: IssueId,
        through: impl Fn(IssueId) -> bool + 'a,
    ) -> impl Iterator<Item = IssueId> + 'a {
        let mut seen: HashSet<IssueId> = HashSet::new();
        let mut queue = VecDeque::from([start]);
        std::iter::from_fn(move || {
            while let Some(at) = queue.pop_front() {
                if through(at) && seen.insert(at) {
                    queue.extend(self.waiting(at).iter().copied());
                    return Some(at);
                }
            }
            None
        })
    }

    /// The shortest way from `start` to `goal`, both ends included; `None` when the issues that wait
    /// for `start`, over any number of steps, never include `goal`. The one walk of the loop check:
    /// making a dependency and giving a kind a date effect both use it, and the plan of the timeline
    /// can ask it for any two issues. Written once and pure, so it is tested without a database.
    #[must_use]
    pub fn path(&self, start: IssueId, goal: IssueId) -> Option<Vec<IssueId>> {
        if start == goal {
            return Some(vec![start]);
        }
        let mut came_from: HashMap<IssueId, IssueId> = HashMap::new();
        let mut seen: HashSet<IssueId> = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            for next in self.waiting(at) {
                if !seen.insert(*next) {
                    continue;
                }
                came_from.insert(*next, at);
                if *next == goal {
                    let mut path = vec![goal];
                    let mut back = goal;
                    while let Some(before) = came_from.get(&back) {
                        path.push(*before);
                        back = *before;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(*next);
            }
        }
        None
    }

    /// The loop that the new dependency `new` would close, as the issues on it in order: the one that
    /// must finish first, the one that waits, and on from there until the one that waits for the
    /// last of them is the first again. `None` when it closes none.
    #[must_use]
    pub fn loop_closed_by(&self, new: Dependency) -> Option<Vec<IssueId>> {
        // The loop is the new step, then the way from the one that waits back to the one that must
        // finish first.
        self.path(new.1, new.0).map(|back| {
            std::iter::once(new.0)
                .chain(back.into_iter().take_while(|issue| *issue != new.0))
                .collect()
        })
    }
}

/// The shortest way from `start` to `goal` along `dependencies` (see [`DependencyGraph::path`]).
#[must_use]
pub fn dependency_path(
    dependencies: &[Dependency],
    start: IssueId,
    goal: IssueId,
) -> Option<Vec<IssueId>> {
    DependencyGraph::new(dependencies).path(start, goal)
}

/// The loop that a new dependency would close (see [`DependencyGraph::loop_closed_by`]);
/// `dependencies` are the ones there are now.
#[must_use]
pub fn loop_closed_by(dependencies: &[Dependency], new: Dependency) -> Option<Vec<IssueId>> {
    DependencyGraph::new(dependencies).loop_closed_by(new)
}

/// An issue that sits under another one (ISS-03).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueParent {
    /// The sub-issue.
    #[not_personal]
    pub child: IssueId,
    /// The issue it belongs under.
    #[not_personal]
    pub parent: IssueId,
    /// Who put it there.
    #[personal(category = "activity", basis = "contract")]
    pub set_by: UserId,
    /// When.
    #[not_personal]
    pub set_at: Timestamp,
}

/// Why an issue cannot be put under another one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentRefusal {
    /// The parent is the issue itself, or one of its own sub-issues, however deep: it would make a
    /// circle.
    Circle,
    /// The chain above the new parent is longer than a chain of sub-issues may be.
    TooDeep,
}

/// Whether `child` may be put under `parent`, given how issues sit under each other now.
/// `parent_of` says which issue an issue sits under, if any. Written once and pure, so the rule is
/// tested without a database: the chain from `parent` upward must reach neither `child` (a circle)
/// nor more than [`SUB_ISSUE_DEPTH_MAX`] levels.
///
/// # Errors
/// [`ParentRefusal::Circle`] when `child` is `parent` or above it, [`ParentRefusal::TooDeep`] for a
/// chain that is too long.
pub fn check_parent(
    child: IssueId,
    parent: IssueId,
    parent_of: impl Fn(IssueId) -> Option<IssueId>,
) -> Result<(), ParentRefusal> {
    let mut at = parent;
    for _ in 0..=SUB_ISSUE_DEPTH_MAX {
        if at == child {
            return Err(ParentRefusal::Circle);
        }
        match parent_of(at) {
            Some(above) => at = above,
            None => return Ok(()),
        }
    }
    Err(ParentRefusal::TooDeep)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::id::SequentialIds;

    fn issues(count: usize) -> Vec<IssueId> {
        let mut ids = SequentialIds::default();
        (0..count)
            .map(|_| IssueId::generate(&mut ids).unwrap())
            .collect()
    }

    #[test]
    fn iss_04_a_kind_stored_with_names_that_differ_in_inner_spaces_is_still_shown_with_two() {
        let kind = |forward: &str, backward: &str| RelationKind {
            id: RelationKindId::generate(&mut SequentialIds::default()).unwrap(),
            forward: forward.to_owned(),
            backward: backward.to_owned(),
            date_effect: DateEffect::Nothing,
            position: 0,
            retired_at: None,
            version: 1,
        };
        assert!(kind("Related to", "related TO").is_symmetric());
        // Stored before one rule told names apart: it keeps being shown as two names.
        assert!(!kind("Related  to", "related to").is_symmetric());
        assert!(!kind("Blocks", "Is blocked by").is_symmetric());
    }

    #[test]
    fn iss_03_an_issue_cannot_sit_under_itself_or_under_its_own_sub_issue() {
        let all = issues(4);
        let (a, b, c, other) = (all[0], all[1], all[2], all[3]);
        // c sits under b, b under a.
        let tree: HashMap<IssueId, IssueId> = [(c, b), (b, a)].into_iter().collect();
        let above = |issue| tree.get(&issue).copied();
        assert_eq!(check_parent(a, a, above), Err(ParentRefusal::Circle));
        assert_eq!(check_parent(a, b, above), Err(ParentRefusal::Circle));
        assert_eq!(check_parent(a, c, above), Err(ParentRefusal::Circle));
        assert_eq!(check_parent(c, a, above), Ok(()));
        assert_eq!(check_parent(other, c, above), Ok(()));
    }

    #[test]
    fn iss_03_a_chain_that_is_too_long_is_refused_and_a_loop_in_the_data_ends() {
        let mut all = issues(SUB_ISSUE_DEPTH_MAX + 5);
        let new_child = all.pop().unwrap();
        let tree: HashMap<IssueId, IssueId> =
            all.windows(2).map(|pair| (pair[1], pair[0])).collect();
        let above = |issue| tree.get(&issue).copied();
        let bottom = all[all.len() - 1];
        assert_eq!(
            check_parent(new_child, bottom, above),
            Err(ParentRefusal::TooDeep)
        );
        // Data that already holds a loop (it cannot be made, but must not hang the check).
        let looped: HashMap<IssueId, IssueId> =
            [(all[0], all[1]), (all[1], all[0])].into_iter().collect();
        assert_eq!(
            check_parent(new_child, all[0], |issue| looped.get(&issue).copied()),
            Err(ParentRefusal::TooDeep)
        );
    }

    #[test]
    fn plan_02_a_dependency_that_closes_a_loop_names_the_loop_in_order() {
        let all = issues(5);
        let (a, b, c, d, other) = (all[0], all[1], all[2], all[3], all[4]);
        // a is first for b, b for c, c for d.
        let there = [(a, b), (b, c), (c, d)];
        assert_eq!(loop_closed_by(&there, (d, a)), Some(vec![d, a, b, c]));
        assert_eq!(loop_closed_by(&there, (c, a)), Some(vec![c, a, b]));
        assert_eq!(loop_closed_by(&there, (b, a)), Some(vec![b, a]));
        assert_eq!(loop_closed_by(&there, (a, d)), None);
        assert_eq!(loop_closed_by(&there, (other, a)), None);
        assert_eq!(dependency_path(&there, a, d), Some(vec![a, b, c, d]));
        assert_eq!(dependency_path(&there, d, a), None);
        // A loop that is stored already does not hang the walk.
        let stored = [(a, b), (b, a)];
        assert_eq!(dependency_path(&stored, a, other), None);
    }

    #[test]
    fn plan_02_only_must_finish_first_is_a_dependency_and_blocks_starts_with_it() {
        assert!(DateEffect::MustFinishFirst.is_dependency());
        assert!(!DateEffect::Nothing.is_dependency());
        let with_effect: Vec<&str> = STARTING_RELATION_KINDS
            .iter()
            .filter(|kind| kind.date_effect.is_dependency())
            .map(|kind| kind.forward.template())
            .collect();
        assert_eq!(with_effect, ["blocks"]);
        assert!(with_effect.len() <= DATE_EFFECT_KINDS_MOST);
    }

    #[test]
    fn iss_04_a_relation_is_read_from_both_ends() {
        let all = issues(3);
        let mut ids = SequentialIds::default();
        let relation = Relation {
            id: RelationId::generate(&mut ids).unwrap(),
            kind: RelationKindId::generate(&mut ids).unwrap(),
            from: all[0],
            to: all[1],
            added_by: UserId::generate(&mut ids),
            added_at: Timestamp::UNIX_EPOCH,
        };
        assert_eq!(relation.other_end(all[0]), Some(all[1]));
        assert_eq!(relation.other_end(all[1]), Some(all[0]));
        assert_eq!(relation.other_end(all[2]), None);
        assert_eq!(relation.direction_of(all[0]), Some(Direction::Forward));
        assert_eq!(relation.direction_of(all[1]), Some(Direction::Backward));
        assert_eq!(Direction::Forward.flipped(), Direction::Backward);
    }

    #[test]
    fn iss_04_the_starting_kinds_read_in_both_directions() {
        let names: Vec<(&str, &str)> = STARTING_RELATION_KINDS
            .iter()
            .map(|kind| (kind.forward.template(), kind.backward.template()))
            .collect();
        assert_eq!(
            names,
            [
                ("blocks", "is blocked by"),
                ("duplicate of", "has duplicate"),
                ("causes", "is caused by"),
                ("related to", "related to"),
            ]
        );
    }
}
