//! What the plans of an agent's calls share (ADR-0029, decision 9): whether a plan is made to be written
//! or only read for a preview, and the lines a preview lists.

use bb_core::agent_work::AgentWorkState;
use bb_core::event::FieldChange;
use serde_json::json;

use super::members::member;
use crate::auth::set_to;

/// Whether a plan is made to be written (`Writing`) or only to say what would be written
/// (`Preview`): a preview that finds a limit reached says so and pauses nobody.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Plan {
    /// The use case goes on to write what the plan holds.
    Writing,
    /// The plan is read for a preview and saves nothing.
    Preview,
}

/// The change of the state of the work, as a preview lists it.
pub(super) fn state_change(before: AgentWorkState, after: AgentWorkState) -> FieldChange {
    FieldChange {
        field: member::WORK_STATE.to_owned(),
        before: Some(json!(before.code())),
        after: Some(json!(after.code())),
    }
}

/// A member that is set to a value.
pub(super) fn set(field: &str, value: serde_json::Value) -> FieldChange {
    set_to(field, value)
}
