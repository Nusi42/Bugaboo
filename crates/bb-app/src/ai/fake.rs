//! The provider of the tests: it records every request it gets and answers from a script, so a
//! test can say what the model answers and see what the server sent (and that it sent nothing
//! while AI is off).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bb_core::ai::ProviderKind;

use super::{AiAnswer, AiFailure, AiProvider, AiRequest, AiTarget};
use crate::ports::BoxFuture;

/// What the fake kept of a call.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedCall {
    /// The kind of provider the call was made for.
    pub kind: ProviderKind,
    /// The address of an OpenAI-compatible model, as the server built it.
    pub address: Option<String>,
    /// The model.
    pub model: String,
    /// Whether the call carried a key.
    pub had_key: bool,
    /// The request.
    pub request: AiRequest,
}

#[derive(Default)]
struct State {
    calls: Vec<RecordedCall>,
    script: VecDeque<Result<AiAnswer, AiFailure>>,
}

/// A provider that answers from a script. Clones share what was sent and what is to be answered,
/// so a test keeps one and hands the other to the services.
#[derive(Clone, Default)]
pub struct FakeAiProvider {
    state: Arc<Mutex<State>>,
}

fn locked(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeAiProvider {
    /// A fake that answers `OK` until it is told otherwise.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The next call is answered with this.
    pub fn will_answer(&self, answer: AiAnswer) {
        locked(&self.state).script.push_back(Ok(answer));
    }

    /// The next call fails: the provider did not answer.
    pub fn will_fail(&self) {
        locked(&self.state)
            .script
            .push_back(Err(AiFailure::Unavailable));
    }

    /// Every call so far, oldest first.
    #[must_use]
    pub fn calls(&self) -> Vec<RecordedCall> {
        locked(&self.state).calls.clone()
    }
}

impl AiProvider for FakeAiProvider {
    fn complete<'a>(
        &'a self,
        target: &'a AiTarget,
        request: &'a AiRequest,
    ) -> BoxFuture<'a, Result<AiAnswer, AiFailure>> {
        let mut state = locked(&self.state);
        state.calls.push(RecordedCall {
            kind: target.kind,
            address: target
                .endpoint
                .as_ref()
                .map(|endpoint| endpoint.base.clone()),
            model: target.model.clone(),
            had_key: !target.key.is_empty(),
            request: request.clone(),
        });
        let answer = state.script.pop_front().unwrap_or_else(|| {
            Ok(AiAnswer {
                text: "OK".to_owned(),
                ..AiAnswer::default()
            })
        });
        Box::pin(async move { answer })
    }
}
