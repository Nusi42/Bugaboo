//! The levels roles have: how a role sees a field of the catalogue (*Hidden < Can see < Can
//! change*) and how it sees people of a kind (*Hidden < Role only < Names*).
//!
//! The two work the same way (ADR-0025, decisions 10 and 11): a role has a level of its own for
//! each subject and may set another one for a project, which the projects below it inherit. So the
//! walk, the mask, the reasons and the rules that are stored are written once, over
//! [`LevelSubject`], and `FieldKey` and `PeopleKind` are its two subjects. A field level can also
//! be set for one issue type (ADR-0027): those rows are [`TypedFieldRule`]s, found with the one
//! lookup of `bb_core::lookup`.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::Hash;

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, IssueTypeId, ProjectId, RoleId};
use bb_core::lookup::nearest_row;
use bb_core::offer::reaches;
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
pub trait LevelSubject: Copy + fmt::Debug + Hash + Ord + Send + Sync + 'static {
    /// The levels of this subject, from the closed one to the most open.
    type Level: Word;

    /// Which kind of level this is.
    const KIND: LevelKind;

    /// The level when nothing is set: closed (default deny).
    const CLOSED: Self::Level;

    /// The word of this subject, as stored and as the name of a changed level in an event.
    fn subject_code(self) -> Cow<'static, str>;

    /// The subject with this word, if there is one.
    fn from_subject_code(code: &str) -> Option<Self>;

    /// Every subject there is: the closed list, and the own fields the rules know.
    fn universe(rules: &LevelRules) -> Vec<Self>;

    /// The rules of this subject among the rules of both kinds.
    fn rules_in(rules: &LevelRules) -> &[LevelRule<Self>];

    /// The same, to add to or take from.
    fn rules_in_mut(rules: &mut LevelRules) -> &mut Vec<LevelRule<Self>>;

    /// The level a role sets for this subject at a place (the projects from the top down are
    /// `chain`), for an item of one issue type, where it was set, and whether the level is for
    /// the issue type itself (ADR-0027). Only fields have levels per issue type: for the other
    /// kind the issue type is ignored.
    fn level_for(
        rules: &LevelRules,
        chain: &[ProjectId],
        role: RoleId,
        subject: Self,
        issue_type: Option<IssueTypeId>,
    ) -> Option<(LevelSource, bool, Self::Level)>;

    /// The level a person holds for this subject in an evaluation, for an item of one issue type.
    fn held_level(
        evaluation: &Evaluation,
        subject: Self,
        issue_type: Option<IssueTypeId>,
    ) -> Self::Level;

    /// The levels of this subject in an evaluation.
    fn levels_in(evaluation: &Evaluation) -> &Levels<Self>;

    /// The same, to narrow.
    fn levels_in_mut(evaluation: &mut Evaluation) -> &mut Levels<Self>;
}

impl LevelSubject for FieldKey {
    type Level = FieldLevel;
    const KIND: LevelKind = LevelKind::Field;
    const CLOSED: FieldLevel = FieldLevel::Hidden;

    fn subject_code(self) -> Cow<'static, str> {
        self.code()
    }

    fn from_subject_code(code: &str) -> Option<Self> {
        Self::from_code(code)
    }

    fn universe(rules: &LevelRules) -> Vec<Self> {
        Field::ALL
            .iter()
            .copied()
            .map(Self::BuiltIn)
            .chain(rules.own_fields.iter().copied().map(Self::Own))
            .collect()
    }

    fn level_for(
        rules: &LevelRules,
        chain: &[ProjectId],
        role: RoleId,
        subject: Self,
        issue_type: Option<IssueTypeId>,
    ) -> Option<(LevelSource, bool, FieldLevel)> {
        let for_all = rules
            .fields
            .iter()
            .filter(|rule| rule.role == role && rule.subject == subject)
            .map(|rule| (rule.project, None, rule.level));
        let for_type = rules
            .typed
            .iter()
            .filter(|rule| rule.role == role && rule.subject == subject)
            .map(|rule| (rule.project, Some(rule.issue_type), rule.level));
        nearest_typed_rule(chain, issue_type, for_all.chain(for_type))
    }

    fn held_level(
        evaluation: &Evaluation,
        subject: Self,
        issue_type: Option<IssueTypeId>,
    ) -> FieldLevel {
        evaluation.field_level_for(issue_type, subject)
    }

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

    fn subject_code(self) -> Cow<'static, str> {
        Cow::Borrowed(self.code())
    }

    fn from_subject_code(code: &str) -> Option<Self> {
        Self::from_code(code)
    }

    fn universe(_rules: &LevelRules) -> Vec<Self> {
        Self::ALL.to_vec()
    }

    fn level_for(
        rules: &LevelRules,
        chain: &[ProjectId],
        role: RoleId,
        subject: Self,
        _issue_type: Option<IssueTypeId>,
    ) -> Option<(LevelSource, bool, PeopleLevel)> {
        level_of_role(chain, &rules.people, role, subject)
            .map(|(source, level)| (source, false, level))
    }

    fn held_level(
        evaluation: &Evaluation,
        subject: Self,
        _issue_type: Option<IssueTypeId>,
    ) -> PeopleLevel {
        evaluation.level(subject)
    }

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
pub type FieldRule = LevelRule<FieldKey>;

