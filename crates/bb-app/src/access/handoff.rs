//! What the entry of the MCP server leaves for the call of the tool of the same message: the actor of
//! an agent and the rights of its person, so the key, its owner and their rights are looked up once
//! for a message and not once for each step (M7-F127).
//!
//! It is left under a handle that only that message gets (`McpIdentity::handoff`, carried in the
//! `RequestInfo` of its calls), and only the call of a tool and the list of tools take it, with the
//! handle and the text of the same key, once, within a short time and never for a request that is about
//! to change something. Every other request looks the key up, whatever was left.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bb_auth::SecretToken;
use bb_policy::Policy;

use super::Actor;
use crate::auth::{AppError, AuthStore, Ctx, locked};

/// How long what the entry left waits for the call of its message.
const HANDOFF_WAIT: Duration = Duration::from_secs(2);
/// How many messages may wait for their call at once; beyond it nothing is left (the call looks the
/// key up itself).
const HANDOFF_MOST: usize = 1024;

/// What is left for one message: the text of the key it came with, the actor, the rights of the
/// person of the actor if they were read, and when it was left.
pub(crate) type Handoff = (String, Actor, Option<Arc<Policy>>, Instant);

impl<S: AuthStore> Ctx<'_, S> {
    /// Leaves the actor of an agent for the call of the tool of this message and gives the handle that
    /// only that call may take it with; `None` when nothing is left.
    pub(crate) fn hand_off(&self, token_text: &str, actor: &Actor) -> Option<String> {
        let handle = SecretToken::generate().ok()?.encode();
        let now = Instant::now();
        let mut left = locked(&self.services.agent_handoff);
        left.retain(|_, (_, _, _, at)| now.duration_since(*at) < HANDOFF_WAIT);
        if left.len() >= HANDOFF_MOST {
            return None;
        }
        let rights = locked(&self.policies).get(&actor.person.id).cloned();
        left.insert(
            handle.clone(),
            (token_text.to_owned(), actor.clone(), rights, now),
        );
        Some(handle)
    }

    /// Who acts in the call of a tool or the list of tools: what the entry of this message left when
    /// the request has its handle, else the one who asks by the key or the session as any request.
    pub(crate) async fn actor_of_message(&self, token_text: &str) -> Result<Actor, AppError> {
        if let Some(handle) = &self.request.handoff
            && !self.change_due.load(Ordering::Relaxed)
            && locked(&self.agent_actor).is_none()
            && let Some(actor) = self.take_handoff(handle, token_text)
        {
            return Ok(actor);
        }
        self.actor(token_text).await
    }

    /// The actor left under `handle` for the key `token_text`, once, while it waits.
    fn take_handoff(&self, handle: &str, token_text: &str) -> Option<Actor> {
        let (text, actor, rights, at) = locked(&self.services.agent_handoff).remove(handle)?;
        if text != token_text || at.elapsed() >= HANDOFF_WAIT {
            return None;
        }
        let agent = actor.agent()?;
        if let Some(rights) = rights {
            locked(&self.policies)
                .entry(actor.person.id)
                .or_insert(rights);
        }
        self.via_key.store(true, Ordering::Relaxed);
        *locked(&self.acting_agent) = Some((agent, actor.person.id));
        *locked(&self.agent_actor) = Some((token_text.to_owned(), actor.clone()));
        Some(actor)
    }
}
