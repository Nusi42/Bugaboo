//! The one plan of a new agent (AI-30, ADR-0029): the record and its event, the counting inside
//! the change, and who may create agents. Creating one agent and creating several run it.

use bb_core::ErrorCode;
use bb_core::agent::{Agent, AgentLimits, AgentState, MOST_AGENTS};
use bb_core::event::RecordKind;
use bb_core::id::{AgentId, UserId};
use bb_policy::Permission;
use serde_json::json;

use crate::auth::{AppError, AuthStore, Ctx, Stops, entity, set_to};

/// A new agent of `person`, called `name`, and the event that records it: the one plan of a new
/// agent, for one agent and for several. It starts *Active* with the starting limits and no work.
///
/// # Errors
/// An internal error when no ID or event can be made.
pub(super) fn new_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    person: UserId,
    name: String,
    limits: AgentLimits,
) -> Result<(Agent, bb_core::event::DomainEvent), AppError> {
    let agent = Agent {
        id: AgentId::generate(&mut ctx.ids()),
        person,
        name,
        state: AgentState::Active,
        paused_for: None,
        changes_a_minute: limits.changes_a_minute,
        issues_at_once: limits.issues_at_once,
        attempts_on_one_issue: limits.attempts_on_one_issue,
        offered_to: None,
        made_by: person,
        made_at: ctx.now(),
        version: 1,
    };
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::Agent, agent.id),
        vec![
            set_to("name", json!(agent.name)),
            set_to("state", json!(agent.state.code())),
            set_to(bb_core::version::FIELD, json!(agent.version)),
        ],
    )?;
    Ok((agent, event))
}

/// Inside the change that makes them: adds new agents of one person. One person's agents are
/// counted one change at a time, so two requests at the same moment cannot both pass the most;
/// past it the change stops (`S::aborted`).
pub(super) async fn insert_agents<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
    agents: &[Agent],
) -> Result<(), S::Error> {
    S::lock_agents_of(transaction, person).await?;
    let held = S::agent_count_of_in(transaction, person).await?;
    let after = usize::try_from(held).map(|held| held.saturating_add(agents.len()));
    if after.map_or(true, |after| after > MOST_AGENTS) {
        return Err(S::aborted());
    }
    for agent in agents {
        S::insert_agent(transaction, agent).await?;
    }
    Ok(())
}

/// How a change that creates agents is answered when it stops: past the most agents
/// (`request.invalid`), or a name the person has used already (`name.taken`).
pub(super) const CREATING_STOPS: Stops =
    Stops::new(ErrorCode::InvalidRequest, ErrorCode::NameTaken);

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses a person who holds *Create agents that work for you* in no project.
    ///
    /// # Errors
    /// `access.forbidden` for such a person.
    pub(super) async fn ensure_may_create_agents(&self, person: UserId) -> Result<(), AppError> {
        if self
            .projects_where(person, Permission::CreateAgents)
            .await?
            .is_empty()
        {
            return Err(AppError::Refused(ErrorCode::Forbidden));
        }
        Ok(())
    }
}
