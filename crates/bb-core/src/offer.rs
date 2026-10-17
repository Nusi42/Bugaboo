//! Where an issue type or a group of labels is offered (ISS-02, TAG-02, ADR-0027 amendment).
//!
//! Each place says it with a row: the whole organization or a project, one issue type or one group
//! of labels, and whether it is offered there. The nearest place with a row wins and a sub-project
//! follows its parent until it has a row of its own (`lookup::nearest_row`, the walk of layouts and
//! workflows). Where no row says anything, an organization-wide subject is offered everywhere and a
//! subject made in a project is offered in that project and below.

use crate::id::{IssueTypeId, LabelGroupId, ProjectId};
use crate::issue::IssueType;
use crate::lookup::{chain_depth, nearest_row};
use crate::personal::Entity;

words! {
    /// What can be offered in some places and not in others.
    pub enum OfferKind {
        /// An issue type.
        IssueType => "issue-type",
        /// A group of labels, with its labels.
        LabelGroup => "label-group",
    }
}

/// The issue type or the group of labels a row is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OfferSubject {
    /// An issue type.
    IssueType(IssueTypeId),
    /// A group of labels.
    LabelGroup(LabelGroupId),
}

impl OfferSubject {
    /// What kind of subject this is.
    #[must_use]
    pub const fn kind(self) -> OfferKind {
        match self {
            Self::IssueType(_) => OfferKind::IssueType,
            Self::LabelGroup(_) => OfferKind::LabelGroup,
        }
    }

    /// The 16 bytes of its ID, as stored next to its kind.
    #[must_use]
    pub const fn id_bytes(self) -> [u8; 16] {
        match self {
            Self::IssueType(id) => id.to_bytes(),
            Self::LabelGroup(id) => id.to_bytes(),
        }
    }

    /// The subject of this kind with these bytes, if they are an ID of the kind.
    #[must_use]
    pub fn from_parts(kind: OfferKind, bytes: [u8; 16]) -> Option<Self> {
        match kind {
            OfferKind::IssueType => IssueTypeId::from_bytes(bytes).ok().map(Self::IssueType),
            OfferKind::LabelGroup => LabelGroupId::from_bytes(bytes).ok().map(Self::LabelGroup),
        }
    }
}

impl crate::personal::NotPersonal for OfferSubject {}

/// Whether a subject is offered at a place: the row of one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Entity)]
pub struct Offer {
    /// The issue type or the group of labels, as its kind and its ID. No foreign key holds it (one
    /// column cannot refer to two tables), and nothing deletes types or groups yet: they are
    /// retired. Whoever adds deleting must delete their rows in the same change.
    #[not_personal]
    pub subject: OfferSubject,
    /// The project, with its sub-projects; none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// Whether the subject is offered there.
    #[not_personal]
    pub offered: bool,
}

/// Whether a subject created in `owner` (none: for the whole organization) reaches a place whose
/// projects from the top down are `place_chain` (empty for the whole organization): one created in a
/// project reaches that project and everything below it, and nothing else. The one definition of
/// this rule: the offers, the rows a person may set and the screens of a project all ask it.
#[must_use]
pub fn reaches(owner: Option<ProjectId>, place_chain: &[ProjectId]) -> bool {
    owner.is_none_or(|owner| place_chain.contains(&owner))
}

/// Whether `subject` is offered in a place whose projects from the top down are `chain`. `owner` is
/// the project it was created in (none for the whole organization). A row below the owner may take
/// it away again; a row above the owner, or for the whole organization, is no part of the answer.
#[must_use]
pub fn is_offered(
    subject: OfferSubject,
    owner: Option<ProjectId>,
    offers: &[Offer],
    chain: &[ProjectId],
) -> bool {
    if !reaches(owner, chain) {
        return false;
    }
    let rows = offers
        .iter()
        .filter(|offer| offer.subject == subject)
        .filter_map(|offer| {
            // The projects from the top down to the place of the row; a row outside the chain is no
            // part of the answer.
            let row_chain = match offer.project {
                None => &chain[..0],
                Some(project) => &chain[..chain_depth(chain, project)?],
            };
            reaches(owner, row_chain).then_some((offer.project, None, offer.offered))
        });
    nearest_row(chain, None, rows).is_none_or(|(_, _, offered)| offered)
}

