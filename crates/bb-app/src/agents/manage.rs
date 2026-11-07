//! The use cases of agents (AI-30, ADR-0029): a person makes agents of their own, renames, pauses,
//! resumes and stops them, hands them to somebody who accepts them; an administrator sees every
//! agent and stops any of them.
//!
//! Making, changing, handing and stopping start at a session: an agent, a key and an app can never
//! make or change agents. Reading one agent (`agent`) also takes the agent's own key, and then the
//! agent reads only its own record, whatever work it has; a key of a person is turned away.

use bb_core::ErrorCode;
use bb_core::agent::{Agent, AgentState, MOST_AGENTS};
use bb_core::event::RecordKind;
use bb_core::id::{AgentId, UserId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::sign_in::SecurityAction;
use bb_core::version::counted_up;
use bb_policy::guards::Refusal;
use bb_policy::{ActorKind, Authorized, Permission, Place};
use serde_json::json;

use super::types::{
    AcceptAgent, AgentAction, AgentPage, AgentQuery, AgentView, AgentsQuery, ChangeAgent,
    CreateAgent, HandAgent, RenameAgent, StopAllAgents,
};
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, change_failed, internal};
use crate::auth::{entity, set_to};
use crate::input::{clean_name, cursor_or_invalid};

/// The event of a change to an agent that counted its version up.
fn counted<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    by: UserId,
    agent: AgentId,
    mut changes: Vec<bb_core::event::FieldChange>,
    version: i64,
) -> Result<bb_core::event::DomainEvent, AppError> {
    changes.push(counted_up(version));
    ctx.event(
        crate::auth::by(by),
        entity(RecordKind::Agent, agent),
        changes,
    )
}

/// The answer for a change made on a version of an agent that is not the current one: the person
/// is told what changed meanwhile (`edit.version-changed` with the conflict).
fn stale_agent(person: UserId, agent: AgentId) -> AppError {
    AppError::stale(
        entity(RecordKind::Agent, agent),
        Place::Organization,
        person,
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The agent as the viewer may see it: its person as *Who sees people* says for the viewer.
    async fn agent_view(
        &self,
        viewer: &crate::access::Actor,
        agent: &Agent,
        with_keys: bool,
    ) -> Result<AgentView, AppError> {
        let access = self.evaluation_of(viewer, Place::Organization).await?;
        let person = self
            .person_ref(
                &access,
                bb_core::people_kinds::PeopleKind::ProjectMembers,
                bb_policy::PeopleContext::Ordinary,
                agent.person,
            )
            .await?;
        // The keys are listed with one agent, not in a list of agents (one read for each agent).
        let keys = if with_keys {
            self.store()
                .keys_of_agent(agent.id)
                .await
                .map_err(|error| internal::<S>(&error))?
        } else {
            Vec::new()
        };
        let mut shown = Vec::with_capacity(keys.len());
        for key in &keys {
            shown.push(self.key_view(viewer, key).await?);
        }
        Ok(AgentView {
            id: agent.id,
            name: agent.name.clone(),
            state: agent.state,
            person,
            limits: agent.limits(),
            offered: agent.offered_to.is_some(),
            version: agent.version,
            made_at: agent.made_at,
            keys: shown,
        })
    }

    /// The agent with this ID, if the person may handle it: their own, or any for an
    /// administrator. Anybody else is told there is no such agent.
    async fn agent_to_handle(
        &self,
        person: UserId,
        administrator: bool,
        id: AgentId,
    ) -> Result<Agent, AppError> {
        let found = self
            .store()
            .agent(id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        found
            .filter(|agent| agent.person == person || administrator)
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// Whether the person may see and stop every agent of the organization.
    async fn administers_agents(&self, person: UserId) -> Result<bool, AppError> {
        Ok(self
            .evaluate(person, Place::Organization)
            .await?
            .allows(Permission::ChangeOrganizationSettings))
    }
}

/// Makes an agent for the person. It starts *Active* with the default limits and no work.
///
/// # Errors
/// `access.forbidden` for a person who holds *Create agents that work for you* in no project,
/// `request.invalid` for a name that cannot be used or too many agents, `name.taken` for a name
/// the person has used already.
pub async fn create_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateAgent,
) -> Result<Authorized<AgentView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    if ctx
        .projects_where(person, Permission::CreateAgents)
        .await?
        .is_empty()
    {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    }
    let name = clean_name(&input.name)?;
    let limits = ctx.starting_limits().await?;
    let agent = Agent {
        id: AgentId::generate(&mut ctx.ids()),
        person,
        name,
        state: AgentState::Active,
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
    let to_store = agent.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // One person's agents are counted one change at a time, so two requests at the
                // same moment cannot both pass the most.
                S::lock_agents_of(transaction, person).await?;
                let held = S::agent_count_of_in(transaction, person).await?;
                if usize::try_from(held).map_or(true, |held| held >= MOST_AGENTS) {
                    return Err(S::aborted());
                }
                S::insert_agent(transaction, &to_store).await
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(
                &error,
                Stops::new(ErrorCode::InvalidRequest, ErrorCode::NameTaken),
            )
        })?;
    let actor = crate::access::Actor {
        person: record.person,
        key: None,
    };
    let view = ctx.agent_view(&actor, &agent, true).await?;
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(view))
}

