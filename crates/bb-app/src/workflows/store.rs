//! The port of the workflow use cases: what they need from storage beyond the own-field port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use crate::fields::FieldStore;
use crate::store_macros::{queries, writes};
use bb_core::id::{IssueId, IssueTypeId, ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::workflow::{
    StatusKey, StoredWorkflow, Workflow, WorkflowContent, WorkflowPlace, WorkflowUse,
};

/// The places workflows are used at and the workflows they name, with everything the judge needs:
/// the workflows that no place uses are not in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectoryRows {
    /// Where each workflow is used.
    pub uses: Vec<WorkflowUse>,
    /// The workflows that are used somewhere, with their statuses and status changes.
    pub workflows: Vec<StoredWorkflow>,
}

/// Which statuses the issues a read of live issues returns are in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusFilter {
    /// Any status.
    Any,
    /// One of these statuses.
    In(Vec<StatusKey>),
    /// None of these statuses.
    NotIn(Vec<StatusKey>),
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
        /// one of these issue types (`None`: any), in the statuses the filter names. What a change to the workflows of some places
        /// looks at, so it reads the issues those places reach and no others.
        fn live_issues(
            projects: Option<Vec<ProjectId>>,
            issue_types: Option<Vec<IssueTypeId>>,
            statuses: StatusFilter
        ) -> Vec<Issue> = live_issues_in;
        /// The same, by ID, at most `limit` issues that come after the issue `after` (the first
        /// ones when it is `None`): what a count that stops at a limit reads, a page at a time.
        fn live_issues_after(
            projects: Option<Vec<ProjectId>>,
            issue_types: Option<Vec<IssueTypeId>>,
            statuses: StatusFilter,
            after: Option<IssueId>,
            limit: usize
        ) -> Vec<Issue> = live_issues_after_in;
    }

    writes! {
        /// Takes the lock that a change to the workflows that count takes alone: it lasts to the end
        /// of the change.
        fn lock_workflows() -> ();
        /// Takes the same lock together with the others who write an issue with a status, so that
        /// no workflow is published between the look at the workflow and the write.
        fn lock_workflows_shared() -> ();
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