/// The level of a field for a role and one issue type, in a project or for the role itself
/// (ADR-0027): found before the level for all types at the same place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypedFieldRule {
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The role.
    pub role: RoleId,
    /// The issue type the level is for.
    pub issue_type: IssueTypeId,
    /// The field.
    pub subject: FieldKey,
    /// The level.
    pub level: FieldLevel,
}

/// How a role sees people of a kind.
pub type PeopleRule = LevelRule<PeopleKind>;

/// The levels roles have set, of both kinds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelRules {
    /// The field levels.
    pub fields: Vec<FieldRule>,
    /// The levels of the kinds of people information.
    pub people: Vec<PeopleRule>,
    /// The levels of fields for one issue type.
    pub typed: Vec<TypedFieldRule>,
    /// The own fields there are, so that each has a level in an evaluation (closed when no role
    /// says otherwise).
    pub own_fields: Vec<FieldId>,
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

    /// The rules of these roles wherever they are set: for the organization and for every project.
    #[must_use]
    pub fn of_roles(&self, roles: &BTreeSet<RoleId>) -> Self {
        let applies = |role: RoleId, _project: Option<ProjectId>| roles.contains(&role);
        Self {
            fields: filtered(&self.fields, applies),
            people: filtered(&self.people, applies),
            typed: self
                .typed
                .iter()
                .filter(|rule| applies(rule.role, rule.project))
                .copied()
                .collect(),
            own_fields: self.own_fields.clone(),
        }
    }

    /// The rules of these roles that apply at a place: their own, and those set for a project of
    /// `chain`. A role's rule for any other project is no part of the place.
    #[must_use]
    pub fn at(&self, roles: &BTreeSet<RoleId>, chain: &[ProjectId]) -> Self {
        let applies = |role: RoleId, project: Option<ProjectId>| {
            roles.contains(&role) && reaches(project, chain)
        };
        Self {
            fields: filtered(&self.fields, applies),
            people: filtered(&self.people, applies),
            typed: self
                .typed
                .iter()
                .filter(|rule| applies(rule.role, rule.project))
                .copied()
                .collect(),
            own_fields: self.own_fields.clone(),
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

pub(crate) use bb_core::lookup::chain_depth;

/// The nearest rule of a role: the project of the chain closest to the place that has one, else
/// the role's own. The one walk of steps 7 and 8 of the evaluation, also read by the tables of
/// what roles see and by the check of what a level becomes when a project's own is taken away.
#[must_use]
pub fn nearest_rule<R: Copy>(
    chain: &[ProjectId],
    rules: impl Iterator<Item = (Option<ProjectId>, R)>,
) -> Option<(LevelSource, R)> {
    nearest_typed_rule(
        chain,
        None,
        rules.map(|(project, rule)| (project, None, rule)),
    )
    .map(|(source, _, rule)| (source, rule))
}

/// [`nearest_rule`] for an issue type (ADR-0027): at one place a rule for the issue type wins over
/// a rule for all types. The second part of the answer says whether the winner is for the issue
/// type itself. The walk is `bb_core::lookup::nearest_row`, which layouts use too.
#[must_use]
pub fn nearest_typed_rule<R>(
    chain: &[ProjectId],
    issue_type: Option<IssueTypeId>,
    rules: impl Iterator<Item = (Option<ProjectId>, Option<IssueTypeId>, R)>,
) -> Option<(LevelSource, bool, R)> {
    nearest_row(chain, issue_type, rules).map(|(project, typed, rule)| {
        let source = project.map_or(LevelSource::RoleOwn, LevelSource::Project);
        (source, typed, rule)
    })
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
    pub(crate) fn of_roles(chain: &[ProjectId], rules: &LevelRules, held: &[&RoleData]) -> Self {
        let mut highest = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let own = K::rules_in(rules);
        for subject in K::universe(rules) {
            let mut top = K::CLOSED;
            let mut per_role = Vec::with_capacity(held.len());
            for role in held {
                let (source, level) = level_of_role(chain, own, role.id, subject)
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
