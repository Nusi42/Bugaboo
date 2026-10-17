//! No issue is left in a status its workflow does not have (ADR-0031 amendment 5).
//!
//! One check, "every live issue that is governed after the change has a status of its workflow",
//! runs on the places and workflows as they will be after the change, inside the transaction that
//! makes the change: *Publish*, a place that uses another workflow (or follows its parent again),
//! and taking a workflow away all call it. Issues in the trash keep their status; when one comes
//! back, the judge lets it start again in the starting status.

use std::collections::HashMap;

use bb_core::id::{IssueTypeId, ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::project::Project;
use bb_core::workflow::{StatusKey, WorkflowPlace, WorkflowUse};

use super::resolve::WorkflowDirectory;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The issues a change to the workflows of some places can reach: the projects (with their
/// sub-projects) and issue types of those places. A part that is `None` reaches everything, so the
/// reach is never smaller than the truth.
pub(crate) struct Reach {
    projects: Option<Vec<ProjectId>>,
    issue_types: Option<Vec<IssueTypeId>>,
}

/// The projects from the top down to `project`, read from the tree of all projects.
pub(crate) fn chain_in(tree: &[Project], project: ProjectId) -> Vec<ProjectId> {
    let parents: HashMap<ProjectId, Option<ProjectId>> =
        tree.iter().map(|each| (each.id, each.parent)).collect();
    let mut chain = vec![project];
    let mut at = project;
    while let Some(Some(parent)) = parents.get(&at) {
        if chain.contains(parent) {
            break;
        }
        chain.push(*parent);
        at = *parent;
    }
    chain.reverse();
    chain
}

impl Reach {
    /// What these places reach, given the tree of the projects.
    pub(crate) fn of(places: &[WorkflowPlace], tree: &[Project]) -> Self {
        let projects = if places.iter().any(|place| place.project.is_none()) {
            None
        } else {
            let mut reached: Vec<ProjectId> = Vec::new();
            for each in tree {
                let chain = chain_in(tree, each.id);
                if places.iter().any(|place| {
                    place
                        .project
                        .is_some_and(|project| chain.contains(&project))
                }) {
                    reached.push(each.id);
                }
            }
            Some(reached)
        };
        let issue_types = if places.iter().any(|place| place.issue_type.is_none()) {
            None
        } else {
            Some(places.iter().filter_map(|place| place.issue_type).collect())
        };
        Self {
            projects,
            issue_types,
        }
    }
}

/// The places a workflow is used at.
pub(crate) fn places_of(uses: &[WorkflowUse], workflow: WorkflowId) -> Vec<WorkflowPlace> {
    uses.iter()
        .filter(|used| used.workflow == workflow)
        .map(WorkflowUse::place)
        .collect()
}

/// How many live issues in the reach are in a status that the workflow governing them does not
/// have, on the uses and workflows stored in this transaction (so, after the change it made).
///
/// # Errors
/// The error of the store.
pub(crate) async fn count_stranded_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    reach: &Reach,
) -> Result<usize, S::Error> {
    let tree = S::project_tree_in(transaction).await?;
    let rows = S::workflow_directory_in(transaction).await?;
    let directory = WorkflowDirectory::build(rows.uses, rows.workflows);
    let issues = S::live_issues_in(
        transaction,
        reach.projects.clone(),
        reach.issue_types.clone(),
    )
    .await?;
    Ok(issues
        .iter()
        .filter(|issue| {
            let chain = chain_in(&tree, issue.project);
            directory
                .in_force(&chain, Some(issue.issue_type))
                .status(&issue.status)
                .is_none()
        })
        .count())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The live issues that this workflow governs and that are in one of these statuses, found
    /// from the places the workflow is used at and no others.
    pub(crate) async fn governed_issues(
        &self,
        directory: &WorkflowDirectory,
        workflow: WorkflowId,
        keys: &[StatusKey],
    ) -> Result<Vec<Issue>, AppError> {
        let failed = |error| internal::<S>(&error);
        let tree = self.store().project_tree().await.map_err(failed)?;
        let reach = Reach::of(&places_of(directory.uses(), workflow), &tree);
        let found = self
            .store()
            .live_issues(reach.projects, reach.issue_types)
            .await
            .map_err(failed)?;
        Ok(found
            .into_iter()
            .filter(|issue| keys.contains(&issue.status))
            .filter(|issue| {
                let chain = chain_in(&tree, issue.project);
                directory.id_in_force(&chain, Some(issue.issue_type)) == Some(workflow)
            })
            .collect())
    }
}
