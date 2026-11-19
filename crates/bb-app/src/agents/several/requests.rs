//! Requests for agents (AI-30, ADR-0029 amendment M9): a program asks a person for agents, the
//! person approves with the password once or declines, and the credential that asked collects the
//! keys once.
//!
//! The security line: a key or a grant may **ask**; it never makes a key by itself. An approval
//! runs the plan of *Create several agents* without keys, so the person is shown none; the keys are
//! made at the pickup, in the change that marks the request collected, so no secret of a key is
//! ever stored.

use bb_core::ErrorCode;
use bb_core::agent_request::{
    AgentRequest, AskedAgent, Asker, MOST_APPROVED_AT_ONCE, MOST_WAITING_REQUESTS, REQUESTS_LISTED,
    RequestStands, RequestState, waits,
};
use bb_core::event::{Actor as EventActor, DomainEvent, RecordKind};
use bb_core::id::{AgentId, AgentRequestId, UserId};
use bb_core::keeping::{KeepKind, Step};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::{AgentsPlan, RequestAnswer, at_row, ensure_at_once, in_the_list};
use crate::access::{Actor, CredentialSource};
use crate::agents::types::{
    AgentRequestPage, AgentRequestView, AgentRequestsQuery, ApproveAgentRequests, AskForAgents,
    CollectAgentKeys, CollectedKey, CollectedKeys, DeclineAgentRequest, SeveralAgent,
};
use crate::auth::web_address_of;
use crate::auth::{
    AppError, AuthStore, Ctx, KeyWanted, Stops, by, change_failed, entity, internal,
    key_end_in_days, set_to,
};
use crate::input::{clean_name, given_reason};
use crate::keeping::KEEP_STEP_ROWS;

/// Who asks, from the credential of the request: an access key of the person or the grant of an
/// app. A session (the assistant, the browser) and an agent never ask (ADR-0029, decision 3).
fn asker_of(actor: &Actor) -> Result<Asker, AppError> {
    let Some(key) = actor.key.as_ref().filter(|key| key.agent.is_none()) else {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    };
    Ok(match &key.source {
        CredentialSource::Key(id) => Asker::Key(*id),
        CredentialSource::App { grant, .. } => Asker::Grant(*grant),
    })
}

/// Whether the key or grant that asked still works at `now`, read on `transaction`: inside a change
/// it sees a key or grant taken back while the change waited.
///
/// # Errors
/// The error of the store.
pub(super) async fn asker_works_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    asker: Asker,
    now: Timestamp,
) -> Result<bool, S::Error> {
    Ok(match asker {
        Asker::Key(id) => S::access_key_in(transaction, id)
            .await?
            .is_some_and(|key| key.works_at(now)),
        Asker::Grant(id) => S::grant_in(transaction, id)
            .await?
            .is_some_and(|grant| grant.works_at(now)),
    })
}

/// Where the agents of `requests` stand in an approval, in the order of their rows
/// (`requests/<request>/agents/1`), so a refused row names its request.
fn places_of(requests: &[AgentRequest]) -> Vec<String> {
    requests
        .iter()
        .flat_map(|request| {
            (0..request.agents.len())
                .map(move |row| format!("requests/{}/agents/{row}", request.id))
        })
        .collect()
}

