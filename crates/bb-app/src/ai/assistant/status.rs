//! Whether the assistant is there for a person at a place (AI-02, COMP-08): the one answer the
//! button, the panel and the entry of quick search ask. It says where the answers come from and
//! whether names of people go with them, so the panel can always tell (UX §4.12). It reads the
//! settings as every other way in does ([`Ctx::ai_state_at`]) and sends nothing.

use bb_core::ai::{Destination, LookingAtKind, ToolCaller};
use bb_policy::{Authorized, Masked, Place};

use super::{LookingAt, session_only};
use crate::auth::{AppError, AuthStore, Ctx};

/// Ask whether the assistant is there.
pub struct AssistantStatusQuery {
    /// The secret of the session, or the access key, the question is asked as.
    pub session_token: String,
    /// The project the person looks at (its ID or short name), if there is one.
    pub project: Option<String>,
}

/// Whether the assistant is there, and what the panel says about where its answers come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantStatus {
    /// AI counts at the place: the organization and the project both have it on.
    pub available: bool,
    /// Where the data goes. None while the assistant is not there, or the address of the model is
    /// not a good one.
    pub destination: Option<Destination>,
    /// Whether people go to the provider by name; otherwise as their role.
    pub names_sent: bool,
}

impl Masked for AssistantStatus {}

impl AssistantStatus {
    /// The assistant is not there.
    const fn gone() -> Self {
        Self {
            available: false,
            destination: None,
            names_sent: false,
        }
    }
}

/// Whether the assistant is there for the person at a place.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key, `resource.not-found` for a project the person
/// may not see or that is kept from AI.
pub async fn assistant_status<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AssistantStatusQuery,
) -> Result<Authorized<AssistantStatus>, AppError> {
    session_only(&input.session_token)?;
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let mut state = ctx.ai_state_at(None).await?;
    if !state.counts() {
        return Ok(held.authorize(AssistantStatus::gone()));
    }
    if let Some(project) = input.project {
        let withheld = ctx.withheld(ToolCaller::Assistant).await?;
        let looked_at = [LookingAt {
            kind: LookingAtKind::Project,
            value: project,
        }];
        let found = ctx
            .projects_looked_at(&actor, &looked_at, &withheld)
            .await?;
        for project in found {
            state = ctx.ai_state_at(Some(project)).await?;
            if !state.counts() {
                return Ok(held.authorize(AssistantStatus::gone()));
            }
        }
    }
    let Ok(destination) = state.destination() else {
        return Ok(held.authorize(AssistantStatus::gone()));
    };
    let names_sent = state.names_sent(destination);
    Ok(held.authorize(AssistantStatus {
        available: true,
        destination: Some(destination),
        names_sent,
    }))
}
