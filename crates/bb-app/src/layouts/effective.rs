//! What a layout shows to one person: the rule "field permissions win" (ADR-0027, decision 4).
//!
//! A field is shown only when **both** halves hold: the layout lists it for one of the person's
//! roles, and the level of the field lets the person see it (on the new-issue form: change it).
//! The second half is the policy engine's answer, asked through the mask of the person
//! (`Evaluation::field_mask_for`) and through the own fields that the same functions of the field
//! use cases already judged (`changeable_own_fields`, `visible_own_fields`), never restated.

use std::collections::BTreeSet;

use bb_core::fields::FieldKey;
use bb_core::id::RoleId;
use bb_core::layout::{LayoutContent, LayoutKind, built_in_fields};
use bb_policy::FieldMask;

use super::types::FormSection;

/// The person a form is made for.
pub(super) struct Viewer<'a> {
    /// The roles that count for the person at the place of the issue.
    pub(super) roles: &'a [RoleId],
    /// What the person may do with each built-in field on an issue of the type.
    pub(super) mask: &'a FieldMask,
    /// The own fields that reach the issue and that the level lets the person see, or fill in on
    /// the new-issue form. An own field that is not here is not shown, whatever the layout says.
    pub(super) own: &'a BTreeSet<FieldKey>,
    /// Own fields that must be filled in: they stay on the new-issue form.
    pub(super) required: &'a [FieldKey],
}

/// Whether the level of the field lets the person see it, or fill it in on the new-issue form.
pub(super) fn level_allows(kind: LayoutKind, field: FieldKey, viewer: &Viewer<'_>) -> bool {
    match field {
        FieldKey::BuiltIn(built_in) => {
            built_in_fields(kind).contains(&built_in)
                && match kind {
                    LayoutKind::IssuePage => viewer.mask.can_see(field),
                    LayoutKind::NewIssue => viewer.mask.can_change(field),
                }
        }
        FieldKey::Own(_) => viewer.own.contains(&field),
    }
}

/// The sections of the form for this person: the fields the layout lists for one of their roles
/// and that the level lets them see or fill in; sections left empty are dropped. A required own
/// field the person can fill in is always on the new-issue form, as the last field of the last
/// section, so a form can always be filled in whatever the layout says.
pub(super) fn effective(
    content: &LayoutContent,
    kind: LayoutKind,
    viewer: &Viewer<'_>,
) -> Vec<FormSection> {
    let mut sections: Vec<FormSection> = content
        .sections
        .iter()
        .map(|section| FormSection {
            tab: section.tab.clone(),
            title: section.title.clone(),
            fields: section
                .fields
                .iter()
                .filter(|placed| {
                    placed.roles.iter().any(|role| viewer.roles.contains(role))
                        && level_allows(kind, placed.field, viewer)
                })
                .map(|placed| placed.field)
                .collect(),
        })
        .filter(|section| !section.fields.is_empty())
        .collect();
    if kind == LayoutKind::NewIssue {
        let missing: Vec<FieldKey> = viewer
            .required
            .iter()
            .copied()
            .filter(|field| {
                level_allows(kind, *field, viewer)
                    && !sections
                        .iter()
                        .any(|section| section.fields.contains(field))
            })
            .collect();
        if !missing.is_empty() {
            match sections.last_mut() {
                Some(last) => last.fields.extend(missing),
                None => sections.push(FormSection {
                    tab: String::new(),
                    title: String::new(),
                    fields: missing,
                }),
            }
        }
    }
    sections
}

#[cfg(test)]
mod tests {
    use bb_core::access::{AppliesTo, FieldLevel};
    use bb_core::fields::Field;
    use bb_core::id::{AssignmentId, FieldId, SequentialIds, UserId};
    use bb_core::layout::{PlacedField, SectionContent};
    use bb_policy::{
        Evaluation, FieldRule, Inputs, LevelRules, Permission, Place, RoleData, Row, Who, evaluate,
    };
    use proptest::prelude::*;

    use super::*;

    const ROLES: usize = 3;
    const FIELDS: [Field; 5] = [
        Field::Description,
        Field::Status,
        Field::Priority,
        Field::AssignedTo,
        Field::ReportedBy,
    ];

    fn role_ids() -> [RoleId; ROLES] {
        let mut ids = SequentialIds::default();
        std::array::from_fn(|_| RoleId::generate(&mut ids).unwrap())
    }

    /// One random world: which roles the person holds, the level each role has for each field,
    /// and a layout with the roles each field is shown to.
    #[derive(Clone, Debug)]
    struct World {
        held: Vec<bool>,
        levels: Vec<Vec<usize>>,
        /// For each field: the section it is in (or none) and the roles it is shown to.
        placed: Vec<(Option<usize>, Vec<bool>)>,
        new_issue: bool,
    }

    fn world() -> impl Strategy<Value = World> {
        (
            prop::collection::vec(any::<bool>(), ROLES),
            prop::collection::vec(prop::collection::vec(0..3_usize, FIELDS.len()), ROLES),
            prop::collection::vec(
                (
                    prop::option::of(0..3_usize),
                    prop::collection::vec(any::<bool>(), ROLES),
                ),
                FIELDS.len(),
            ),
            any::<bool>(),
        )
            .prop_map(|(held, levels, placed, new_issue)| World {
                held,
                levels,
                placed,
                new_issue,
            })
    }