/// The event of a request: what it became.
fn request_event<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    person: UserId,
    id: AgentRequestId,
    state: RequestState,
) -> Result<DomainEvent, AppError> {
    ctx.event(
        by(person),
        entity(RecordKind::AgentRequest, id),
        vec![set_to("state", json!(state.code()))],
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person is shown for who asked: the name of the key; or the app's own name with the
    /// web address it comes back to, which Bugaboo checked when the person allowed it.
    async fn asker_shown(
        &self,
        actor: &Actor,
        asker: Asker,
    ) -> Result<(String, Option<String>), AppError> {
        match asker {
            Asker::Key(id) => Ok((
                self.store()
                    .access_key(id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .map(|key| key.name)
                    .unwrap_or_default(),
                None,
            )),
            Asker::Grant(id) => {
                let client = match self
                    .store()
                    .grant(id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                {
                    Some(grant) => self
                        .store()
                        .oauth_client(grant.client)
                        .await
                        .map_err(|error| internal::<S>(&error))?,
                    None => None,
                };
                Ok((
                    actor.app_name().unwrap_or_default(),
                    client.and_then(|client| web_address_of(&client.return_addresses)),
                ))
            }
        }
    }

    /// Checks what a program asks for and plans the request: the asker may change things and holds
    /// *Create agents that work for you* where it counts, the list is not empty and not too long,
    /// the names and what each is for can be used, the days are allowed, and the reason is one.
    ///
    /// # Errors
    /// `access.forbidden` for a session, an agent, or a credential that may not create agents;
    /// `ai.off` while agents are off in the organization; `request.invalid` for a list, a reason
    /// or a row that cannot be used (the row named).
    pub(crate) async fn preview_ask_for_agents(
        &self,
        input: &AskForAgents,
    ) -> Result<(AgentRequest, DomainEvent), AppError> {
        let actor = self.actor(&input.session_token).await?;
        let asker = asker_of(&actor)?;
        // A request leads to agents, and agents that are off in the organization are made for
        // nothing: the program is told so before the person is asked.
        if !self.agents_on_in_the_organization().await? {
            return Err(ErrorCode::AiOff.into());
        }
        let person = actor.person.id;
        let mut may = false;
        for project in self
            .projects_where(person, Permission::CreateAgents)
            .await?
        {
            if self
                .evaluation_of(&actor, Place::Project(project))
                .await?
                .allows(Permission::CreateAgents)
            {
                may = true;
                break;
            }
        }
        if !may {
            return Err(AppError::Refused(ErrorCode::Forbidden));
        }
        if input.agents.is_empty() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        ensure_at_once(&input.agents)?;
        let reason = given_reason(Some(&input.reason), ErrorCode::InvalidRequest)?;
        let settings = self.settings().await?;
        let now = self.now();
        let mut agents = Vec::with_capacity(input.agents.len());
        for (row, wanted) in input.agents.iter().enumerate() {
            let name = clean_name(&wanted.name).map_err(at_row(in_the_list(row), "name"))?;
            if agents.iter().any(|known: &AskedAgent| known.name == name) {
                return Err(at_row(in_the_list(row), "name")(
                    ErrorCode::NameTaken.into(),
                ));
            }
            let purpose =
                clean_name(&wanted.purpose).map_err(at_row(in_the_list(row), "purpose"))?;
            key_end_in_days(now, wanted.key_days, &settings)
                .map_err(at_row(in_the_list(row), "key_days"))?;
            agents.push(AskedAgent {
                name,
                purpose,
                key_days: wanted.key_days,
                agent: None,
            });
        }
        let (asked_by_name, asked_from) = self.asker_shown(&actor, asker).await?;
        let id = AgentRequestId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let request = AgentRequest {
            id,
            person,
            asked_by_key: match asker {
                Asker::Key(key) => Some(key),
                Asker::Grant(_) => None,
            },
            asked_by_grant: match asker {
                Asker::Grant(grant) => Some(grant),
                Asker::Key(_) => None,
            },
            asked_by_name,
            asked_from,
            reason,
            agents,
            state: RequestState::Waiting,
            asked_at: now,
            answered_at: None,
        };
        let event = request_event(self, person, id, RequestState::Waiting)?;
        Ok((request, event))
    }

    /// The request a credential asks to collect, and its person: only the same access key or grant
    /// that asked gets it; another credential is told there is none, and the security log says so.
    ///
    /// # Errors
    /// `resource.not-found` for a request of somebody else or of another credential,
    /// `access.forbidden` for a session or an agent.
    async fn request_to_collect(
        &self,
        input: &CollectAgentKeys,
    ) -> Result<(AgentRequest, UserId), AppError> {
        let actor = self.actor(&input.session_token).await?;
        let asker = asker_of(&actor)?;
        let person = actor.person.id;
        let request = self.request_of(person, input.id).await?;
        if request.asker() != Some(asker) {
            self.note(
                SecurityAction::AgentKeysRefused,
                Some(person),
                Some(entity(RecordKind::AgentRequest, request.id)),
            )
            .await;
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        Ok((request, person))
    }

    /// What collecting would do now, with nothing made: where the request stands and, when it is
    /// approved, the names of the keys a pickup would make (the preview of `collect_agent_keys`).
    ///
    /// # Errors
    /// As [`collect_agent_keys`].
    pub(crate) async fn preview_collect_agent_keys(
        &self,
        input: &CollectAgentKeys,
    ) -> Result<CollectedKeys, AppError> {
        let (request, _) = self.request_to_collect(input).await?;
        let stands = request.stands_at(self.now());
        Ok(CollectedKeys {
            stands,
            keys: Vec::new(),
            names: if stands == RequestStands::Approved {
                request
                    .agents
                    .iter()
                    .map(|asked| asked.name.clone())
                    .collect()
            } else {
                Vec::new()
            },
        })
    }

    /// The request of `id` if it is the person's; anybody else is told there is none.
    async fn request_of(
        &self,
        person: UserId,
        id: AgentRequestId,
    ) -> Result<AgentRequest, AppError> {
        self.store()
            .agent_request(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|request| request.person == person)
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Deletes the requests for agents asked longer ago than their keep period: one step of the
    /// job that clears sessions (the keep rule *Requests for agents*).
    pub(crate) async fn delete_old_agent_requests(&self) -> Result<Step, AppError> {
        let before = self.keep_cutoff(KeepKind::AgentRequests).await?;
        let event = self.event(
            EventActor::System,
            crate::auth::fixed_entity(RecordKind::AgentRequest, "old-agent-requests")?,
            vec![crate::auth::changed("old-agent-requests")],
        )?;
        let deleted = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::delete_old_agent_requests(transaction, before, KEEP_STEP_ROWS).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        Ok(Step::of(deleted, KEEP_STEP_ROWS))
    }
}

/// A program asks the person for agents: only a request is made, nothing else. One key or grant
/// has at most [`MOST_WAITING_REQUESTS`] waiting.
///
/// # Errors
/// As [`Ctx::preview_ask_for_agents`]; `request.invalid` when the key or grant has the most requests
/// waiting.
pub async fn ask_for_agents<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskForAgents,
) -> Result<Authorized<AgentRequestView>, AppError> {
    let (request, event) = ctx.preview_ask_for_agents(&input).await?;
    let (person, id, asked_at) = (request.person, request.id, request.asked_at);
    let since = asked_at.checked_sub(waits()).unwrap_or(asked_at);
    let to_store = request.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_agents_of(transaction, person).await?;
                let asker = to_store.asker().ok_or_else(S::aborted)?;
                let waiting = S::waiting_requests_of_in(transaction, asker, since).await?;
                if usize::try_from(waiting).map_or(true, |held| held >= MOST_WAITING_REQUESTS) {
                    return Err(S::aborted());
                }
                S::insert_agent_request(transaction, &to_store).await
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(
                &error,
                Stops::new(ErrorCode::InvalidRequest, ErrorCode::InvalidRequest),
            )
        })?;
    ctx.note(
        SecurityAction::AgentsAskedFor,
        Some(person),
        Some(entity(RecordKind::AgentRequest, id)),
    )
    .await;
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(AgentRequestView::of(&request, ctx.now())))
}

/// The requests asked of the person in the last week, newest first, as they stand now.
///
/// # Errors
/// `auth.access-key-not-allowed` for a key: only the person in the browser answers requests.
pub async fn agent_requests<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentRequestsQuery,
) -> Result<Authorized<AgentRequestPage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let now = ctx.now();
    let since = now
        .checked_sub(bb_core::time::days(
            bb_core::agent_request::REQUEST_KEEP_DAYS,
        ))
        .unwrap_or(now);
    let mut found = ctx
        .store()
        .agent_requests_of(person, since, REQUESTS_LISTED + 1)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let truncated = found.len() > usize::try_from(REQUESTS_LISTED).unwrap_or(usize::MAX);
    found.truncate(usize::try_from(REQUESTS_LISTED).unwrap_or(usize::MAX));
    let items = found
        .iter()
        .map(|request| AgentRequestView::of(request, now))
        .collect();
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(AgentRequestPage { items, truncated }))
}

