//! A workflow is a record the edit locks work on (ADR-0019, ADR-0031 decision 4): where it sits,
//! its version, and what a person needs to change it. The issues it governs are found here too,
//! for *Publish* and for taking a workflow away.

use std::collections::HashMap;

use bb_core::event::EntityRef;
use bb_core::id::{ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::workflow::StatusKey;
use bb_policy::{Permission, Place};

use super::resolve::WorkflowDirectory;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::RecordFacts;

impl<S: AuthStore> Ctx<'_, S> {
    /// The place that holds a workflow, for the permissions: the lowest project that all its uses
    /// sit in or below, or the whole organization when it is used there, in no place, or in places
    /// that share no project. A person who holds *Manage workflows* there holds it at every place
    /// the workflow is used.
    pub(crate) async fn place_of_workflow(&self, id: WorkflowId) -> Result<Place, AppError> {
        let uses = self
            .store()
            .workflow_uses()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut common: Option<Vec<ProjectId>> = None;
        for used in uses.iter().filter(|used| used.workflow == id) {
            let Some(project) = used.project else {
                return Ok(Place::Organization);
            };
            let chain = self.chain_of(project).await?;
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
        Ok(common
            .and_then(|chain| chain.last().copied())
            .map_or(Place::Organization, Place::Project))
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

    /// The issues that are not in the trash, are in one of these statuses, and are governed by this
    /// workflow, each with the projects from the top down to its project.
    pub(crate) async fn governed_issues(
        &self,
        directory: &WorkflowDirectory,
        workflow: WorkflowId,
        keys: Vec<StatusKey>,
    ) -> Result<Vec<(Issue, Vec<ProjectId>)>, AppError> {
        let found = self
            .store()
            .issues_in_statuses(keys)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut chains: HashMap<ProjectId, Vec<ProjectId>> = HashMap::new();
        let mut governed = Vec::new();
        for issue in found {
            let chain = if let Some(known) = chains.get(&issue.project) {
                known.clone()
            } else {
                let chain = self.chain_of(issue.project).await?;
                chains.insert(issue.project, chain.clone());
                chain
            };
            if directory.id_in_force(&chain, Some(issue.issue_type)) == Some(workflow) {
                governed.push((issue, chain));
            }
        }
        Ok(governed)
    }
}
