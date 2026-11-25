//! What the totals of a search ask the store, and how the people of the groups are made one group
//! where the reader sees them alike.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_policy::{Evaluation, Place as Where};

use super::super::readable::Summable;
use super::super::request::{
    FieldPlace, GroupBy, GroupKey, GroupPage, ReadableWhere, SUMS_MAX, SearchStore, SumAsked,
    Total, Totals, TotalsAsked,
};
use super::super::run::{Prepared, ShownKey, stopped};
use super::super::target::{FieldTarget, target_of};
use super::SumWord;
use super::shown::Unit;
use crate::access::Actor;
use crate::ai::Withheld;
use crate::auth::{AppError, AuthStore, Ctx};

/// The field a total is grouped by, as the person may use it.
pub(crate) struct Grouping {
    /// The word of the field.
    pub(crate) word: String,
    /// What it reads.
    pub(crate) target: FieldTarget,
    /// What the person may do where the people of its groups are shown: the whole organization,
    /// or for a field of people whose names are not shown everywhere, the place that shows least.
    pub(crate) viewer: Arc<Evaluation>,
}

impl Grouping {
    /// Whether the groups are people, who stand in the order of their names (PRIV-09).
    pub(crate) fn of_people(&self) -> bool {
        self.target.people_kind().is_some()
    }
}

/// The groups of a total, each shown as the reader may see it, and the total of all.
pub(crate) struct Counted {
    pub(crate) groups: Vec<(ShownKey, Total)>,
    pub(crate) all: Total,
}

/// The key of a group as the store gave it, and its key of the second field ([`GroupBy::then`]).
pub(crate) type KeyPair = (GroupKey, Option<GroupKey>);

/// The groups of a total each shown as the reader may see it, with the keys the store gave them
/// (in the same order) and the grouping that made them (people shown alike one group).
pub(crate) struct ShownGroups {
    pub(crate) counted: Counted,
    pub(crate) keys: Vec<KeyPair>,
    pub(crate) by: GroupBy,
}

/// The sums of a total, as the use case asks them and shows them.
pub(super) struct Sums {
    /// The words of the sums, in the order of the answer.
    pub(super) words: Vec<String>,
    /// What each word is in, and the place of its sum among those the store adds up.
    pub(super) units: Vec<Unit>,
    /// The sums the store adds up.
    pub(super) asked: Vec<SumAsked>,
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The field `by` as the person may group by it: a field they may search by, or a field of
    /// people they see but not by name everywhere, whose people are shown as the place that shows
    /// least shows them (the field is then added to the fields of the request). A field kept from
    /// AI is refused for the AI like one that does not exist.
    ///
    /// # Errors
    /// `request.invalid` for a field the person may not group by.
    pub(crate) async fn grouping_of(
        &self,
        actor: &Actor,
        prepared: &mut Prepared,
        by: &str,
        withheld: Option<&Withheld>,
    ) -> Result<Grouping, AppError> {
        let refused = || AppError::Refused(ErrorCode::InvalidRequest);
        let kept = |target: FieldTarget| withheld.is_some_and(|kept| kept.hides_target(target));
        if let Some(target) = target_of(&prepared.looked.fields, by).filter(|each| !kept(*each)) {
            return Ok(Grouping {
                word: by.to_owned(),
                target,
                viewer: self.evaluation_of(actor, Where::Organization).await?,
            });
        }
        let (searchable, viewer) = prepared
            .looked
            .people_without_names(by)
            .filter(|(searchable, _)| !kept(searchable.target))
            .ok_or_else(refused)?;
        let target = searchable.target;
        prepared.request.fields.push(searchable);
        Ok(Grouping {
            word: by.to_owned(),
            target,
            viewer,
        })
    }

    /// The totals the store adds up for `asked`.
    pub(crate) async fn totals_in(
        &self,
        prepared: &Prepared,
        asked: &TotalsAsked,
    ) -> Result<Totals, AppError> {
        self.store()
            .search_totals(&prepared.request, asked)
            .await
            .map_err(stopped)
    }