/// The person approves the ticked requests with the password once: every agent of them is made in
/// one change through the plan of *Create several agents*, without keys, and each request waits
/// for its keys to be collected. All or nothing.
///
/// # Errors
/// `resource.not-found` for a request that is not the person's; `request.invalid` at
/// `requests/<request>` for one that does not wait any more or whose key or grant no longer works,
/// and for too many requests or agents at once; `auth.wrong-credentials` for a wrong password; and
/// the refusals of the plan of several agents, at `requests/<request>/agents/<row>/<member>`.
pub async fn approve_agent_requests<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ApproveAgentRequests,
) -> Result<Authorized<AgentRequestPage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let mut ids = input.requests.clone();
    ids.sort();
    ids.dedup();
    if ids.is_empty() || ids.len() > MOST_APPROVED_AT_ONCE {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let now = ctx.now();
    let mut requests = Vec::with_capacity(ids.len());
    for id in ids {
        let request = ctx.request_of(person, id).await?;
        let at_request = |code: ErrorCode| AppError::RefusedAtField {
            code,
            field: format!("requests/{id}"),
        };
        if request.stands_at(now) != RequestStands::Waiting {
            return Err(at_request(ErrorCode::InvalidRequest));
        }
        // A key or grant taken back, renewed or run out could never collect: its request is not
        // approved, so no agents are made for nothing.
        let asker = request
            .asker()
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let works = ctx
            .store()
            .read_snapshot(move |transaction| {
                Box::pin(asker_works_in::<S>(transaction, asker, now))
            })
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !works {
            return Err(at_request(ErrorCode::InvalidRequest));
        }
        requests.push(request);
    }
    let rows: Vec<SeveralAgent> = requests
        .iter()
        .flat_map(|request| request.agents.iter())
        .map(|asked| SeveralAgent {
            name: asked.name.clone(),
            expires_at: None,
        })
        .collect();
    ensure_at_once(&rows)?;
    let places = places_of(&requests);
    let plan = ctx
        .plan_agents(person, &rows, |row| {
            places.get(row).cloned().unwrap_or_default()
        })
        .await?
        .without_keys();
    ctx.confirm_password(&record.person, input.password).await?;
    let created: Vec<AgentId> = plan.agents().map(|agent| agent.id).collect();
    let mut made = created.into_iter();
    let mut answers = Vec::with_capacity(requests.len());
    for request in &mut requests {
        for asked in &mut request.agents {
            asked.agent = made.next();
        }
        request.state = RequestState::Approved;
        request.answered_at = Some(now);
        answers.push(RequestAnswer {
            id: request.id,
            from: RequestState::Waiting,
            to: RequestState::Approved,
            agents: request.agents.clone(),
            asker: request
                .asker()
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?,
            event: request_event(ctx, person, request.id, RequestState::Approved)?,
        });
    }
    plan.answering(answers).record_change(ctx).await?;
    for request in &requests {
        ctx.note(
            SecurityAction::AgentRequestApproved,
            Some(person),
            Some(entity(RecordKind::AgentRequest, request.id)),
        )
        .await;
    }
    let items = requests
        .iter()
        .map(|request| AgentRequestView::of(request, now))
        .collect();
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(AgentRequestPage {
            items,
            truncated: false,
        }))
}

