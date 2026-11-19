//! Requests for agents (AI-30, ADR-0029 amendment M9): a program asks with a person's key or an
//! app's grant, the person approves with the password once or declines, and only the credential
//! that asked collects the keys, once. A key never makes a key by itself.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::agent_request::{REQUEST_HOURS, REQUEST_KEEP_DAYS, RequestStands};
use bb_core::ai::{ToolCaller, ToolScope};
use bb_core::id::{AgentRequestId, ProjectId};
use bb_core::notification::NotificationKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Permission, StartingRole};
use serde_json::{Value, json};

use super::agents::{an_agent, key_of, page, person_key_request, refused};
use super::scenarios::{Added, a_project, measured, token};
use super::workflows_rules::member;
use super::world::{Outcome, PASSWORD, World};
use crate::agents::{
    AgentRequestsQuery, ApproveAgentRequests, CollectAgentKeys, DeclineAgentRequest,
    agent_requests, approve_agent_requests, ask_for_agents, collect_agent_keys,
    decline_agent_request, my_agents,
};
use crate::ai::context::PeopleMode;
use crate::ai::tools::{ToolAnswer, ToolCallQuery, ToolMode, call_tool};
use crate::auth::{AppError, CreateAccessKey, create_access_key};
use crate::notifications::{NotificationsQuery, notifications};

/// A team member of WEB, who may create agents there, and a key of theirs that may change things.
pub(super) struct Asking {
    pub(super) world: World,
    pub(super) project: ProjectId,
    pub(super) session: String,
    pub(super) key: String,
}

pub(super) async fn asking() -> Result<Asking, String> {
    let world = World::with_ai_apps_and_agents().await?;
    let project = a_project(&world, "WEB").await?;
    let team = member(
        &world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await
    .map_err(|error| error.to_string())?;
    let key = a_person_key(&world, &team.session_token, KeyChoice::SeeAndChange)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Asking {
        world,
        project,
        session: team.session_token,
        key,
    })
}

pub(super) async fn a_person_key(
    world: &World,
    session: &str,
    choice: KeyChoice,
) -> Result<String, AppError> {
    let request = person_key_request(session.to_owned(), "Script", choice, Vec::new());
    Ok(create_access_key(&world.ctx(), request)
        .await?
        .into_value()
        .secret)
}

/// What a program sends to ask for a planner and a builder.
pub(super) fn team_of_two() -> Value {
    json!({
        "reason": "A team for the release",
        "agents": [
            {"name": "Planner", "purpose": "plans"},
            {"name": "Builder", "purpose": "builds", "key_days": 30},
        ],
    })
}

pub(super) fn as_program(token: &str, tool: &str, input: Value, mode: ToolMode) -> ToolCallQuery {
    ToolCallQuery {
        session_token: token.to_owned(),
        caller: ToolCaller::AiApp,
        app: Some("Script".to_owned()),
        granted: ToolScope::SeeAndChange,
        people: PeopleMode::AsSeen,
        tool: tool.to_owned(),
        input,
        mode,
    }
}

pub(super) async fn program(
    world: &World,
    token: &str,
    tool: &str,
    input: Value,
) -> Result<ToolAnswer, AppError> {
    Ok(
        call_tool(&world.ctx(), as_program(token, tool, input, ToolMode::Run))
            .await?
            .into_value(),
    )
}

/// Asks for the team of two with `token`, and answers the ID of the request.
pub(super) async fn ask(world: &World, token: &str) -> Result<AgentRequestId, AppError> {
    program(world, token, "ask_for_agents", team_of_two()).await?;
    let newest = world
        .store
        .lock()
        .state
        .agent_requests
        .last()
        .map(|request| request.id)
        .ok_or(AppError::Internal("no request".to_owned()))?;
    Ok(newest)
}

pub(super) fn collect(token: &str, id: AgentRequestId) -> CollectAgentKeys {
    CollectAgentKeys {
        session_token: token.to_owned(),
        id,
    }
}