    /// The totals of a field that is not people, with the groups read as `page` says; the keys
    /// are shown by the caller, for the groups it gives out.
    pub(super) async fn key_totals(
        &self,
        prepared: &Prepared,
        grouping: &Grouping,
        (sums, all): (&[SumAsked], bool),
        page: Option<GroupPage>,
    ) -> Result<Totals, AppError> {
        let asked = TotalsAsked {
            by: Some(GroupBy::of(grouping.word.clone()).paged(page)),
            sums: sums.to_vec(),
            all,
        };
        self.totals_in(prepared, &asked).await
    }

    /// The totals of a field of people, each person shown as the reader may see them. The people
    /// are read first without sums; people shown alike (by one role, or as *Someone*) are then one
    /// group each, counted again by the statement so an issue of two of them counts once. The sums
    /// and the total of all, when asked for, come with that second read.
    pub(crate) async fn people_totals(
        &self,
        prepared: &Prepared,
        grouping: &Grouping,
        (sums, all): (&[SumAsked], bool),
    ) -> Result<Counted, AppError> {
        let by = GroupBy::of(grouping.word.clone());
        Ok(self
            .shown_groups(prepared, grouping, by, (sums, all))
            .await?
            .counted)
    }

    /// The totals of `grouping` split again by the field of one value `then` (the cells of a
    /// board), each group shown as the reader may see it (people shown alike one group), with the
    /// keys the store gave each group and the grouping that made them.
    ///
    /// # Errors
    /// `search.too-slow`.
    pub(crate) async fn cell_totals(
        &self,
        prepared: &Prepared,
        grouping: &Grouping,
        then: &str,
    ) -> Result<ShownGroups, AppError> {
        let by = GroupBy::of(grouping.word.clone()).split_by(then);
        self.shown_groups(prepared, grouping, by, (&[], false))
            .await
    }

    /// The groups of `by`, each shown as the reader may see it: read first without sums; when the
    /// groups are people and some are shown alike, those are one group each, counted again by the
    /// statement so an issue of two of them counts once. The sums and the total of all, when asked
    /// for, come with that second read.
    async fn shown_groups(
        &self,
        prepared: &Prepared,
        grouping: &Grouping,
        mut by: GroupBy,
        (sums, all): (&[SumAsked], bool),
    ) -> Result<ShownGroups, AppError> {
        let first = TotalsAsked {
            by: Some(by.clone()),
            sums: Vec::new(),
            all: false,
        };
        let found = self.totals_in(prepared, &first).await?;
        // Each group once, though a second field splits it into several rows of the totals.
        let mut keys: Vec<GroupKey> = Vec::new();
        for group in &found.groups {
            if !keys.contains(&group.key) {
                keys.push(group.key.clone());
            }
        }
        let shown = self
            .shown_keys(&grouping.viewer, &prepared.looked, grouping.target, &keys)
            .await?;
        let sets = grouping
            .of_people()
            .then(|| people_sets(&keys, &shown))
            .flatten();
        let (totals, set_keys) = if sets.is_none() && sums.is_empty() && !all {
            (found, Vec::new())
        } else {
            let (set_keys, people): (Vec<ShownKey>, Vec<Vec<UserId>>) =
                sets.unwrap_or_default().into_iter().unzip();
            by.people = people;
            let asked = TotalsAsked {
                by: Some(by.clone()),
                sums: sums.to_vec(),
                all,
            };
            (self.totals_in(prepared, &asked).await?, set_keys)
        };
        // Without sets every group is shown as in the first read.
        let shown_of = |key: &GroupKey| match key {
            GroupKey::People(at) => set_keys.get(usize::try_from(*at).ok()?).cloned(),
            other => keys
                .iter()
                .position(|each| each == other)
                .and_then(|at| shown.get(at).cloned())
                .or_else(|| Some(ShownKey::Other(other.clone()))),
        };
        let (groups, raw) = totals
            .groups
            .into_iter()
            .filter_map(|group| {
                let shown = shown_of(&group.key)?;
                Some(((shown, group.total), (group.key, group.then)))
            })
            .unzip();
        Ok(ShownGroups {
            counted: Counted {
                groups,
                all: totals.all,
            },
            keys: raw,
            by,
        })
    }