    fn evaluation_of(world: &World, roles: &[RoleId; ROLES]) -> Evaluation {
        let mut ids = SequentialIds::default();
        let person = UserId::generate(&mut ids);
        let rows = roles
            .iter()
            .zip(&world.held)
            .filter(|(_, held)| **held)
            .map(|(role, _)| Row {
                id: AssignmentId::generate(&mut ids).unwrap(),
                who: Who::Person,
                role: Some(*role),
                project: None,
                applies_to: AppliesTo::Organization,
            })
            .collect();
        let levels = LevelRules {
            fields: roles
                .iter()
                .zip(&world.levels)
                .flat_map(|(role, levels)| {
                    FIELDS.iter().zip(levels).map(|(field, level)| FieldRule {
                        project: None,
                        role: *role,
                        subject: (*field).into(),
                        level: FieldLevel::ALL[*level],
                    })
                })
                .collect(),
            ..LevelRules::default()
        };
        evaluate(&Inputs {
            person,
            place: Place::Organization,
            chain: Vec::new(),
            rows,
            roles: roles
                .iter()
                .enumerate()
                .map(|(at, id)| RoleData {
                    id: *id,
                    name: format!("Role {at}"),
                    position: i64::try_from(at).unwrap(),
                    permissions: BTreeSet::from([Permission::SeeProjects]),
                    two_step: false,
                })
                .collect(),
            rules: levels,
        })
    }

    fn content_of(world: &World, roles: &[RoleId; ROLES], kind: LayoutKind) -> LayoutContent {
        let mut sections = vec![
            SectionContent {
                tab: String::new(),
                title: String::new(),
                fields: Vec::new(),
            };
            3
        ];
        for (field, (section, shown)) in FIELDS.iter().zip(&world.placed) {
            // A field a form cannot place is not in a stored layout (`LayoutContent::check`).
            if !built_in_fields(kind).contains(field) {
                continue;
            }
            if let Some(at) = section {
                sections[*at].fields.push(PlacedField {
                    field: (*field).into(),
                    roles: roles
                        .iter()
                        .zip(shown)
                        .filter(|(_, on)| **on)
                        .map(|(role, _)| *role)
                        .collect(),
                });
            }
        }
        LayoutContent::of_sections(sections)
    }

    proptest! {
        /// CFG-01: whatever the layout lists, a person is shown a field only when one of their
        /// roles is ticked for it and their own level (the highest of their roles, read from the
        /// levels the world was made from and not from the engine) lets them see it, or fill it
        /// in on the new-issue form. And the other way round: nothing that passes both halves is
        /// lost.
        #[test]
        fn cfg_01_a_layout_never_shows_a_field_the_role_cannot_see(world in world()) {
            let roles = role_ids();
            let kind = if world.new_issue { LayoutKind::NewIssue } else { LayoutKind::IssuePage };
            let held = evaluation_of(&world, &roles);
            let content = content_of(&world, &roles, kind);
            let mine: Vec<RoleId> = held.roles().iter().map(|role| role.id).collect();
            let mask = held.field_mask_for(None);
            let own = BTreeSet::new();
            let viewer = Viewer { roles: &mine, mask: &mask, own: &own, required: &[] };
            let shown: Vec<FieldKey> = effective(&content, kind, &viewer)
                .into_iter()
                .flat_map(|section| section.fields)
                .collect();
            let needed = match kind {
                LayoutKind::IssuePage => FieldLevel::CanSee,
                LayoutKind::NewIssue => FieldLevel::CanChange,
            };
            for (at, field) in FIELDS.iter().enumerate() {
                let best = roles
                    .iter()
                    .zip(&world.held)
                    .zip(&world.levels)
                    .filter(|((_, held), _)| **held)
                    .map(|(_, levels)| FieldLevel::ALL[levels[at]])
                    .max();
                let ticked = content.placed().any(|placed| {
                    placed.field == FieldKey::from(*field)
                        && placed.roles.iter().any(|role| mine.contains(role))
                });
                let allowed = best.is_some_and(|level| level >= needed)
                    && built_in_fields(kind).contains(field);
                prop_assert_eq!(
                    shown.contains(&FieldKey::from(*field)),
                    ticked && allowed,
                    "{:?} on {:?}", field, kind
                );
            }
        }
    }

    #[test]
    fn a_required_field_stays_on_the_new_issue_form_for_everybody_who_can_fill_it_in() {
        let roles = role_ids();
        let world = World {
            held: vec![true, false, false],
            levels: vec![
                vec![2; FIELDS.len()],
                vec![0; FIELDS.len()],
                vec![0; FIELDS.len()],
            ],
            placed: vec![(None, vec![false; ROLES]); FIELDS.len()],
            new_issue: true,
        };
        let held = evaluation_of(&world, &roles);
        let mask = held.field_mask_for(None);
        let mine = vec![roles[0]];
        let mut ids = SequentialIds::default();
        let required = FieldKey::Own(FieldId::generate(&mut ids).unwrap());
        let own = BTreeSet::from([required]);
        let viewer = Viewer {
            roles: &mine,
            mask: &mask,
            own: &own,
            required: &[required],
        };
        let sections = effective(&LayoutContent::default(), LayoutKind::NewIssue, &viewer);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].fields, vec![required]);
        // Somebody who may not fill it in is not asked for it (the server does not ask either).
        let none = BTreeSet::new();
        let viewer = Viewer {
            roles: &mine,
            mask: &mask,
            own: &none,
            required: &[required],
        };
        assert!(effective(&LayoutContent::default(), LayoutKind::NewIssue, &viewer).is_empty());
    }
}
