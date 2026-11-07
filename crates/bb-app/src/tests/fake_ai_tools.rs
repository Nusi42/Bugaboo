//! The AI part of the in-memory store: the marks *Never sent to AI* and the log of tool calls.

use bb_core::ai::{NeverSentMark, ToolCall};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::ai::NeverSentStore;
use crate::ai::tools::ToolCallStore;

impl NeverSentStore for FakeStore {
    async fn never_sent_marks_in(tx: &mut Tx) -> Result<Vec<NeverSentMark>, FakeError> {
        Ok(tx.state.never_sent.clone())
    }

    async fn put_never_sent_mark(tx: &mut Tx, mark: &NeverSentMark) -> Result<bool, FakeError> {
        let stored = tx
            .state
            .never_sent
            .iter()
            .position(|row| row.subject == mark.subject);
        let current = stored.map_or(0, |at| tx.state.never_sent[at].version);
        if current + 1 != mark.version {
            return Ok(false);
        }
        match stored {
            Some(at) => tx.state.never_sent[at] = *mark,
            None => tx.state.never_sent.push(*mark),
        }
        Ok(true)
    }
}

impl ToolCallStore for FakeStore {
    async fn append_tool_call(&self, call: &ToolCall) -> Result<(), FakeError> {
        self.lock().tool_calls.push(call.clone());
        Ok(())
    }

    async fn prune_tool_calls(&self, before: Timestamp) -> Result<u64, FakeError> {
        let mut shared = self.lock();
        let kept = shared.tool_calls.len();
        shared.tool_calls.retain(|call| call.at >= before);
        Ok(u64::try_from(kept - shared.tool_calls.len()).unwrap_or(u64::MAX))
    }

    async fn tool_calls(&self, limit: u64) -> Result<Vec<ToolCall>, FakeError> {
        let shared = self.lock();
        Ok(shared
            .tool_calls
            .iter()
            .rev()
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .cloned()
            .collect())
    }
}
