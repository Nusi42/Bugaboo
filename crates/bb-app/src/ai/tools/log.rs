//! The log of tool calls (AI-05, DATA-LOGS, ADR-0013 amendment): every call of every tool is one
//! row, whoever made it. Who, how, which tool, whether it reads or changes, the size in and out, the
//! time and how it ended. Never the content: a row cannot hold a question, an answer or a name.

use std::future::Future;

use bb_core::ai::{TOOL_CALL_KEEP_DAYS, Tool, ToolCall, ToolCaller, ToolEffect, ToolOutcome};
use bb_core::id::{ToolCallId, UserId};
use jiff::Timestamp;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::FieldStore;
use crate::store_macros::reads;

/// What the log of tool calls needs from storage.
pub trait ToolCallStore: FieldStore + 'static {
    reads! {
        /// Adds a row to the log.
        fn append_tool_call(call: &ToolCall) -> ();
        /// Deletes the rows made before `before`. Returns how many went.
        fn prune_tool_calls(before: Timestamp) -> u64;
        /// The newest rows, newest first.
        fn tool_calls(limit: u64) -> Vec<ToolCall>;
    }
}

/// Who made a call and how it went, for one row of the log.
pub(crate) struct Called<'a> {
    pub(crate) person: UserId,
    pub(crate) caller: ToolCaller,
    pub(crate) app: Option<&'a str>,
    /// None for a call of a name that is no tool.
    pub(crate) tool: Option<Tool>,
    pub(crate) effect: Option<ToolEffect>,
    pub(crate) bytes_in: usize,
    pub(crate) bytes_out: usize,
    pub(crate) outcome: ToolOutcome,
}

fn size(bytes: usize) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Writes one row of the log. A log that cannot be written is reported in the server log and
    /// does not undo or refuse the call (as the security log, SEC-15).
    pub(crate) async fn log_tool_call(&self, called: Called<'_>) {
        let call = ToolCall {
            id: match ToolCallId::generate(&mut self.ids()) {
                Ok(id) => id,
                Err(error) => {
                    tracing::error!(cause = %error, "a tool call could not be given an ID");
                    return;
                }
            },
            person: called.person,
            caller: called.caller,
            app: called.app.map(str::to_owned),
            tool: called.tool,
            effect: called.effect,
            bytes_in: size(called.bytes_in),
            bytes_out: size(called.bytes_out),
            at: self.now(),
            outcome: called.outcome,
        };
        if let Err(error) = self.store().append_tool_call(&call).await {
            tracing::error!(
                tool = call.tool.map(Tool::code),
                cause = %S::describe(&error),
                "the log of tool calls could not be written"
            );
        }
    }

    /// Deletes the rows of the log older than its keep period ([`TOOL_CALL_KEEP_DAYS`]). Returns
    /// how many went.
    pub(crate) async fn prune_old_tool_calls(&self) -> Result<u64, AppError> {
        let before = self
            .now()
            .checked_sub(bb_core::time::days(TOOL_CALL_KEEP_DAYS))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.store()
            .prune_tool_calls(before)
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
