//! The one lookup order of rows that can be set for the organization, for a project and for one
//! issue type (ADR-0027, decision 2). Layouts and field levels per issue type are found with it.
//!
//! For a project and an issue type the nearest place with a row wins: the project, then its
//! parents, then the organization. At one place a row for the issue type wins over a row for all
//! types. A row for another issue type is no part of the answer.

use crate::id::{IssueTypeId, ProjectId};

/// The depth of a project of the chain (the projects from the top down to the place), from 1;
/// `None` for a project that is not in the chain. The organization is depth 0.
#[must_use]
pub fn chain_depth(chain: &[ProjectId], project: ProjectId) -> Option<usize> {
    chain
        .iter()
        .position(|step| *step == project)
        .map(|at| at + 1)
}

/// The row that counts for a place whose projects from the top down are `chain`, and for the
/// issue type `issue_type` (`None` when only rows for all types count). Each candidate is the
/// project it is set for (`None` for the organization), the issue type it is for (`None` for all
/// types) and the row itself. Returns the project the winner is set for, whether it is for the
/// issue type itself, and the row; `None` when no row applies.
#[must_use]
pub fn nearest_row<R>(
    chain: &[ProjectId],
    issue_type: Option<IssueTypeId>,
    candidates: impl IntoIterator<Item = (Option<ProjectId>, Option<IssueTypeId>, R)>,
) -> Option<(Option<ProjectId>, bool, R)> {
    let mut best: Option<((usize, bool), Option<ProjectId>, R)> = None;
    for (project, for_type, row) in candidates {
        let typed = match (for_type, issue_type) {
            (None, _) => false,
            (Some(wanted), Some(asked)) if wanted == asked => true,
            // A row for another issue type, or for a type nobody asked about.
            (Some(_), _) => continue,
        };
        let depth = match project {
            None => 0,
            Some(project) => match chain_depth(chain, project) {
                Some(depth) => depth,
                None => continue,
            },
        };
        let rank = (depth, typed);
        if best.as_ref().is_none_or(|(deepest, _, _)| rank > *deepest) {
            best = Some((rank, project, row));
        }
    }
    best.map(|((_, typed), project, row)| (project, typed, row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    fn ids() -> (ProjectId, ProjectId, IssueTypeId, IssueTypeId) {
        let mut source = SequentialIds::default();
        (
            ProjectId::generate(&mut source).unwrap(),
            ProjectId::generate(&mut source).unwrap(),
            IssueTypeId::generate(&mut source).unwrap(),
            IssueTypeId::generate(&mut source).unwrap(),
        )
    }

    #[test]
    fn adr_0027_the_nearest_place_wins_and_at_one_place_the_type_wins() {
        let (parent, child, bug, task) = ids();
        let chain = [parent, child];
        let rows = vec![
            (None, None, "organization"),
            (None, Some(bug), "organization, bugs"),
            (Some(parent), None, "parent"),
            (Some(child), None, "child"),
            (Some(parent), Some(bug), "parent, bugs"),
        ];
        let pick = |issue_type| nearest_row(&chain, issue_type, rows.clone()).map(|row| row.2);
        // The child's row for all types is nearer than the parent's row for bugs.
        assert_eq!(pick(Some(bug)), Some("child"));
        assert_eq!(pick(Some(task)), Some("child"));
        assert_eq!(pick(None), Some("child"));
        // Without the child's row, the parent's row for bugs wins over its row for all types.
        let without_child: Vec<_> = rows
            .iter()
            .filter(|row| row.2 != "child")
            .copied()
            .collect();
        let pick =
            |issue_type| nearest_row(&chain, issue_type, without_child.clone()).map(|row| row.2);
        assert_eq!(pick(Some(bug)), Some("parent, bugs"));
        assert_eq!(pick(Some(task)), Some("parent"));
        assert_eq!(pick(None), Some("parent"));
    }

    #[test]
    fn adr_0027_a_row_of_another_project_or_type_is_no_part_of_the_answer() {
        let (parent, child, bug, task) = ids();
        let rows = vec![(Some(child), None, 1), (None, Some(bug), 2)];
        assert_eq!(nearest_row(&[parent], Some(task), rows.clone()), None);
        let found = nearest_row(&[parent], Some(bug), rows).unwrap();
        assert_eq!((found.0, found.1, found.2), (None, true, 2));
    }
}
