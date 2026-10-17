//! The port of the workflow use cases: what they need from storage beyond the own-field port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use bb_core::id::{IssueTypeId, ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::workflow::{StoredWorkflow, Workflow, WorkflowContent, WorkflowPlace, WorkflowUse};

use crate::fields::FieldStore;
use crate::store_macros::{queries, writes};

/// The places workflows are used at and the workflows they name, with everything the judge needs:
/// the workflows that no place uses are not in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectoryRows {
    /// Where each workflow is used.
    pub uses: Vec<WorkflowUse>,
    /// The workflows that are used somewhere, with their statuses and status changes.
    pub workflows: Vec<StoredWorkflow>,
}

/// What the workflow use cases need from storage.
pub trait WorkflowStore: FieldStore + 'static {
    queries! {
        /// Where each workflow is used. The lookup of ADR-0031 (`bb_core::workflow::find`) picks
        /// the one that counts.
        fn workflow_uses() -> Vec<WorkflowUse> = workflow_uses_in;
        /// The places workflows are used at and the workflows they name: two reads, and one when
        /// no place uses a stored workflow (the statuses are read together with the uses).
        fn workflow_directory() -> DirectoryRows = workflow_directory_in;
        /// The header of every workflow, by name.
        fn workflow_headers() -> Vec<Workflow> = workflow_headers_in;
        /// One workflow with its statuses and status changes.
        fn stored_workflow(id: WorkflowId) -> Option<StoredWorkflow> = stored_workflow_in;
        /// These workflows with their statuses and status changes, in as few reads as one.
        fn stored_workflows(ids: Vec<WorkflowId>) -> Vec<StoredWorkflow> = stored_workflows_in;
        /// Whether any issue was ever written, also one in the trash: what tells an organization
        /// that ran before workflows from a new one.
        fn has_issues() -> bool = has_issues_in;
        /// The issues that are not in the trash, in one of these projects (`None`: any) and of
        /// one of these issue types (`None`: any). What a change to the workflows of some places
        /// looks at, so it reads the issues those places reach and no others.
        fn live_issues(
            projects: Option<Vec<ProjectId>>,
            issue_types: Option<Vec<IssueTypeId>>
        ) -> Vec<Issue> = live_issues_in;
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
