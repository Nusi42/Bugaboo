//! The adapters of the AI provider (AI-01, ADR-0013): Anthropic through the Messages API, and any
//! service with the chat shape of `OpenAI`, which covers a model on the organization's own network.
//! They implement the port `AiProvider` of `bb-app`; `bb-server` wires them. The fake for tests is
//! behind the feature `testing`.
//!
//! The parts that write a request and read an answer are pure and tested against recorded examples;
//! the one outgoing call is in `http` (ARCH-HTTP-EDGE). A failure is one class: what the provider
//! sent back is never passed on.

#![forbid(unsafe_code)]

mod anthropic;
mod http;
mod openai;

use bb_app::ai::{AiAnswer, AiFailure, AiProvider, AiRequest, AiTarget};
use bb_app::ports::BoxFuture;
use bb_core::ai::ProviderKind;
use zeroize::Zeroizing;

// One definition of the fake, with the port it stands in for; the adapter crate offers it to the
// tests of the program.
#[cfg(feature = "testing")]
pub use bb_app::ai::{FakeAiProvider, RecordedCall};

/// Calls the provider that the target names: one adapter for both kinds, chosen for every call, so
/// a change of the settings counts from the next one.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpAiProvider;

impl HttpAiProvider {
    fn post_of(target: &AiTarget, request: &AiRequest) -> Option<http::Post> {
        match target.kind {
            ProviderKind::Anthropic => Some(http::Post {
                url: anthropic::ADDRESS.to_owned(),
                headers: vec![
                    ("x-api-key", Zeroizing::new(target.key.to_string())),
                    (
                        "anthropic-version",
                        Zeroizing::new(anthropic::API_VERSION.to_owned()),
                    ),
                ],
                body: anthropic::request_body(&target.model, request).to_string(),
                encrypted: true,
            }),
            ProviderKind::OpenAiCompatible => {
                let endpoint = target.endpoint.as_ref()?;
                let mut headers = Vec::new();
                if !target.key.is_empty() {
                    headers.push((
                        "authorization",
                        Zeroizing::new(format!("Bearer {}", target.key.as_str())),
                    ));
                }
                Some(http::Post {
                    url: format!("{}{}", endpoint.base, openai::PATH),
                    headers,
                    body: openai::request_body(&target.model, request).to_string(),
                    encrypted: endpoint.encrypted,
                })
            }
        }
    }
}

impl AiProvider for HttpAiProvider {
    fn complete<'a>(
        &'a self,
        target: &'a AiTarget,
        request: &'a AiRequest,
    ) -> BoxFuture<'a, Result<AiAnswer, AiFailure>> {
        Box::pin(async move {
            let post = Self::post_of(target, request).ok_or(AiFailure::Unavailable)?;
            let body = http::post(post).await.map_err(|_| AiFailure::Unavailable)?;
            match target.kind {
                ProviderKind::Anthropic => anthropic::read_answer(&body),
                ProviderKind::OpenAiCompatible => openai::read_answer(&body),
            }
            .ok_or(AiFailure::Unavailable)
        })
    }
}
