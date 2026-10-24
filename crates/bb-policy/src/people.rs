//! `PersonRef` and *Who sees people* (PRIV-12, PRIV-13, ADR-0022, ADR-0025 decision 11,
//! architecture §7 *How `PersonRef` resolves*).
//!
//! Every person in an answer is a [`PersonRef`], resolved for the viewer, the kind of people
//! information and the place. A role label and *Someone* carry no ID of the person, so a browser
//! cannot tell two hidden people apart or follow one through the answers. E-mail addresses are
//! part of no `PersonRef`. Enforced by: ARCH-PERSON-REF.

use std::collections::BTreeSet;

use bb_core::access::PeopleLevel;
use bb_core::id::{IdSource, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::text::CodeDraws;
use serde::{Deserialize, Serialize};

use crate::catalogue::Permission;
use crate::evaluate::Evaluation;
use crate::mask::Masked;

/// How a person is shown to one viewer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "shown", rename_all = "lowercase")]
pub enum PersonRef {
    /// By name, with the ID for links.
    Name {
        /// The ID of the person.
        #[cfg_attr(feature = "openapi", schema(value_type = String))]
        id: UserId,
        /// The name.
        name: String,
    },
    /// By the label of their role; the screen says "a {role}". No ID.
    Role {
        /// The name of the role.
        role: String,
    },
    /// As *Someone*. Nothing else.
    Someone,
    /// A removed person: the screen says "Former user {code}". No ID.
    Former {
        /// The random code, like `K7Q2`.
        code: String,
    },
}

/// A person holds no field of the catalogue: a list of them is masked as it is.
impl Masked for PersonRef {}

/// Where a person is shown: in an ordinary answer, or in a screen where the people who manage
/// people or roles work (*Organization settings*, *Who has which role*, *Check access*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeopleContext {
    /// An ordinary place: the viewer's level for the kind counts.
    Ordinary,
    /// A management screen (M2-Q3): people who manage see names, everybody else sees the
    /// people there by the kind *Project members*.
    Management,
}

/// The permissions that make a viewer a manager of people or roles at a place (M2-Q3).
const MANAGERS: [Permission; 4] = [
    Permission::ManagePeople,
    Permission::ManageGroups,
    Permission::ManageRoles,
    Permission::DecideRoles,
];

impl Evaluation {
    /// Whether the viewer manages people or roles here (*Manage people*, *Manage groups*, *Manage
    /// roles* or *Decide who has which role*): they always see names in the management screens
    /// and see everybody there (M2-Q3).
    #[must_use]
    pub fn manages_people(&self) -> bool {
        MANAGERS.iter().any(|permission| self.allows(*permission))
    }

    /// Whether the viewer is shown which roles the people of a project hold (*Who sees people* for
    /// project members is at least *Role only*). The one answer for every screen that names the
    /// roles of people: *Who may change*, *Try it*.
    #[must_use]
    pub fn may_know_roles_of_people(&self) -> bool {
        self.kind_level(PeopleKind::ProjectMembers) >= PeopleLevel::RoleOnly
    }

    /// How the viewer sees people of `kind` here. The viewer's level is the most open of their
    /// roles; in a management screen a person who manages people or roles always sees names.
    #[must_use]
    pub fn people_level(&self, kind: PeopleKind, context: PeopleContext) -> PeopleLevel {
        match context {
            PeopleContext::Ordinary => self.kind_level(kind),
            PeopleContext::Management => {
                if self.manages_people() {
                    PeopleLevel::Names
                } else {
                    self.kind_level(PeopleKind::ProjectMembers)
                }
            }
        }
    }

    /// Whether the viewer is shown the role label of `person` as `kind` says: when they see people
    /// by their role there, and the person is not themselves (they see their own name). The one
    /// rule: [`resolve`] shows a role label exactly when this is true, and a caller that looks the
    /// label up asks it first, so nobody's rights are evaluated for a label that is never shown.
    #[must_use]
    pub fn shows_role_label(
        &self,
        kind: PeopleKind,
        context: PeopleContext,
        person: UserId,
    ) -> bool {
        self.person() != Some(person) && self.people_level(kind, context) == PeopleLevel::RoleOnly
    }

    /// Whether a picture of a person may be shown: only where the kind of the place also gives
    /// *Names*.
    #[must_use]
    pub fn shows_picture(&self, kind: PeopleKind, context: PeopleContext) -> bool {
        self.kind_level(PeopleKind::Pictures) == PeopleLevel::Names
            && self.people_level(kind, context) == PeopleLevel::Names
    }
}

/// A removed person, as the viewer needs to know them (PRIV-04, M9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormerUser {
    /// The random code stored with the tombstone.
    pub label: String,
}

/// The person to show, with what the store knows of them.
#[derive(Clone, Copy, Debug)]
pub struct Target<'a> {
    /// The ID of the person.
    pub id: UserId,
    /// Their name.
    pub name: &'a str,
    /// The label of their first role in the order of roles at the place, if they have one.
    pub role: Option<&'a str>,
    /// Set when the person was removed.
    pub removed: Option<&'a FormerUser>,
}

/// Shows `target` to `viewer` as `kind` says (steps 1 to 3 of *How `PersonRef` resolves*).
#[must_use]
pub fn resolve(
    viewer: &Evaluation,
    kind: PeopleKind,
    context: PeopleContext,
    target: Target<'_>,
) -> PersonRef {
    if viewer.person() == Some(target.id) {
        return PersonRef::Name {
            id: target.id,
            name: target.name.to_owned(),
        };
    }
    let level = viewer.people_level(kind, context);
    if let Some(former) = target.removed {
        return if level == PeopleLevel::Names {
            PersonRef::Former {
                code: former.label.clone(),
            }
        } else {
            PersonRef::Someone
        };
    }
    if viewer.shows_role_label(kind, context, target.id) {
        return target
            .role
            .map_or(PersonRef::Someone, |role| PersonRef::Role {
                role: role.to_owned(),
            });
    }
    match level {
        PeopleLevel::Names => PersonRef::Name {
            id: target.id,
            name: target.name.to_owned(),
        },
        PeopleLevel::RoleOnly | PeopleLevel::Hidden => PersonRef::Someone,
    }
}

/// The length of a label when there is room: four characters, 32^4 = over a million labels.
const LABEL_LENGTH: usize = 4;
/// The longest label.
const LABEL_LENGTH_MAX: usize = 12;

/// Makes the random code of a *Former user* (PRIV-13): four characters of Crockford Base32 from
/// the secure random source, drawn again on a clash and longer when four run short. Never made
/// from the person's ID or from a counter, so labels do not count up and cannot be traced back.
///
/// `taken` says whether a label belongs to a removed person already. `None` only when even the
/// longest labels clash, which would mean the random source is broken.
#[must_use]
pub fn former_user_label(
    source: &mut impl IdSource,
    taken: &dyn Fn(&str) -> bool,
) -> Option<String> {
    CodeDraws::new(source, LABEL_LENGTH, LABEL_LENGTH_MAX).find(|label| !taken(label))
}

/// The set of labels as the store's rows give them, for [`former_user_label`].
pub fn taken_in(labels: &BTreeSet<String>) -> impl Fn(&str) -> bool + '_ {
    move |label| labels.contains(label)
}
