//! Workflows (WF-01…WF-04, WF-06, ADR-0031): the statuses of an issue type and the status changes
//! between them, as data.
//!
//! Every decision about a status change is `bb_core::workflow::judge::judge`: the use cases here
//! give it the facts (who the person is for the issue, what they may do with each field, which
//! fields have a value) and turn its answer into an error or a list of buttons. There is no second
//! place that decides a status change (ADR-0031 decision 3).

mod facts;
mod read;
mod resolve;
mod status;
mod store;
mod types;

pub use read::{ProjectStatuses, ProjectStatusesQuery, project_statuses};
pub use store::WorkflowStore;
pub use types::{StatusChangeView, StatusView};

pub(crate) use resolve::WorkflowDirectory;
pub(crate) use status::StatusCheck;