/// The agents of the person, one page at a time (by ID).
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn my_agents<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentsQuery,
) -> Result<Authorized<AgentPage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), |cursor| {
        cursor.parse::<AgentId>().ok()
    })?;
    let page = Cut::of(
        ctx.store()
            .agents_of(person, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let actor = crate::access::Actor {
        person: record.person,
        key: None,
    };
    let next_cursor = page.next_cursor(|last| last.id.to_string());
    let people: Vec<UserId> = page.items.iter().map(|agent| agent.person).collect();
    ctx.preload_people(&people).await?;
    let mut items = Vec::with_capacity(page.items.len());
    for agent in &page.items {
        items.push(ctx.agent_view(&actor, agent, false).await?);
    }
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(AgentPage { items, next_cursor }))
}

/// Every agent of the organization, for an administrator (*Organization settings › Agents*).
///
/// # Errors
/// `access.forbidden` for anybody who may not change the organization settings.
pub async fn organization_agents<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentsQuery,
) -> Result<Authorized<AgentPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), |cursor| {
        cursor.parse::<AgentId>().ok()
    })?;
    let page = Cut::of(
        ctx.store()
            .all_agents(after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let next_cursor = page.next_cursor(|last| last.id.to_string());
    let people: Vec<UserId> = page.items.iter().map(|agent| agent.person).collect();
    ctx.preload_people(&people).await?;
    let mut items = Vec::with_capacity(page.items.len());
    for agent in &page.items {
        items.push(ctx.agent_view(&actor, agent, false).await?);
    }
    Ok(access.authorize(AgentPage { items, next_cursor }))
}

/// One agent. The person responsible for it and an administrator read any field of it; the agent
/// itself reads only its own record, whatever work it has.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's (it does not say whether it exists).
pub async fn agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentQuery,
) -> Result<Authorized<AgentView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = if let ActorKind::Agent { agent: own } = actor.kind() {
        // An agent reads its own record and nothing else.
        if own != input.id {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        ctx.store()
            .agent(own)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?
    } else if actor.key_scope().is_some() {
        // The keys of an agent are the agent's sign-in data: a key of the person does not reach
        // them.
        return Err(AppError::Refused(ErrorCode::AccessKeyNotAllowed));
    } else {
        let administrator = ctx.administers_agents(actor.person.id).await?;
        ctx.agent_to_handle(actor.person.id, administrator, input.id)
            .await?
    };
    let view = ctx.agent_view(&actor, &found, true).await?;
    Ok(ctx
        .evaluation_of(&actor, Place::Organization)
        .await?
        .authorize(view))
}

/// Gives the agent another name.
///
/// # Errors
/// `resource.not-found`, `request.invalid` for a name that cannot be used, `name.taken` for a name
/// the person has used already, `edit.version-changed` when the agent changed meanwhile.
pub async fn rename_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RenameAgent,
) -> Result<Authorized<AgentView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let name = clean_name(&input.name)?;
    let found = ctx.agent_to_handle(person, false, input.id).await?;
    if found.version != input.version {
        return Err(stale_agent(person, found.id));
    }
    let event = counted(
        ctx,
        person,
        found.id,
        vec![bb_core::event::FieldChange {
            field: "name".to_owned(),
            before: Some(json!(found.name)),
            after: Some(json!(name)),
        }],
        found.version,
    )?;
    let (id, version, new_name) = (found.id, found.version, name.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::rename_agent(transaction, id, &new_name, version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(
                &error,
                Stops::new(ErrorCode::VersionChanged, ErrorCode::NameTaken),
            )
            .about(&entity(RecordKind::Agent, id), Place::Organization, person)
        })?;
    agent(
        ctx,
        AgentQuery {
            session_token: input.session_token,
            id,
        },
    )
    .await
}