/// Why an issue type takes no new issue in a project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRefusal {
    /// The type is retired.
    Retired,
    /// The project does not offer the type.
    NotOffered,
}

/// Whether an issue type takes a new issue (or an issue changed to it) in a project whose projects
/// from the top down are `chain`: it is not retired, and the project offers it. The one definition,
/// before a change and inside it. An issue keeps a type that fails this.
///
/// # Errors
/// The reason the type takes no issue there.
pub fn type_takes_issues(
    issue_type: &IssueType,
    offers: &[Offer],
    chain: &[ProjectId],
) -> Result<(), TypeRefusal> {
    if issue_type.retired_at.is_some() {
        return Err(TypeRefusal::Retired);
    }
    if is_offered(
        OfferSubject::IssueType(issue_type.id),
        issue_type.project,
        offers,
        chain,
    ) {
        Ok(())
    } else {
        Err(TypeRefusal::NotOffered)
    }
}

/// Whether two subjects made in these places can meet in one project, so their names must differ
/// (ISS-02, TAG-02): one made for the whole organization meets every other subject; two made in
/// projects meet when they are the same project or one is inside the other. `is_within(project,
/// outside)` says whether a project is inside another, at any depth.
#[must_use]
pub fn can_meet(
    one: Option<ProjectId>,
    other: Option<ProjectId>,
    is_within: impl Fn(ProjectId, ProjectId) -> bool,
) -> bool {
    match (one, other) {
        (Some(one), Some(other)) => one == other || is_within(one, other) || is_within(other, one),
        _ => true,
    }
}