/// The person declines a request: it ends, and nothing is made. It takes away and adds nothing, so
/// the session is enough.
///
/// # Errors
/// `resource.not-found` for a request that is not the person's, `request.invalid` for one that
/// does not wait any more.
pub async fn decline_agent_request<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeclineAgentRequest,
) -> Result<Authorized<AgentRequestView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let now = ctx.now();
    let mut request = ctx.request_of(person, input.id).await?;
    if request.stands_at(now) != RequestStands::Waiting {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let event = request_event(ctx, person, request.id, RequestState::Declined)?;
    let (id, agents) = (request.id, request.agents.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::answer_agent_request(
                    transaction,
                    id,
                    RequestState::Waiting,
                    RequestState::Declined,
                    now,
                    &agents,
                )
                .await?
                {
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
                Stops::new(ErrorCode::InvalidRequest, ErrorCode::InvalidRequest),
            )
        })?;
    ctx.note(
        SecurityAction::AgentRequestDeclined,
        Some(person),
        Some(entity(RecordKind::AgentRequest, id)),
    )
    .await;
    request.state = RequestState::Declined;
    request.answered_at = Some(now);
    Ok(ctx
        .evaluate(person, Place::Organization)
        .await?
        .authorize(AgentRequestView::of(&request, now)))
}

/// The program that asked collects the keys of an approved request, once: one key for each of its
/// agents, made in the change that marks it collected. Before that it is told where the request
/// stands, and after it nothing more. Only the same access key or grant that asked may collect.
///
/// # Errors
/// `resource.not-found` for a request of somebody else or asked by another credential (written to
/// the security log), `access.forbidden` for a session or an agent, and the refusals of a key.
pub async fn collect_agent_keys<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CollectAgentKeys,
) -> Result<Authorized<CollectedKeys>, AppError> {
    let (request, person) = ctx.request_to_collect(&input).await?;
    let now = ctx.now();
    let stands = request.stands_at(now);
    let evaluation = ctx.evaluate(person, Place::Organization).await?;
    if stands != RequestStands::Approved {
        return Ok(evaluation.authorize(CollectedKeys {
            stands,
            keys: Vec::new(),
            names: Vec::new(),
        }));
    }
    let settings = ctx.settings().await?;
    let mut drafts = Vec::with_capacity(request.agents.len());
    for asked in &request.agents {
        let agent = asked
            .agent
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        ctx.ensure_agent_may_have_a_key(person, agent).await?;
        let expires_at = key_end_in_days(now, asked.key_days, &settings)?;
        drafts.push(KeyWanted::of_agent(agent, asked.name.clone(), expires_at).draft(ctx, person));
    }
    let answer = RequestAnswer {
        id: request.id,
        from: RequestState::Approved,
        to: RequestState::Collected,
        agents: request.agents.clone(),
        asker: request
            .asker()
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?,
        event: request_event(ctx, person, request.id, RequestState::Collected)?,
    };
    let (_, keys) = AgentsPlan::keys_only(person, drafts)
        .answering(vec![answer])
        .record_change(ctx)
        .await?;
    ctx.note(
        SecurityAction::AgentKeysCollected,
        Some(person),
        Some(entity(RecordKind::AgentRequest, request.id)),
    )
    .await;
    Ok(evaluation.authorize(CollectedKeys {
        stands: RequestStands::Collected,
        keys: keys
            .into_iter()
            .map(|(secret, stored)| CollectedKey {
                agent: stored.key.agent,
                name: stored.key.name,
                secret: secret.encode().as_str().to_owned(),
            })
            .collect(),
        names: Vec::new(),
    }))
}
