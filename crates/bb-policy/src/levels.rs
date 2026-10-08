//! The levels roles have: how a role sees a field of the catalogue (*Hidden < Can see < Can
//! change*) and how it sees people of a kind (*Hidden < Role only < Names*).
//!
//! The two work the same way (ADR-0025, decisions 10 and 11): a role has a level of its own for
//! each subject and may set another one for a project, which the projects below it inherit. So the
//! walk, the mask, the reasons and the rules that are stored are written once, over
//! [`LevelSubject`], and `Field` and `PeopleKind` are its two subjects.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::words;
use bb_core::words::Word;

use crate::evaluate::{Evaluation, RoleData};

words! {
    /// The two kinds of level a role has. The word starts the name of a changed level in an
    /// event (`field.title`, `people.followers`) and tells the store which tables to read.
    pub enum LevelKind {
        /// How a role sees a field of the catalogue.
        Field => "field",
        /// How a role sees people of a kind.
        People => "people",
    }
}

/// Something a role has a level for: a field, or a kind of people information.
pub trait LevelSubject: Word {
    /// The levels of this subject, from the closed one to the most open.
    type Level: Word;

    /// Which kind of level this is.
    const KIND: LevelKind;

    /// The level when nothing is set: closed (default deny).
    const CLOSED: Self::Level;

    /// The rules of this subject among the rules of both kinds.
    fn rules_in(rules: &LevelRules) -> &[LevelRule<Self>];

    /// The same, to add to or take from.
    fn rules_in_mut(rules: &mut LevelRules) -> &mut Vec<LevelRule<Self>>;

    /// The levels of this subject in an evaluation.
    fn levels_in(evaluation: &Evaluation) -> &Levels<Self>;

    /// The same, to narrow.
    fn levels_in_mut(evaluation: &mut Evaluation) -> &mut Levels<Self>;
}

impl LevelSubject for Field {
    type Level = FieldLevel;
    const KIND: LevelKind = LevelKind::Field;
    const CLOSED: FieldLevel = FieldLevel::Hidden;

    fn rules_in(rules: &LevelRules) -> &[FieldRule] {
        &rules.fields
    }

    fn rules_in_mut(rules: &mut LevelRules) -> &mut Vec<FieldRule> {
        &mut rules.fields
    }

    fn levels_in(evaluation: &Evaluation) -> &Levels<Self> {
        &evaluation.fields
    }

    fn levels_in_mut(evaluation: &mut Evaluation) -> &mut Levels<Self> {
        &mut evaluation.fields
    }
}

impl LevelSubject for PeopleKind {
    type Level = PeopleLevel;
    const KIND: LevelKind = LevelKind::People;
    const CLOSED: PeopleLevel = PeopleLevel::Hidden;

    fn rules_in(rules: &LevelRules) -> &[PeopleRule] {
        &rules.people
    }

    fn rules_in_mut(rules: &mut LevelRules) -> &mut Vec<PeopleRule> {
        &mut rules.people
    }

    fn levels_in(evaluation: &Evaluation) -> &Levels<Self> {
        &evaluation.people
    }

    fn levels_in_mut(evaluation: &mut Evaluation) -> &mut Levels<Self> {
        &mut evaluation.people
    }
}

/// The level of a subject for a role: in a project (`Some`) or the role's own (`None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelRule<K: LevelSubject> {
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The role.
    pub role: RoleId,
    /// The field or the kind of people information.
    pub subject: K,
    /// The level.
    pub level: K::Level,
}

/// The level of a field for a role.
pub type FieldRule = LevelRule<Field>;

/// How a role sees people of a kind.
pub type PeopleRule = LevelRule<PeopleKind>;

/// The levels roles have set, of both kinds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelRules {
    /// The field levels.
    pub fields: Vec<FieldRule>,
    /// The levels of the kinds of people information.
    pub people: Vec<PeopleRule>,
}

impl LevelRules {
    /// The rules of one kind.
    #[must_use]
    pub fn of<K: LevelSubject>(&self) -> &[LevelRule<K>] {
        K::rules_in(self)
    }

    /// Adds a rule.
    pub fn add<K: LevelSubject>(&mut self, rule: LevelRule<K>) {
        K::rules_in_mut(self).push(rule);
    }

    /// The rules of these roles that apply at a place: their own, and those set for a project of
    /// `chain`. A role's rule for any other project is no part of the place.
    #[must_use]
    pub fn at(&self, roles: &BTreeSet<RoleId>, chain: &[ProjectId]) -> Self {
        let applies = |role: RoleId, project: Option<ProjectId>| {
            roles.contains(&role) && project.is_none_or(|project| chain.contains(&project))
        };
        Self {
            fields: filtered(&self.fields, applies),
            people: filtered(&self.people, applies),
        }
    }
}