/// Whether one more subject fits in `owner`: the subjects made there (none: for the whole
/// organization) are counted on their own, so a project does not use up the room of another one.
#[must_use]
pub fn has_room(
    made_in: impl IntoIterator<Item = Option<ProjectId>>,
    owner: Option<ProjectId>,
    most: usize,
) -> bool {
    made_in.into_iter().filter(|each| *each == owner).count() < most
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    struct World {
        parent: ProjectId,
        child: ProjectId,
        other: ProjectId,
        bug: OfferSubject,
        task: OfferSubject,
    }

    fn world() -> World {
        let mut source = SequentialIds::default();
        World {
            parent: ProjectId::generate(&mut source).unwrap(),
            child: ProjectId::generate(&mut source).unwrap(),
            other: ProjectId::generate(&mut source).unwrap(),
            bug: OfferSubject::IssueType(IssueTypeId::generate(&mut source).unwrap()),
            task: OfferSubject::IssueType(IssueTypeId::generate(&mut source).unwrap()),
        }
    }

    #[test]
    fn iss_02_with_no_row_an_organization_wide_subject_is_offered_everywhere() {
        let w = world();
        assert!(is_offered(w.bug, None, &[], &[w.parent, w.child]));
        assert!(is_offered(w.bug, None, &[], &[]));
    }

    #[test]
    fn iss_02_a_row_counts_for_sub_projects_until_one_has_its_own() {
        let w = world();
        let rows = [
            Offer {
                subject: w.bug,
                project: None,
                offered: false,
            },
            Offer {
                subject: w.bug,
                project: Some(w.parent),
                offered: true,
            },
        ];
        assert!(!is_offered(w.bug, None, &rows, &[w.other]));
        assert!(is_offered(w.bug, None, &rows, &[w.parent]));
        assert!(is_offered(w.bug, None, &rows, &[w.parent, w.child]));
        let with_own = [
            rows[0],
            rows[1],
            Offer {
                subject: w.bug,
                project: Some(w.child),
                offered: false,
            },
        ];
        assert!(!is_offered(w.bug, None, &with_own, &[w.parent, w.child]));
        assert!(is_offered(w.bug, None, &with_own, &[w.parent]));
    }

    #[test]
    fn tag_02_a_subject_created_in_a_project_is_offered_there_and_below_only() {
        let w = world();
        assert!(is_offered(w.bug, Some(w.parent), &[], &[w.parent]));
        assert!(is_offered(w.bug, Some(w.parent), &[], &[w.parent, w.child]));
        assert!(!is_offered(w.bug, Some(w.parent), &[], &[w.other]));
        let off = [Offer {
            subject: w.bug,
            project: Some(w.child),
            offered: false,
        }];
        assert!(!is_offered(
            w.bug,
            Some(w.parent),
            &off,
            &[w.parent, w.child]
        ));
        assert!(is_offered(w.bug, Some(w.parent), &off, &[w.parent]));
    }

    #[test]
    fn iss_02_a_row_above_the_owner_or_for_the_organization_is_no_part_of_the_answer() {
        let w = world();
        let rows = [
            Offer {
                subject: w.bug,
                project: None,
                offered: true,
            },
            Offer {
                subject: w.bug,
                project: Some(w.parent),
                offered: true,
            },
        ];
        // Made in the child: the rows on the organization and the parent do not offer it above.
        assert!(!is_offered(w.bug, Some(w.child), &rows, &[w.parent]));
        assert!(!is_offered(w.bug, Some(w.child), &rows, &[w.other]));
        assert!(is_offered(
            w.bug,
            Some(w.child),
            &rows,
            &[w.parent, w.child]
        ));
    }

    #[test]
    fn a_row_of_another_subject_is_no_part_of_the_answer() {
        let w = world();
        let rows = [Offer {
            subject: w.task,
            project: None,
            offered: false,
        }];
        assert!(is_offered(w.bug, None, &rows, &[w.parent]));
        assert!(!is_offered(w.task, None, &rows, &[w.parent]));
    }

    #[test]
    fn iss_02_subjects_meet_when_one_is_for_the_organization_or_in_the_same_chain() {
        let w = world();
        let within = |project, outside| project == w.child && outside == w.parent;
        assert!(can_meet(None, Some(w.other), within));
        assert!(can_meet(Some(w.other), None, within));
        assert!(can_meet(Some(w.parent), Some(w.parent), within));
        assert!(can_meet(Some(w.child), Some(w.parent), within));
        assert!(can_meet(Some(w.parent), Some(w.child), within));
        assert!(!can_meet(Some(w.parent), Some(w.other), within));
        assert!(!can_meet(Some(w.child), Some(w.other), within));
    }

    #[test]
    fn iss_02_the_room_is_counted_for_each_owner_on_its_own() {
        let w = world();
        let made = [None, Some(w.parent), Some(w.parent), Some(w.other)];
        assert!(has_room(made, Some(w.parent), 3));
        assert!(!has_room(made, Some(w.parent), 2));
        assert!(has_room(made, Some(w.child), 1));
        assert!(!has_room(made, None, 1));
        assert!(has_room(made, None, 2));
    }

    #[test]
    fn iss_02_a_type_takes_issues_when_it_is_not_retired_and_the_project_offers_it() {
        let w = world();
        let OfferSubject::IssueType(id) = w.bug else {
            return;
        };
        let mut found = IssueType {
            id,
            name: "Bug".to_owned(),
            icon: crate::issue::IssueIcon::CircleHelp,
            project: None,
            position: 0,
            retired_at: None,
            version: 1,
        };
        let off = [Offer {
            subject: w.bug,
            project: Some(w.parent),
            offered: false,
        }];
        assert_eq!(type_takes_issues(&found, &off, &[w.other]), Ok(()));
        assert_eq!(
            type_takes_issues(&found, &off, &[w.parent, w.child]),
            Err(TypeRefusal::NotOffered)
        );
        found.retired_at = Some(jiff::Timestamp::UNIX_EPOCH);
        assert_eq!(
            type_takes_issues(&found, &[], &[w.other]),
            Err(TypeRefusal::Retired)
        );
    }

    #[test]
    fn iss_02_a_subject_reaches_its_project_and_below_only() {
        let w = world();
        assert!(reaches(None, &[]));
        assert!(reaches(None, &[w.parent]));
        assert!(reaches(Some(w.parent), &[w.parent, w.child]));
        assert!(!reaches(Some(w.parent), &[]));
        assert!(!reaches(Some(w.child), &[w.parent]));
    }
}
