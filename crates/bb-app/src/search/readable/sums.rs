//! What a total may group by and add up (QRY-08, ADR-0050 decision 7): where the person may read a
//! field that a total sums, by the field levels of each place a search looks at, and the fields of
//! people a total may group by though their names are not shown everywhere (ADR-0022).

use std::sync::Arc;

use bb_core::fields::{Field, FieldKey, word_key};
use bb_core::id::FieldId;
use bb_core::own_fields::FieldType;
use bb_policy::Evaluation;

use super::{
    Definition, FieldTarget, Looked, OwnReach, Place, Searchable, Sources, catalogue_field,
    catalogue_target, own_search_field, own_words, places_in, reached_places,
};

/// The places a search looks at, as `looked` holds them.
fn places_looked(looked: &Looked) -> Vec<Place> {
    places_in(&looked.seen, &looked.projects)
}

/// A field a total adds up: a length of time of the catalogue, or an own field of numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Summable {
    /// *Estimated time*, *Time left* or *Time spent*.
    Length(Field),
    /// An own field of whole numbers or decimals.
    Number(FieldId),
}

impl Looked {
    /// The own field of numbers with this word, whether or not the person may read it: who may
    /// read it where is [`Looked::readable_places`].
    pub(crate) fn number_field(&self, word: &str) -> Option<(FieldId, FieldType)> {
        self.own_by_word(word)
            .map(|(definition, _)| (definition.field.id, definition.field.kind))
            .filter(|(_, kind)| bb_core::totals::SUMMED_KINDS.contains(kind))
    }

    /// The own field that owns this word of the search language, with the word, whether or not
    /// the person may read it.
    fn own_by_word(&self, word: &str) -> Option<(&Definition, String)> {
        let wanted = word_key(word);
        self.own
            .iter()
            .zip(own_words(&self.own))
            .find_map(|(definition, owned)| {
                owned
                    .filter(|each| word_key(each) == wanted)
                    .map(|each| (definition, each))
            })
    }

    /// Where the person may read a field that a total adds up, on the issues of the search (PRIV-13,
    /// SEC-04): each place the search looks at where they may read it, with the issue types they
    /// may read it on there when it is not all of them. An own field counts only where it is used.
    /// The one rule of the totals for the field levels of each project; empty for a field they may
    /// read nowhere.
    pub(crate) fn readable_places(&self, field: Summable) -> Vec<crate::search::FieldPlace> {
        let (key, definition) = match field {
            Summable::Length(field) => (FieldKey::from(field), None),
            Summable::Number(id) => (
                FieldKey::Own(id),
                self.own.iter().find(|each| each.field.id == id),
            ),
        };
        if matches!(field, Summable::Number(_)) && definition.is_none() {
            return Vec::new();
        }
        places_looked(self)
            .into_iter()
            .filter(|place| definition.is_none_or(|each| each.scope.reaches_project(&place.chain)))
            .filter_map(|place| {
                let project = *place.chain.last()?;
                if place.mask.can_see(key) {
                    return Some(crate::search::FieldPlace {
                        project,
                        types: None,
                    });
                }
                let types: Vec<_> = self
                    .types
                    .iter()
                    .map(|each| each.id)
                    .filter(|each| place.held.field_mask_for(Some(*each)).can_see(key))
                    .collect();
                (!types.is_empty()).then_some(crate::search::FieldPlace {
                    project,
                    types: Some(types),
                })
            })
            .collect()
    }

    /// Where the values of a summed own field count: the projects of the search it is used in, and
    /// its issue types.
    pub(crate) fn reach_of(&self, field: FieldId) -> Option<OwnReach> {
        let definition = self.own.iter().find(|each| each.field.id == field)?;
        Some(OwnReach {
            field,
            projects: places_looked(self)
                .iter()
                .filter(|place| definition.scope.reaches_project(&place.chain))
                .filter_map(|place| place.chain.last().copied())
                .collect(),
            issue_types: definition.scope.limited_types().map(<[_]>::to_vec),
        })
    }

    /// A field of people the person may see everywhere the search looks but whose names they do
    /// not see in every place (ADR-0022): a total may still be grouped by it, each person shown as
    /// the place that shows the least shows them. The field, and what the person may do in that
    /// place. None for a field that is no such field.
    pub(crate) fn people_without_names(&self, word: &str) -> Option<(Searchable, Arc<Evaluation>)> {
        let places = places_looked(self);
        let wanted = word_key(word);
        let sources = Sources {
            statuses: Vec::new(),
            issue_types: Vec::new(),
            own: Vec::new(),
        };
        let built_in = [Field::AssignedTo, Field::ReportedBy]
            .into_iter()
            .find(|field| word_key(&field.search_word()) == wanted)
            .and_then(|field| {
                let (target, kind) = catalogue_target(field)?;
                Some((
                    Searchable {
                        field: catalogue_field(field, kind, &sources),
                        target,
                    },
                    FieldKey::from(field),
                    places.iter().collect::<Vec<_>>(),
                ))
            });
        let found = built_in.or_else(|| {
            let (definition, word) = self.own_by_word(word)?;
            Some((
                Searchable {
                    field: own_search_field(&word, definition),
                    target: FieldTarget::Own {
                        field: definition.field.id,
                        kind: definition.field.kind,
                    },
                },
                FieldKey::Own(definition.field.id),
                reached_places(&places, definition).collect(),
            ))
        });
        let (searchable, key, reached) = found?;
        let kind = searchable.target.people_kind()?;
        if reached.is_empty() || !reached.iter().all(|place| place.may_use(key, None)) {
            return None;
        }
        reached
            .into_iter()
            .filter(|place| !place.held.sees_names(kind))
            .min_by_key(|place| {
                place
                    .held
                    .people_level(kind, bb_policy::PeopleContext::Ordinary)
            })
            .map(|place| (searchable, Arc::clone(&place.held)))
    }
}