/// The rules for which `keep` says yes (given their role and project).
fn filtered<K: LevelSubject>(
    rules: &[LevelRule<K>],
    keep: impl Fn(RoleId, Option<ProjectId>) -> bool,
) -> Vec<LevelRule<K>> {
    rules
        .iter()
        .filter(|rule| keep(rule.role, rule.project))
        .copied()
        .collect()
}

/// Where the level of a subject came from, for one role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelSource {
    /// Set for a project (the place itself or the nearest parent that set one).
    Project(ProjectId),
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: the level is the closed one (default deny).
    Unset,
}

/// What one role says about one subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleLevel<L> {
    /// The role.
    pub role: RoleId,
    /// Its level.
    pub level: L,
    /// Where it was set.
    pub source: LevelSource,
}

/// The depth of a project of the chain, from 1; `None` for another project.
fn chain_depth(chain: &[ProjectId], project: ProjectId) -> Option<usize> {
    chain
        .iter()
        .position(|step| *step == project)
        .map(|at| at + 1)
}

/// The nearest rule of a role: the project of the chain closest to the place that has one, else
/// the role's own. The one walk of steps 7 and 8 of the evaluation, also read by the tables of
/// what roles see and by the check of what a level becomes when a project's own is taken away.
#[must_use]
pub fn nearest_rule<R: Copy>(
    chain: &[ProjectId],
    rules: impl Iterator<Item = (Option<ProjectId>, R)>,
) -> Option<(LevelSource, R)> {
    let mut best: Option<(usize, LevelSource, R)> = None;
    for (project, rule) in rules {
        let (depth, source) = match project {
            None => (0, LevelSource::RoleOwn),
            Some(project) => match chain_depth(chain, project) {
                Some(depth) => (depth, LevelSource::Project(project)),
                None => continue,
            },
        };
        if best.as_ref().is_none_or(|(deepest, _, _)| depth > *deepest) {
            best = Some((depth, source, rule));
        }
    }
    best.map(|(_, source, rule)| (source, rule))
}

/// The level a role has for a subject at a place whose projects from the top down are `chain`,
/// and where it was set; `None` when nothing is set.
#[must_use]
pub fn level_of_role<K: LevelSubject>(
    chain: &[ProjectId],
    rules: &[LevelRule<K>],
    role: RoleId,
    subject: K,
) -> Option<(LevelSource, K::Level)> {
    nearest_rule(
        chain,
        rules
            .iter()
            .filter(|rule| rule.role == role && rule.subject == subject)
            .map(|rule| (rule.project, rule.level)),
    )
}

/// The levels of one kind in an evaluation: the level of every subject (the most open of the
/// roles that count) and what each role said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Levels<K: LevelSubject> {
    highest: BTreeMap<K, K::Level>,
    sources: BTreeMap<K, Vec<RoleLevel<K::Level>>>,
}

impl<K: LevelSubject> Levels<K> {
    /// Steps 7 and 8: for each subject and each role that counts, the level the role sets on the
    /// nearest project of the chain, else its own; the level of the subject is the highest.
    pub(crate) fn of_roles(
        chain: &[ProjectId],
        rules: &[LevelRule<K>],
        held: &[&RoleData],
    ) -> Self {
        let mut highest = BTreeMap::new();
        let mut sources = BTreeMap::new();
        for subject in K::ALL.iter().copied() {
            let mut top = K::CLOSED;
            let mut per_role = Vec::with_capacity(held.len());
            for role in held {
                let (source, level) = level_of_role(chain, rules, role.id, subject)
                    .unwrap_or((LevelSource::Unset, K::CLOSED));
                top = top.max(level);
                per_role.push(RoleLevel {
                    role: role.id,
                    level,
                    source,
                });
            }
            highest.insert(subject, top);
            sources.insert(subject, per_role);
        }
        Self { highest, sources }
    }

    /// The level of a subject; the closed one for a subject that is not listed.
    pub(crate) fn level(&self, subject: K) -> K::Level {
        self.highest.get(&subject).copied().unwrap_or(K::CLOSED)
    }

    /// What each role that counts says about a subject.
    pub(crate) fn roles(&self, subject: K) -> Vec<RoleLevel<K::Level>> {
        self.sources.get(&subject).cloned().unwrap_or_default()
    }

    /// The level of every subject.
    pub(crate) const fn all(&self) -> &BTreeMap<K, K::Level> {
        &self.highest
    }

    /// Closes every subject: the person is not here at all.
    pub(crate) fn close(&mut self) {
        for level in self.highest.values_mut() {
            *level = K::CLOSED;
        }
    }

    /// Lowers every level above `most` to it.
    pub(crate) fn cap(&mut self, most: K::Level) {
        for level in self.highest.values_mut() {
            *level = (*level).min(most);
        }
    }
}
