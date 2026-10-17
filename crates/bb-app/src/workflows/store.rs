//! The port of the workflow use cases: what they need from storage beyond the own-field port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use bb_core::id::WorkflowId;
use bb_core::issue::Issue;
use bb_core::workflow::{
    StatusKey, StoredWorkflow, Workflow, WorkflowContent, WorkflowPlace, WorkflowUse,
};

use crate::fields::FieldStore;
use crate::store_macros::{queries, writes};

/// What the workflow use cases need from storage.
pub trait WorkflowStore: FieldStore + 'static {
    queries! {
        /// Where each workflow is used. The lookup of ADR-0031 (`bb_core::workflow::find`) picks
        /// the one that counts.
        fn workflow_uses() -> Vec<WorkflowUse> = workflow_uses_in;
        /// The header of every workflow, by name.
        fn workflow_headers() -> Vec<Workflow> = workflow_headers_in;
        /// One workflow with its statuses and status changes.
        fn stored_workflow(id: WorkflowId) -> Option<StoredWorkflow> = stored_workflow_in;
        /// These workflows with their statuses and status changes, in as few reads as one.
        fn stored_workflows(ids: Vec<WorkflowId>) -> Vec<StoredWorkflow> = stored_workflows_in;
        /// Whether any issue was ever written, also one in the trash: what tells an organization
        /// that ran before workflows from a new one.
        fn has_issues() -> bool = has_issues_in;
        /// The issues that are not in the trash and are in one of these statuses, in any project:
        /// what *Publish* looks at when a workflow drops a status.
        fn issues_in_statuses(keys: Vec<StatusKey>) -> Vec<Issue> = issues_in_statuses_in;
    }

    writes! {
        /// Adds a workflow with its statuses and status changes.
        fn insert_workflow(workflow: &Workflow, content: &WorkflowContent) -> ();
        /// Replaces the name, the statuses and the status changes of a workflow, counts its version
        /// up, and says whether the workflow was still at `version`.
        fn replace_workflow(id: WorkflowId, name: &str, content: &WorkflowContent, version: i64) -> bool;
        /// Takes a workflow away together with the places it was used at, and says whether it was
        /// still at `version`.
        fn delete_workflow(id: WorkflowId, version: i64) -> bool;
        /// Says that the place uses the workflow from now on (the place had a workflow or not).
        fn put_workflow_use(used: &WorkflowUse) -> ();
        /// Takes the use at exactly this place away, so the place follows its parent again.
        fn remove_workflow_use(place: WorkflowPlace) -> ();
    }
}