    /// Every group of `grouping` without sums, each shown as the reader may see it: the counts of
    /// the groups of a list.
    ///
    /// # Errors
    /// `search.too-slow`.
    pub(crate) async fn group_counts(
        &self,
        prepared: &Prepared,
        grouping: &Grouping,
    ) -> Result<Vec<(ShownKey, Total)>, AppError> {
        if grouping.of_people() {
            return Ok(self
                .people_totals(prepared, grouping, (&[], false))
                .await?
                .groups);
        }
        let totals = self
            .key_totals(prepared, grouping, (&[], false), None)
            .await?;
        let keys: Vec<GroupKey> = totals
            .groups
            .iter()
            .map(|group| group.key.clone())
            .collect();
        let shown = self
            .shown_keys(&grouping.viewer, &prepared.looked, grouping.target, &keys)
            .await?;
        Ok(shown
            .into_iter()
            .zip(totals.groups)
            .map(|(key, group)| (key, group.total))
            .collect())
    }
}

/// The sums of the words: each with where the reader may read it. A word that names no sum, a
/// word asked twice, more sums than a total may ask for, a field the reader may read nowhere and a
/// field kept from AI are refused alike. The own fields that are added up have their reach added
/// to the request.
pub(super) fn sums_of(
    prepared: &mut Prepared,
    words: &[String],
    withheld: Option<&Withheld>,
) -> Result<Sums, AppError> {
    let refused = || AppError::Refused(ErrorCode::InvalidRequest);
    let read: Vec<SumWord> = words
        .iter()
        .map(|word| SumWord::read(word).ok_or_else(refused))
        .collect::<Result<_, _>>()?;
    let twice = read
        .iter()
        .enumerate()
        .any(|(at, word)| read[..at].contains(word));
    if twice || read.iter().filter(|each| **each != SumWord::Count).count() > SUMS_MAX {
        return Err(refused());
    }
    let looked = &prepared.looked;
    let mut reaches = Vec::new();
    let mut sums = Sums {
        words: read.iter().map(SumWord::word).collect(),
        units: Vec::with_capacity(read.len()),
        asked: Vec::new(),
    };
    for word in read {
        let (target, summable) = match &word {
            SumWord::Count => {
                sums.units.push(Unit::Count);
                continue;
            }
            SumWord::Estimated => (
                FieldTarget::EstimatedTime,
                Summable::Length(Field::EstimatedTime),
            ),
            SumWord::TimeLeft => (FieldTarget::TimeLeft, Summable::Length(Field::TimeLeft)),
            SumWord::TimeSpent => (FieldTarget::TimeSpent, Summable::Length(Field::TimeSpent)),
            SumWord::Number(field) => {
                let (id, kind) = looked.number_field(field).ok_or_else(refused)?;
                reaches.extend(looked.reach_of(id));
                (FieldTarget::Own { field: id, kind }, Summable::Number(id))
            }
        };
        let places: Vec<FieldPlace> = looked.readable_places(summable);
        if places.is_empty() || withheld.is_some_and(|kept| kept.hides_target(target)) {
            return Err(refused());
        }
        let at = sums.asked.len();
        sums.units.push(match target {
            FieldTarget::Own { kind, .. } => Unit::Own(at, kind),
            _ => Unit::Minutes(at),
        });
        sums.asked.push(SumAsked {
            target,
            readable: ReadableWhere::In(places),
        });
    }
    for reach in reaches {
        if !prepared
            .request
            .own_reach
            .iter()
            .any(|each| each.field == reach.field)
        {
            prepared.request.own_reach.push(reach);
        }
    }
    Ok(sums)
}

/// The people of the groups as sets of the people shown alike, with how they are shown; none
/// while every person is shown on their own.
fn people_sets(keys: &[GroupKey], shown: &[ShownKey]) -> Option<Vec<(ShownKey, Vec<UserId>)>> {
    let mut sets: Vec<(ShownKey, Vec<UserId>)> = Vec::new();
    for (key, shown) in keys.iter().zip(shown) {
        let GroupKey::Person(person) = key else {
            continue;
        };
        match sets.iter_mut().find(|(each, _)| each == shown) {
            Some((_, people)) => people.push(*person),
            None => sets.push((shown.clone(), vec![*person])),
        }
    }
    sets.iter()
        .any(|(_, people)| people.len() > 1)
        .then_some(sets)
}
