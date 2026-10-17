//! Workflows (WF-01…WF-04, WF-06, ADR-0031): the statuses of an issue type and the status changes
//! between them, as data.
//!
//! Every decision about a status change is `bb_core::workflow::judge::judge`: the use cases here
//! give it the facts (who the person is for the issue, what they may do with each field, which
//! fields have a value) and turn its answer into an error or a list of buttons. There is no second
//! place that decides a status change (ADR-0031 decision 3).

mod editor;
mod facts;
mod preview;
mod publish;
mod read;
mod records;
mod resolve;
mod starting;
mod status;
mod store;
mod strand;
mod try_it;
mod types;
mod write;

pub use editor::{workflow_editor, workflows};
pub use preview::publish_preview;
pub use publish::publish_workflow;
pub use read::{ProjectChange, ProjectStatuses, ProjectStatusesQuery, project_statuses};
pub use starting::{CreateStartingWorkflows, create_starting_workflows};
pub use store::{DirectoryRows, StatusFilter, WorkflowStore};
pub use try_it::try_workflow;
pub use types::*;
pub use write::{create_workflow, delete_workflow, use_workflow};

pub(crate) use resolve::WorkflowDirectory;
pub(crate) use status::StatusCheck;
pub(crate) use strand::{
    ensure_status_in_force_in, ensure_written_status_in, restart_moved_in, restart_restored_in,
};
