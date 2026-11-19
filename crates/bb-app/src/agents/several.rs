//! *Create several agents* (AI-30, ADR-0029 amendment M9): the agents of a list, each with one key,
//! made in one change with the password once, all or nothing.
//!
//! It runs the one plan of an agent (`new_agent`, `insert_agents`) and the one plan of a key
//! (`KeyWanted`, `a_try_at_the_key`, `insert_key`) that making one agent and one key run, so there is
//! one rule of names, limits, ends and counts.

use std::collections::BTreeSet;

use bb_auth::AccessKeySecret;
use bb_core::ErrorCode;
use bb_core::agent::{Agent, MOST_AGENTS, MOST_AGENTS_AT_ONCE};
use bb_core::agent_request::{AskedAgent, Asker, RequestState};
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{AgentRequestId, UserId};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Place};

/// The requests of programs run this plan too, from inside: its change is not open to any other
/// module.
pub(super) mod requests;

use super::plan::{CREATING_STOPS, insert_agents, new_agent};
use super::types::{CreateSeveralAgents, CreatedAgent, CreatedAgents, SeveralAgent};
use crate::access::Actor;
use crate::auth::{
    AccessKeyRecord, AppError, AuthStore, CreatedAccessKey, Ctx, KeyWanted, a_try_at_the_key,
    change_failed, entity, insert_key, internal, key_end,
};
use crate::input::clean_name;

/// The agents of a person read at once to find the names they used: all of them.
const MOST_AGENTS_READ: u64 = MOST_AGENTS as u64;

/// Where row `row` of a list of agents stands (`agents/2`).
fn in_the_list(row: usize) -> String {
    format!("agents/{row}")
}

/// The refusal of one row: the row is named by where it stands and its member (`agents/2/name`,
/// or `requests/<request>/agents/1/name` in an approval), so the person sees which one to fix.
fn at_row(place: String, member: &str) -> impl Fn(AppError) -> AppError + '_ {
    move |error| match error {
        AppError::Refused(code) => AppError::RefusedAtField {
            code,
            field: format!("{place}/{member}"),
        },
        other => other,
    }
}

/// The agents of a list and their keys, checked and ready to be written in one change.
pub(super) struct AgentsPlan {
    /// The person they are made for.
    person: UserId,
    /// The agents, in the order of the list, with the events that make them.
    agents: Vec<(Agent, DomainEvent)>,
    /// One key for each agent, without its secret yet.
    keys: Vec<AccessKeyRecord>,
    /// The requests for agents the same change answers.
    answers: Vec<RequestAnswer>,
}

/// A request answered inside the change, without its event: what the change checks and writes.
type Answered = (
    AgentRequestId,
    RequestState,
    RequestState,
    Vec<AskedAgent>,
    Asker,
);

/// A request for agents that a change moves on: approved with its agents, or collected.
pub(super) struct RequestAnswer {
    /// The request.
    pub(super) id: AgentRequestId,
    /// The state it must be in.
    pub(super) from: RequestState,
    /// The state it goes to.
    pub(super) to: RequestState,
    /// Its agents, with the agents that were made.
    pub(super) agents: Vec<AskedAgent>,
    /// The key or grant that asked: the change stops when it no longer works.
    pub(super) asker: Asker,
    /// The event of the answer.
    pub(super) event: DomainEvent,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Checks the rows and plans an agent with one key for each: names that can be used, are not
    /// in the list twice and not taken by an agent of the person; ends the organization allows; at
    /// least one row. The most agents of a person is judged in the change; the most of one list
    /// ([`MOST_AGENTS_AT_ONCE`]) by the use case that takes the list. `place` names where a row
    /// stands, for its refusal.
    ///
    /// # Errors
    /// `access.forbidden` for a person who may create agents nowhere, `request.invalid` for an
    /// empty list, and the refusal of a row at that row.
    pub(super) async fn plan_agents(
        &self,
        person: UserId,
        rows: &[SeveralAgent],
        place: impl Fn(usize) -> String,
    ) -> Result<AgentsPlan, AppError> {
        let ctx = self;
        ctx.ensure_may_create_agents(person).await?;
        if rows.is_empty() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut taken: BTreeSet<String> = ctx
            .store()
            .agents_of(person, None, MOST_AGENTS_READ)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|agent| agent.name)
            .collect();
        let settings = ctx.settings().await?;
        let limits = ctx.starting_limits().await?;
        let mut agents = Vec::with_capacity(rows.len());
        let mut keys = Vec::with_capacity(rows.len());
        for (row, wanted) in rows.iter().enumerate() {
            let name = clean_name(&wanted.name).map_err(at_row(place(row), "name"))?;
            if !taken.insert(name.clone()) {
                return Err(at_row(place(row), "name")(ErrorCode::NameTaken.into()));
            }
            let expires_at = key_end(ctx.now(), wanted.expires_at.as_deref(), &settings)
                .map_err(at_row(place(row), "expires_at"))?;
            let (agent, event) = new_agent(ctx, person, name.clone(), limits)?;
            keys.push(KeyWanted::of_agent(agent.id, name, expires_at).draft(ctx, person));
            agents.push((agent, event));
        }
        Ok(AgentsPlan {
            person,
            agents,
            keys,
            answers: Vec::new(),
        })
    }
}

