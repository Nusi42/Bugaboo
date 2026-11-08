//! The agent as the actor of a request (AI-30, ADR-0029): who the request is from when it comes with
//! the key of an agent.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::agent::{AgentLimits, AgentState, MOST_AGENT_KEYS};
use bb_core::agent_work::StopCause;
use bb_core::id::{AccessKeyId, AgentId, ProjectId, UserId};
use bb_core::person::Person;
use bb_core::settings::{
    AGENTS_ENABLED, AGENTS_MAXIMUM_ATTEMPTS_ON_ONE_ISSUE, AGENTS_MAXIMUM_CHANGES_A_MINUTE,
    AGENTS_MAXIMUM_ISSUES_AT_ONCE, SettingValue,
};
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
            // The person has lost the right to agents: the work they gave stops (AI-33).
            self.stop_work_of_agent(found.id, StopCause::AccessLost)
                .await?;
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        }
        // Agents off for the organization refuse every agent; off for a project, they work nowhere
        // in it or below it.
        if !self.agents_on_in_the_organization().await? {
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        }
        // The change this request makes is counted against the limits of the agent before anything
        // is written; over the limit the agent is paused and the request is refused.
        self.count_agent_change(&found).await?;
        let work = self.work_of_agent_now(found.id).await?;
        let off = self.projects_with_agents_off(found.person).await?;
        *crate::auth::locked(&self.acting_agent) = Some((found.id, found.person));
        Ok(Narrowing::agent(work.projects.clone(), off))
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

    /// The name of a key of the agent that a new key takes the place of.
    ///
    /// # Errors
    /// `resource.not-found` when the key is not a key of this agent of the person.
    pub(crate) async fn renewed_key_name(
        &self,
        owner: UserId,
        agent: AgentId,
        key: AccessKeyId,
    ) -> Result<String, AppError> {
        let keys = self
            .store()
            .keys_of_agent(agent)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let found = keys
            .into_iter()
            .find(|held| held.key.id == key && held.key.owner == owner)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        // The agent itself must still be the person's and not stopped, like for a new key.
        self.ensure_agent_may_have_a_key(owner, agent)
            .await
            .or_else(|error| match error {
                AppError::Refused(ErrorCode::InvalidRequest) => Ok(()),
                other => Err(other),
            })?;
        Ok(found.key.name)
    }

    /// Whether agents may work in the organization at all.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn agents_on_in_the_organization(&self) -> Result<bool, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(layers.get(&AGENTS_ENABLED).unwrap_or(true))
    }

    /// The projects, of those the person sees, where agents are switched off: the nearest value in
    /// the chain of a project decides (its own, else its parent's, up to the organization's), so a
    /// sub-project that is off stays off when the work names its parent, and one that is switched
    /// on again is on below a parent that is off. One read of the values of the projects.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn projects_with_agents_off(
        &self,
        person: UserId,
    ) -> Result<BTreeSet<ProjectId>, AppError> {
        let own: std::collections::BTreeMap<ProjectId, bool> = self
            .store()
            .project_values(AGENTS_ENABLED.definition())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter_map(|(project, value)| match value {
                SettingValue::Bool(on) => Some((project, on)),
                _ => None,
            })
            .collect();
        let organization_on = self.agents_on_in_the_organization().await?;
        let shown = self.shown_with_levels(person, None).await?;
        let tree = shown.tree();
        Ok(tree
            .projects()
            .filter(|project| {
                let chain = tree.chain(*project).unwrap_or_else(|| vec![*project]);
                // `chain` runs from the top down: the nearest value is the last one set.
                let on = chain
                    .iter()
                    .rev()
                    .find_map(|each| own.get(each).copied())
                    .unwrap_or(organization_on);
                !on
            })
            .collect())
    }

    /// The most any agent can be given as its limits: the maximums that the organization set.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn agent_maximums(&self) -> Result<AgentLimits, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(AgentLimits {
            changes_a_minute: layers.number(&AGENTS_MAXIMUM_CHANGES_A_MINUTE),
            issues_at_once: layers.number(&AGENTS_MAXIMUM_ISSUES_AT_ONCE),
            attempts_on_one_issue: layers.number(&AGENTS_MAXIMUM_ATTEMPTS_ON_ONE_ISSUE),
        })
    }

    /// The limits a new agent starts with: the defaults of ADR-0029, cut to the maximums that the
    /// organization set.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn starting_limits(&self) -> Result<AgentLimits, AppError> {
        Ok(AgentLimits::default().within(self.agent_maximums().await?))
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
