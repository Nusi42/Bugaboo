//! Where an issue type or a group of labels is offered (ISS-02, TAG-02, ADR-0027 amendment).
//!
//! Each place says it with a row: the whole organization or a project, one issue type or one group
//! of labels, and whether it is offered there. The nearest place with a row wins and a sub-project
//! follows its parent until it has a row of its own (`lookup::nearest_row`, the walk of layouts and
//! workflows). Where no row says anything, an organization-wide subject is offered everywhere and a
//! subject made in a project is offered in that project and below.

use crate::id::{IssueTypeId, LabelGroupId, ProjectId};
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
    /// The issue type or the group of labels.
    #[not_personal]
    pub subject: OfferSubject,
    /// The project, with its sub-projects; none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// Whether the subject is offered there.
    #[not_personal]
    pub offered: bool,
}

/// Whether `subject` is offered in a place whose projects from the top down are `chain`. `owner` is
/// the project it was made in (none for the whole organization). A row below the owner may take it
/// away again; a row above the owner, or for the whole organization, is no part of the answer.
#[must_use]
pub fn is_offered(
    subject: OfferSubject,
    owner: Option<ProjectId>,
    offers: &[Offer],
    chain: &[ProjectId],
) -> bool {
    // A subject made in a project reaches only that project and below.
    let owner_depth = match owner {
        None => 0,
        Some(owner) => match chain_depth(chain, owner) {
            Some(depth) => depth,
            None => return false,
        },
    };
    let rows = offers
        .iter()
        .filter(|offer| offer.subject == subject)
        .filter(|offer| {
            offer.project.map_or(0, |project| {
                chain_depth(chain, project).unwrap_or(usize::MAX)
            }) >= owner_depth
        })
        .map(|offer| (offer.project, None, offer.offered));
    nearest_row(chain, None, rows).is_none_or(|(_, _, offered)| offered)
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
}