pub(super) async fn approve(
    world: &World,
    session: &str,
    ids: Vec<AgentRequestId>,
) -> Result<(), AppError> {
    approve_agent_requests(
        &world.ctx(),
        ApproveAgentRequests {
            session_token: session.to_owned(),
            password: PASSWORD.to_owned(),
            requests: ids,
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn agent_count(world: &World, session: &str) -> Result<usize, AppError> {
    Ok(my_agents(&world.ctx(), page(session))
        .await?
        .into_value()
        .items
        .len())
}

fn keys_stored(world: &World) -> usize {
    world.store.lock().state.access_keys.len()
}

fn logged(world: &World, action: SecurityAction) -> usize {
    world
        .store
        .lock()
        .log
        .iter()
        .filter(|entry| entry.action == action)
        .count()
}

/// The refusal of a request that should have been refused (an answer holds secrets, so it is not
/// printed).
pub(super) fn error_of<T>(result: Result<T, AppError>) -> AppError {
    match result {
        Ok(_) => AppError::Internal("it went through".to_owned()),
        Err(error) => error,
    }
}

#[tokio::test]
async fn ai_30_a_program_asks_for_agents_and_nothing_is_made() -> Outcome {
    let given = asking().await?;
    let keys_before = keys_stored(&given.world);
    let answer = program(&given.world, &given.key, "ask_for_agents", team_of_two()).await?;
    assert!(answer.text.contains("waiting"), "{}", answer.text);
    assert_eq!(agent_count(&given.world, &given.session).await?, 0);
    assert_eq!(keys_stored(&given.world), keys_before);
    assert_eq!(logged(&given.world, SecurityAction::AgentsAskedFor), 1);
    Ok(())
}

#[tokio::test]
async fn ai_30_only_a_key_or_grant_that_may_change_and_create_agents_asks() -> Outcome {
    let given = asking().await?;
    let world = &given.world;
    // A key that can only look: the tool changes something.
    let look = a_person_key(world, &given.session, KeyChoice::SeeOnly).await?;
    assert!(
        program(world, &look, "ask_for_agents", team_of_two())
            .await
            .is_err()
    );
    // A key of single permissions without *Create agents that work for you*.
    let narrow = create_access_key(
        &world.ctx(),
        CreateAccessKey {
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            choice: KeyChoice::Permissions,
            ..person_key_request(
                given.session.clone(),
                "Narrow",
                KeyChoice::Permissions,
                Vec::new(),
            )
        },
    )
    .await?
    .into_value()
    .secret;
    assert!(
        program(world, &narrow, "ask_for_agents", team_of_two())
            .await
            .is_err()
    );
    // The same with the permission asks.
    let wide = create_access_key(
        &world.ctx(),
        CreateAccessKey {
            permissions: vec![Permission::SeeProjects, Permission::CreateAgents],
            choice: KeyChoice::Permissions,
            ..person_key_request(
                given.session.clone(),
                "Wide",
                KeyChoice::Permissions,
                Vec::new(),
            )
        },
    )
    .await?
    .into_value()
    .secret;
    program(world, &wide, "ask_for_agents", team_of_two()).await?;
    // A key of a Viewer, who may create agents nowhere.
    let viewer = member(
        world,
        "viewer@example.test",
        StartingRole::Viewer,
        given.project,
    )
    .await?;
    let viewer_key = a_person_key(world, &viewer.session_token, KeyChoice::SeeAndChange).await?;
    assert!(
        program(world, &viewer_key, "ask_for_agents", team_of_two())
            .await
            .is_err()
    );
    // The assistant, in the person's session, is not a program.
    let assistant = ToolCallQuery {
        caller: ToolCaller::Assistant,
        ..as_program(
            &given.session,
            "ask_for_agents",
            team_of_two(),
            ToolMode::Apply,
        )
    };
    assert!(call_tool(&world.ctx(), assistant).await.is_err());
    // The grant of an app the person allowed to see only, even when the call claims more: the
    // tool changes something, and the grant may not.
    let (_, look_only) = super::oauth::signed_in(world, KeyChoice::SeeOnly).await?;
    assert!(
        program(
            world,
            &look_only.access_token,
            "ask_for_agents",
            team_of_two()
        )
        .await
        .is_err()
    );
    assert_eq!(
        world.store.lock().state.agent_requests.len(),
        1,
        "only the wide key asked"
    );
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_never_asks_for_agents() -> Outcome {
    let given = asking().await?;
    let fixer = an_agent(&given.world, &given.session, "Fixer").await?;
    let agent_key = key_of(&given.world, &given.session, fixer).await?;
    for caller in [ToolCaller::Agent, ToolCaller::AiApp] {
        let call = ToolCallQuery {
            caller,
            ..as_program(&agent_key, "ask_for_agents", team_of_two(), ToolMode::Run)
        };
        assert!(
            call_tool(&given.world.ctx(), call).await.is_err(),
            "{caller:?}"
        );
    }
    // Not even past the registry, at the use case.
    let direct = ask_for_agents(
        &given.world.ctx(),
        crate::agents::AskForAgents {
            session_token: agent_key,
            reason: "More of us".to_owned(),
            agents: vec![crate::agents::AgentAsked {
                name: "Child".to_owned(),
                purpose: "helps".to_owned(),
                key_days: None,
            }],
        },
    )
    .await;
    assert!(refused(&error_of(direct), ErrorCode::Forbidden));
    assert!(given.world.store.lock().state.agent_requests.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_30_a_key_or_grant_has_at_most_three_requests_waiting() -> Outcome {
    let given = asking().await?;
    for _ in 0..3 {
        ask(&given.world, &given.key).await?;
    }
    let fourth = program(&given.world, &given.key, "ask_for_agents", team_of_two()).await;
    assert!(fourth.is_err());
    // Another key of the same person has room of its own.
    let other = a_person_key(&given.world, &given.session, KeyChoice::SeeAndChange).await?;
    ask(&given.world, &other).await?;
    // A declined request waits no more, so there is room again.
    let first = given.world.store.lock().state.agent_requests[0].id;
    decline_agent_request(
        &given.world.ctx(),
        DeclineAgentRequest {
            session_token: given.session.clone(),
            id: first,
        },
    )
    .await?;
    ask(&given.world, &given.key).await?;
    Ok(())
}

#[tokio::test]
async fn ai_30_the_person_is_told_and_sees_the_waiting_requests() -> Outcome {
    let given = asking().await?;
    let id = ask(&given.world, &given.key).await?;
    let listed = agent_requests(
        &given.world.ctx(),
        AgentRequestsQuery {
            session_token: given.session.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(listed.items.len(), 1);
    let item = &listed.items[0];
    assert_eq!(item.id, id);
    assert_eq!(item.stands, RequestStands::Waiting);
    assert_eq!(item.asked_by, "Script");
    assert_eq!(item.reason, "A team for the release");
    assert_eq!(item.agents.len(), 2);
    let heard = notifications(
        &given.world.ctx(),
        NotificationsQuery {
            session_token: given.session.clone(),
            filter: bb_core::notification::NotificationFilter::All,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value()
    .items;
    assert!(
        heard
            .iter()
            .any(|item| item.kind == NotificationKind::AgentsAskedFor),
        "the person is told"
    );
    // A key never reads the list: only the person in the browser answers requests.
    let by_key = agent_requests(
        &given.world.ctx(),
        AgentRequestsQuery {
            session_token: given.key.clone(),
        },
    )
    .await;
    assert!(by_key.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_30_approving_requests_makes_their_agents_in_one_change_with_the_password_once()
-> Outcome {
    let given = asking().await?;
    let world = &given.world;
    let first = ask(world, &given.key).await?;
    let other = a_person_key(world, &given.session, KeyChoice::SeeAndChange).await?;
    program(
        world,
        &other,
        "ask_for_agents",
        json!({"reason": "A checker", "agents": [{"name": "Checker", "purpose": "checks"}]}),
    )
    .await?;
    let second = world.store.lock().state.agent_requests[1].id;
    // A wrong password approves nothing.
    let wrong = approve_agent_requests(
        &world.ctx(),
        ApproveAgentRequests {
            session_token: given.session.clone(),
            password: "not the password".to_owned(),
            requests: vec![first, second],
        },
    )
    .await;
    assert!(refused(&error_of(wrong), ErrorCode::WrongCredentials));
    assert_eq!(agent_count(world, &given.session).await?, 0);
    // A name taken in one of them refuses both.
    an_agent(world, &given.session, "Checker").await?;
    assert!(
        approve(world, &given.session, vec![first, second])
            .await
            .is_err()
    );
    assert_eq!(agent_count(world, &given.session).await?, 1);
    // Without the clash, both requests are approved in one change.
    approve(world, &given.session, vec![first]).await?;
    assert_eq!(agent_count(world, &given.session).await?, 3);
    assert_eq!(logged(world, SecurityAction::AgentRequestApproved), 1);
    // A key never approves.
    let by_key = approve(world, &given.key, vec![second]).await;
    assert!(by_key.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_30_approving_shows_no_key_to_the_person() -> Outcome {
    let given = asking().await?;
    let id = ask(&given.world, &given.key).await?;
    let keys_before = keys_stored(&given.world);
    approve(&given.world, &given.session, vec![id]).await?;
    // The agents are made, and their keys wait for the program: none exists yet.
    assert_eq!(agent_count(&given.world, &given.session).await?, 2);
    assert_eq!(keys_stored(&given.world), keys_before);
    let listed = agent_requests(
        &given.world.ctx(),
        AgentRequestsQuery {
            session_token: given.session.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(listed.items[0].stands, RequestStands::Approved);
    assert!(
        listed.items[0]
            .agents
            .iter()
            .all(|asked| asked.agent.is_some())
    );
    Ok(())
}

#[tokio::test]
async fn ai_30_a_declined_request_makes_nothing() -> Outcome {
    let given = asking().await?;
    let id = ask(&given.world, &given.key).await?;
    decline_agent_request(
        &given.world.ctx(),
        DeclineAgentRequest {
            session_token: given.session.clone(),
            id,
        },
    )
    .await?;
    assert_eq!(agent_count(&given.world, &given.session).await?, 0);
    assert!(
        approve(&given.world, &given.session, vec![id])
            .await
            .is_err()
    );
    let collected = collect_agent_keys(&given.world.ctx(), collect(&given.key, id))
        .await?
        .into_value();
    assert_eq!(collected.stands, RequestStands::Declined);
    assert!(collected.keys.is_empty());
    assert_eq!(
        logged(&given.world, SecurityAction::AgentRequestDeclined),
        1
    );
    Ok(())
}

#[tokio::test]
async fn ai_30_only_the_credential_that_asked_collects_the_keys_once() -> Outcome {
    let given = asking().await?;
    let world = &given.world;
    let id = ask(world, &given.key).await?;
    // Before the approval the program is told it waits.
    let early = collect_agent_keys(&world.ctx(), collect(&given.key, id))
        .await?
        .into_value();
    assert_eq!(early.stands, RequestStands::Waiting);
    assert!(early.keys.is_empty());
    approve(world, &given.session, vec![id]).await?;
    // Another key of the same person is told there is no such request, and it is logged.
    let other = a_person_key(world, &given.session, KeyChoice::SeeAndChange).await?;
    let stolen = collect_agent_keys(&world.ctx(), collect(&other, id)).await;
    assert!(refused(&error_of(stolen), ErrorCode::NotFound));
    assert_eq!(logged(world, SecurityAction::AgentKeysRefused), 1);
    // The person's session is no program either.
    assert!(
        collect_agent_keys(&world.ctx(), collect(&given.session, id))
            .await
            .is_err()
    );
    // The key that asked gets one key for each agent, and each works for its agent.
    let answer = program(
        world,
        &given.key,
        "collect_agent_keys",
        json!({"request": id.to_string()}),
    )
    .await?;
    let collected: Vec<String> = world
        .store
        .lock()
        .state
        .access_keys
        .iter()
        .filter(|record| record.key.agent.is_some())
        .map(|record| record.key.name.clone())
        .collect();
    assert_eq!(collected, ["Planner", "Builder"]);
    assert!(answer.text.contains("bb_"), "the keys are in the answer");
    assert_eq!(logged(world, SecurityAction::AgentKeysCollected), 1);
    assert_eq!(
        logged(world, SecurityAction::AccessKeyCreated),
        4,
        "two keys of the person and the two of the agents"
    );
    // A second pickup makes nothing more.
    let again = collect_agent_keys(&world.ctx(), collect(&given.key, id))
        .await?
        .into_value();
    assert_eq!(again.stands, RequestStands::Collected);
    assert!(again.keys.is_empty());
    assert_eq!(keys_stored(world), 4);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_request_runs_out_after_a_day_and_its_keys_cannot_be_collected_after_a_day()
-> Outcome {
    let given = asking().await?;
    let world = &given.world;
    let day = REQUEST_HOURS * 3600;
    // A request asked a day ago is not approved any more.
    let old = ask(world, &given.key).await?;
    for request in &mut world.store.lock().state.agent_requests {
        request.asked_at = request
            .asked_at
            .checked_sub(jiff::SignedDuration::from_secs(day))
            .map_err(|error| error.to_string())?;
    }
    let refusal = approve(world, &given.session, vec![old]).await;
    assert!(
        matches!(&refusal, Err(AppError::RefusedAtField { code: ErrorCode::InvalidRequest, field })
            if *field == format!("requests/{old}")),
        "the request is named"
    );
    // One waits unanswered, one is approved: a day later both ran out, and nothing is made.
    let late = ask(world, &given.key).await?;
    let id = ask(world, &given.key).await?;
    approve(world, &given.session, vec![id]).await?;
    world.clock.advance(day);
    let keys_before = keys_stored(world);
    for asked in [late, id] {
        let stands = collect_agent_keys(&world.ctx(), collect(&given.key, asked))
            .await?
            .into_value();
        assert_eq!(stands.stands, RequestStands::RanOut);
        assert!(stands.keys.is_empty());
    }
    assert_eq!(keys_stored(world), keys_before);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_key_never_makes_a_key_by_itself() -> Outcome {
    let given = asking().await?;
    let world = &given.world;
    let id = ask(world, &given.key).await?;
    // The routes that make agents, keys and approvals refuse every key.
    let fixer = an_agent(world, &given.session, "Fixer").await?;
    let agent_key = key_of(world, &given.session, fixer).await?;
    for key in [given.key.clone(), agent_key] {
        assert!(approve(world, &key, vec![id]).await.is_err());
        let one_key =
            create_access_key(&world.ctx(), super::agents::key_request(&key, fixer, None)).await;
        assert!(one_key.is_err());
        let several = crate::agents::create_several_agents(
            &world.ctx(),
            super::several_agents::several(&key, vec![super::several_agents::row("Child")]),
        )
        .await;
        assert!(several.is_err());
    }
    // Before the person approves, collecting makes no key.
    let keys_before = keys_stored(world);
    program(
        world,
        &given.key,
        "collect_agent_keys",
        json!({"request": id.to_string()}),
    )
    .await?;
    assert_eq!(keys_stored(world), keys_before);
    Ok(())
}

#[tokio::test]
async fn priv_05_requests_for_agents_are_deleted_after_their_keep_period() -> Outcome {
    let given = asking().await?;
    let world = &given.world;
    ask(world, &given.key).await?;
    world.clock.advance(REQUEST_KEEP_DAYS * 24 * 3600 - 60);
    world.ctx().purge_dead_sessions().await?;
    assert_eq!(world.store.lock().state.agent_requests.len(), 1);
    world.clock.advance(120);
    world.ctx().purge_dead_sessions().await?;
    assert!(world.store.lock().state.agent_requests.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_ask_for_agents_answers_what_would_change_and_saves_nothing() -> Outcome {
    let given = asking().await?;
    let preview = call_tool(
        &given.world.ctx(),
        as_program(
            &given.key,
            "ask_for_agents",
            team_of_two(),
            ToolMode::Preview,
        ),
    )
    .await?
    .into_value();
    assert!(preview.text.contains("Planner"), "{}", preview.text);
    assert!(given.world.store.lock().state.agent_requests.is_empty());
    let saved = program(&given.world, &given.key, "ask_for_agents", team_of_two()).await?;
    assert_ne!(preview.text, saved.text);
    assert_eq!(given.world.store.lock().state.agent_requests.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_collect_agent_keys_answers_what_would_change_and_saves_nothing() -> Outcome {
    let given = asking().await?;
    let id = ask(&given.world, &given.key).await?;
    approve(&given.world, &given.session, vec![id]).await?;
    let keys_before = keys_stored(&given.world);
    let input = json!({"request": id.to_string()});
    let preview = call_tool(
        &given.world.ctx(),
        as_program(
            &given.key,
            "collect_agent_keys",
            input.clone(),
            ToolMode::Preview,
        ),
    )
    .await?
    .into_value();
    assert!(preview.text.contains("Planner"), "{}", preview.text);
    assert_eq!(keys_stored(&given.world), keys_before);
    let saved = program(&given.world, &given.key, "collect_agent_keys", input).await?;
    assert!(saved.text.contains("bb_"));
    assert_eq!(keys_stored(&given.world), keys_before + 2);
    Ok(())
}

pub(crate) async fn ask_for_agents_scenario(world: &World) -> Added {
    world.switch_on_ai_apps_and_agents();
    a_project(world, "WEB").await?;
    let key = a_person_key(world, &token(world), KeyChoice::SeeAndChange)
        .await
        .map_err(|error| error.to_string())?;
    measured(
        &world.store,
        ask_for_agents(
            &world.ctx(),
            crate::agents::AskForAgents {
                session_token: key,
                reason: "A team".to_owned(),
                agents: vec![crate::agents::AgentAsked {
                    name: "Planner".to_owned(),
                    purpose: "plans".to_owned(),
                    key_days: None,
                }],
            },
        ),
    )
    .await
}

/// A request asked by a key of the administrator, and that key.
async fn a_request(world: &World) -> Result<(AgentRequestId, String), String> {
    world.switch_on_ai_apps_and_agents();
    a_project(world, "WEB").await?;
    let key = a_person_key(world, &token(world), KeyChoice::SeeAndChange)
        .await
        .map_err(|error| error.to_string())?;
    let id = ask(world, &key).await.map_err(|error| error.to_string())?;
    Ok((id, key))
}

pub(crate) async fn approve_agent_requests_scenario(world: &World) -> Added {
    let (id, _) = a_request(world).await?;
    measured(&world.store, approve(world, &token(world), vec![id])).await
}

pub(crate) async fn decline_agent_request_scenario(world: &World) -> Added {
    let (id, _) = a_request(world).await?;
    measured(
        &world.store,
        decline_agent_request(
            &world.ctx(),
            DeclineAgentRequest {
                session_token: token(world),
                id,
            },
        ),
    )
    .await
}

pub(crate) async fn collect_agent_keys_scenario(world: &World) -> Added {
    let (id, key) = a_request(world).await?;
    approve(world, &token(world), vec![id])
        .await
        .map_err(|error| error.to_string())?;
    measured(
        &world.store,
        collect_agent_keys(&world.ctx(), collect(&key, id)),
    )
    .await
}

#[tokio::test]
async fn ai_30_a_grant_of_an_app_asks_and_only_that_grant_collects() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    a_project(&world, "WEB").await?;
    let (_, tokens) = super::oauth::signed_in(&world, KeyChoice::SeeAndChange).await?;
    let grant = tokens.access_token.to_string();
    let id = ask(&world, &grant).await?;
    approve(&world, &token(&world), vec![id]).await?;
    // A key of the same person is not the grant that asked.
    let key = a_person_key(&world, &token(&world), KeyChoice::SeeAndChange).await?;
    let by_key = collect_agent_keys(&world.ctx(), collect(&key, id)).await;
    assert!(refused(&error_of(by_key), ErrorCode::NotFound));
    let collected = collect_agent_keys(&world.ctx(), collect(&grant, id))
        .await?
        .into_value();
    assert_eq!(collected.stands, RequestStands::Collected);
    assert_eq!(collected.keys.len(), 2);
    Ok(())
}