/// Refuses a list longer than [`MOST_AGENTS_AT_ONCE`].
///
/// # Errors
/// `request.invalid` for such a list.
pub(super) fn ensure_at_once<T>(rows: &[T]) -> Result<(), AppError> {
    if rows.len() > MOST_AGENTS_AT_ONCE {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(())
}

impl AgentsPlan {
    /// The keys of agents that exist already, with nothing else: the keys a pickup makes.
    pub(super) fn keys_only(person: UserId, keys: Vec<AccessKeyRecord>) -> Self {
        Self {
            person,
            agents: Vec::new(),
            keys,
            answers: Vec::new(),
        }
    }

    /// The agents without their keys: an approval makes the agents, and the keys wait for the
    /// pickup, so no key is shown to the person or kept anywhere.
    pub(super) fn without_keys(mut self) -> Self {
        self.keys.clear();
        self
    }

    /// The agents that were planned, in order.
    pub(super) fn agents(&self) -> impl Iterator<Item = &Agent> {
        self.agents.iter().map(|(agent, _)| agent)
    }

    /// The same change also answers these requests; it stops unless each is still in its state.
    pub(super) fn answering(mut self, answers: Vec<RequestAnswer>) -> Self {
        self.answers = answers;
        self
    }

    /// Writes every agent and its key in one change, each with its event, and the security log
    /// entry of each key after it; answers the keys with their secrets, in the order of the list.
    /// Two keys can (very rarely) draw the same public prefix: the change is then made once more
    /// with new secrets.
    ///
    /// # Errors
    /// `request.invalid` past the most agents or keys, `name.taken` for a name taken meanwhile.
    async fn record_change<S: AuthStore>(
        self,
        ctx: &Ctx<'_, S>,
    ) -> Result<(Vec<Agent>, Vec<(AccessKeySecret, AccessKeyRecord)>), AppError> {
        let person = self.person;
        let (agents, mut agent_events): (Vec<Agent>, Vec<DomainEvent>) =
            self.agents.into_iter().unzip();
        let answers: Vec<Answered> = self
            .answers
            .into_iter()
            .map(|answer| {
                agent_events.push(answer.event);
                (
                    answer.id,
                    answer.from,
                    answer.to,
                    answer.agents,
                    answer.asker,
                )
            })
            .collect();
        let now = ctx.now();
        let mut clashed = false;
        let keys = loop {
            let mut events = agent_events.clone();
            let mut tries = Vec::with_capacity(self.keys.len());
            for draft in &self.keys {
                let (secret, stored, event) = a_try_at_the_key(ctx, draft)?;
                events.push(event);
                tries.push((secret, stored));
            }
            let to_store: Vec<AccessKeyRecord> =
                tries.iter().map(|(_, stored)| stored.clone()).collect();
            let new_agents = agents.clone();
            let answered = answers.clone();
            let result = ctx
                .store()
                .record_changes(&events, move |transaction| {
                    Box::pin(async move {
                        insert_agents::<S>(transaction, person, &new_agents).await?;
                        for key in &to_store {
                            insert_key::<S>(transaction, key, key.key.agent, None).await?;
                        }
                        for (id, from, to, agents, asker) in &answered {
                            // A key or grant taken back while the person approved or the program
                            // collected stops the change: nothing is made for a dead credential.
                            if !requests::asker_works_in::<S>(transaction, *asker, now).await?
                                || !S::answer_agent_request(
                                    transaction,
                                    *id,
                                    *from,
                                    *to,
                                    now,
                                    agents,
                                )
                                .await?
                            {
                                return Err(S::aborted());
                            }
                        }
                        Ok(())
                    })
                })
                .await;
            match result {
                Ok(()) => break tries,
                Err(error) if S::is_duplicate(&error) && !clashed && !self.keys.is_empty() => {
                    clashed = true;
                }
                Err(error) => return Err(change_failed::<S>(&error, CREATING_STOPS)),
            }
        };
        for (_, stored) in &keys {
            ctx.note(
                SecurityAction::AccessKeyCreated,
                Some(person),
                Some(entity(RecordKind::AccessKey, stored.key.id)),
            )
            .await;
        }
        if !keys.is_empty() {
            ctx.write_replay_list().await;
        }
        Ok((agents, keys))
    }
}

/// Makes every agent of the list with one key each, in one change, after the password once. When
/// one row cannot be made, none is.
///
/// # Errors
/// `auth.wrong-credentials` for a wrong password, `access.forbidden` for a person who may create
/// agents nowhere, `request.invalid` for an empty or too long list or past the most agents, and a
/// row's refusal at that row (`agents/<row>/name` or `agents/<row>/expires_at`).
pub async fn create_several_agents<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateSeveralAgents,
) -> Result<Authorized<CreatedAgents>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    ensure_at_once(&input.agents)?;
    // The plan first: a person who may create agents nowhere is told so whatever they typed.
    let plan = ctx.plan_agents(person, &input.agents, in_the_list).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let (agents, keys) = plan.record_change(ctx).await?;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let mut items = Vec::with_capacity(agents.len());
    for (agent, (secret, stored)) in agents.iter().zip(keys) {
        items.push(CreatedAgent {
            agent: ctx.agent_view(&actor, agent, true).await?,
            key: CreatedAccessKey {
                key: ctx.key_view(&actor, &stored).await?,
                secret: secret.encode().as_str().to_owned(),
            },
        });
    }
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(CreatedAgents { items }))
}
