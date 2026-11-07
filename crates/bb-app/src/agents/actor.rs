//! The agent as the actor of a request (AI-30, ADR-0029): who the request is from when it comes with
//! the key of an agent.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::agent::{AgentState, MOST_AGENT_KEYS};
use bb_core::id::{AgentId, ProjectId, UserId};
use bb_core::person::Person;
use bb_policy::{Narrowing, Permission};

use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// What the agent behind an agent key narrows its person to, and the request counts as the
    /// agent's from here on (what it changes is recorded as made by the agent).
    ///
    /// The agent is looked at at every request, so a person who stops it, pauses it or hands it
    /// to somebody else stops it at once. Its work is read through the port; until hand-overs and
    /// standing rules exist it has none and counts nowhere. Where its person loses *Create agents
    /// that work for you* the agent counts nowhere, because the policy cuts it there
    /// (`Narrowing::agent`).
    ///
    /// # Errors
    /// `auth.not-signed-in`, like any key that does not work, for an agent that is not there, is
    /// not active, or does not belong to the owner of the key.
    pub(crate) async fn agent_narrowing(
        &self,
        agent: &AgentId,
        owner: &Person,
    ) -> Result<Narrowing, AppError> {
        let found = self
            .store()
            .agent(*agent)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some(found) = found.filter(|found| found.may_act() && found.person == owner.id) else {
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        };
        // A person who holds the permission to create agents in no project any more has lost the
        // right to agents: the agent's next request is refused (AI-30).
        if self
            .projects_where(found.person, Permission::CreateAgents)
            .await?
            .is_empty()
        {
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        }
        let work = self
            .store()
            .agent_work(found.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        *crate::auth::locked(&self.acting_agent) = Some((found.id, found.person));
        Ok(Narrowing::agent(work.projects))
    }

    /// The projects where `person` holds `permission`, for the whole organization read once.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn projects_where(
        &self,
        person: UserId,
        permission: Permission,
    ) -> Result<BTreeSet<ProjectId>, AppError> {
        let shown = self.shown_with_levels(person, None).await?;
        let all: Vec<ProjectId> = shown.tree().projects().collect();
        Ok(shown
            .evaluations_in(&all)
            .into_iter()
            .filter(|(_, held)| held.allows(permission))
            .map(|(project, _)| project)
            .collect())
    }

    /// Whether the agent may have one more key: it must be the person's, still there, and under
    /// the most keys. What the agent may do is its work (`Narrowing::agent`), so the key itself
    /// narrows nothing more.
    ///
    /// # Errors
    /// `resource.not-found` for an agent that is not the person's or is stopped,
    /// `request.invalid` when the agent has the most keys.
    pub(crate) async fn ensure_agent_may_have_a_key(
        &self,
        owner: UserId,
        agent: AgentId,
    ) -> Result<(), AppError> {
        let found = self
            .store()
            .agent(agent)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !found.is_some_and(|found| found.person == owner && found.state != AgentState::Stopped) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        let held = self
            .store()
            .keys_of_agent(agent)
            .await
            .map_err(|error| internal::<S>(&error))?
            .len();
        if held >= MOST_AGENT_KEYS {
            return Err(ErrorCode::InvalidRequest.into());
        }
        Ok(())
    }
}