/// Pauses, resumes or stops an agent. The person responsible may do all three to their own agent;
/// an administrator may stop any agent, and the stop is written into the security log. A stopped
/// agent does not work again. Its keys stop working with it, because the next request of an agent
/// that is not active is refused.
///
/// # Errors
/// `resource.not-found`, `request.invalid` for a change the state does not allow (resuming an agent
/// that is not paused), `edit.version-changed` when the agent changed meanwhile.
pub async fn change_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeAgent,
) -> Result<Authorized<AgentView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let administrator = ctx.administers_agents(person).await?;
    let found = ctx.agent_to_handle(person, administrator, input.id).await?;
    let own = found.person == person;
    // An administrator may only stop an agent of somebody else; pausing and resuming are the
    // responsible person's.
    if !own && input.action != AgentAction::Stop {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let to = match (input.action, found.state) {
        (AgentAction::Pause, AgentState::Active) => AgentState::Paused,
        (AgentAction::Resume, AgentState::Paused) => AgentState::Active,
        (AgentAction::Stop, AgentState::Active | AgentState::Paused) => AgentState::Stopped,
        _ => return Err(ErrorCode::InvalidRequest.into()),
    };
    if found.version != input.version {
        return Err(stale_agent(person, found.id));
    }
    let event = counted(
        ctx,
        person,
        found.id,
        vec![bb_core::event::FieldChange {
            field: "state".to_owned(),
            before: Some(json!(found.state.code())),
            after: Some(json!(to.code())),
        }],
        found.version,
    )?;
    let (id, version) = (found.id, found.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::set_agent_state(transaction, id, to, version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)).about(
                &entity(RecordKind::Agent, id),
                Place::Organization,
                person,
            )
        })?;
    if !own {
        ctx.note(
            SecurityAction::AgentStopped,
            Some(person),
            Some(entity(RecordKind::Agent, found.id)),
        )
        .await;
    }
    agent(
        ctx,
        AgentQuery {
            session_token: input.session_token,
            id: found.id,
        },
    )
    .await
}

/// *Stop all my agents*: every agent of the person that is not stopped yet is stopped, in one
/// change.
///
/// # Errors
/// None of its own.
pub async fn stop_all_agents<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StopAllAgents,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let event = ctx.event(
        crate::auth::by(person),
        entity(RecordKind::Person, person),
        vec![bb_core::event::FieldChange {
            field: "agents".to_owned(),
            before: Some(json!("running")),
            after: Some(json!("stopped")),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::stop_agents_of(transaction, person).await.map(drop) })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(Done)
}

/// Hands the agent to another person. The other person has to accept (`accept_agent`); until
/// then the agent stays with its person. A person who holds *Create agents that work for you* in
/// no project cannot be handed one.
///
/// # Errors
/// `resource.not-found` for an agent that is not the person's or a person who is not there,
/// `access.forbidden` when the other person may not have agents, `edit.version-changed`.
pub async fn hand_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HandAgent,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let found = ctx.agent_to_handle(person, false, input.id).await?;
    if found.version != input.version || found.state == AgentState::Stopped {
        return Err(stale_agent(person, found.id));
    }
    if ctx.person_known(input.to).await?.is_none() || input.to == person {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    if ctx
        .projects_where(input.to, Permission::CreateAgents)
        .await?
        .is_empty()
    {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    }
    let event = counted(
        ctx,
        person,
        found.id,
        vec![set_to("offered_to", json!(input.to.to_string()))],
        found.version,
    )?;
    let (id, version, to) = (found.id, found.version, input.to);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::offer_agent(transaction, id, Some(to), version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)).about(
                &entity(RecordKind::Agent, id),
                Place::Organization,
                person,
            )
        })?;
    Ok(Done)
}

/// Accepts an agent that was handed over. The agent becomes the person's; its keys are taken back,
/// because they were made by the one who handed it over, and the new person makes their own.
///
/// # Errors
/// `resource.not-found` for an agent that was not handed to the person.
pub async fn accept_agent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AcceptAgent,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let found = ctx
        .store()
        .agent(input.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|agent| agent.offered_to == Some(person) && agent.state != AgentState::Stopped)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    // The receiver must still be a person who may have agents, like when it was offered.
    if ctx
        .projects_where(person, Permission::CreateAgents)
        .await?
        .is_empty()
    {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    }
    let event = counted(
        ctx,
        person,
        found.id,
        vec![bb_core::event::FieldChange {
            field: "person".to_owned(),
            before: Some(json!(found.person.to_string())),
            after: Some(json!(person.to_string())),
        }],
        found.version,
    )?;
    let (id, version) = (found.id, found.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // The receiver's agents are counted one change at a time, like when one is made.
                S::lock_agents_of(transaction, person).await?;
                let held = S::agent_count_of_in(transaction, person).await?;
                if usize::try_from(held).map_or(true, |held| held >= MOST_AGENTS) {
                    return Err(S::refused(Refusal::TooMuch));
                }
                S::delete_keys_of_agent(transaction, id).await?;
                if S::hand_agent_to(transaction, id, person, version).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    Ok(Done)
}
