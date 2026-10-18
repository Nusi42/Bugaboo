//! A workflow is a record the edit locks work on (ADR-0019, ADR-0031 decision 4): where it sits,
//! its version, and what a person needs to change it. The issues it governs are found here too,
//! for *Publish* and for taking a workflow away.

use bb_core::event::EntityRef;
use bb_core::id::{ProjectId, WorkflowId};
use bb_core::workflow::WorkflowUse;
use bb_policy::{Permission, Place, Tree};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::RecordFacts;

impl<S: AuthStore> Ctx<'_, S> {
    /// The place that holds a workflow, for the permissions: the lowest project that all its uses
    /// sit in or below, or the whole organization when it is used there, in no place, or in places
    /// that share no project. A person who holds *Manage workflows* there holds it at every place
    /// the workflow is used.
    pub(crate) async fn place_of_workflow(&self, id: WorkflowId) -> Result<Place, AppError> {
        let failed = |error| internal::<S>(&error);
        let tree = self.project_tree_now().await?;
        let uses = self.store().workflow_uses().await.map_err(failed)?;
        Ok(place_from_uses(&tree, &uses, id))
    }

    /// Whether a person may put a workflow to use at a place: the workflow sits at that place or
    /// at one above it, so a workflow's place never moves down into a project it was not created
    /// for. One that is used nowhere is the organization's to hand out.
    pub(crate) async fn may_adopt_workflow(
        &self,
        actor: &Actor,
        id: WorkflowId,
        chain: &[ProjectId],
    ) -> Result<bool, AppError> {
        let failed = |error| internal::<S>(&error);
        let tree = self.project_tree_now().await?;
        let uses = self.store().workflow_uses().await.map_err(failed)?;
        let unused = !uses.iter().any(|used| used.workflow == id);
        if unused
            && !self
                .evaluation_of(actor, Place::Organization)
                .await?
                .allows(Permission::ManageWorkflows)
        {
            return Ok(false);
        }
        Ok(match place_from_uses(&tree, &uses, id) {
            Place::Project(project) => chain.contains(&project),
            // Used at the organization or nowhere: anybody may adopt it. Used by projects that
            // share no project, it is for those who manage workflows for the whole organization.
            Place::Organization if unused || is_used_at_organization(&uses, id) => true,
            Place::Organization => self
                .evaluation_of(actor, Place::Organization)
                .await?
                .allows(Permission::ManageWorkflows),
        })
    }

    /// The facts of a workflow for the editing use cases; `None` when there is no such workflow or
    /// the person may not manage workflows where it sits: they do not know it.
    pub(crate) async fn workflow_facts(
        &self,
        actor: &Actor,
        record: &EntityRef,
    ) -> Result<Option<RecordFacts>, AppError> {
        let Ok(id) = WorkflowId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        let Some(stored) = self
            .store()
            .stored_workflow(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        let place = self.place_of_workflow(id).await?;
        let held = self.evaluation_of(actor, place).await?;
        if !held.allows(Permission::ManageWorkflows) {
            return Ok(None);
        }
        Ok(Some(RecordFacts {
            place,
            version: stored.workflow.version,
            edit: Permission::ManageWorkflows,
        }))
    }
}

/// Whether the workflow is used by the whole organization.
fn is_used_at_organization(uses: &[WorkflowUse], id: WorkflowId) -> bool {
    uses.iter()
        .any(|used| used.workflow == id && used.project.is_none())
}

/// The place that holds a workflow, from the tree of the projects and the places workflows are
/// used at.
pub(crate) fn place_from_uses(tree: &Tree, uses: &[WorkflowUse], id: WorkflowId) -> Place {
    let mut common: Option<Vec<ProjectId>> = None;
    for used in uses.iter().filter(|used| used.workflow == id) {
        let Some(project) = used.project else {
            return Place::Organization;
        };
        let chain = tree.chain(project).unwrap_or_default();
        common = Some(match common {
            None => chain,
            Some(so_far) => so_far
                .iter()
                .zip(&chain)
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect(),
        });
    }
    common
        .and_then(|chain| chain.last().copied())
        .map_or(Place::Organization, Place::Project)
}
