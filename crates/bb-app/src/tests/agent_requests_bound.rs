//! Requests for agents are bound to the credential that asked (AI-30, ADR-0029 amendment M9): no
//! request while agents are off, a key or grant that no longer works neither gets its request
//! approved nor collects, an approval stays within the most of one list and names the request of a
//! refused row, and the person sees where an app lives beside the name it gave itself.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::agent::MOST_AGENTS_AT_ONCE;
use bb_core::agent_request::RequestStands;
use bb_core::id::AgentRequestId;
use serde_json::json;

use super::agent_handoff::agents_off;
use super::agent_requests::{
    Asking, a_person_key, agent_count, approve, ask, asking, collect, error_of, program,
};
use super::agents::{an_agent, refused};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::agents::{AgentRequestsQuery, agent_requests, ask_for_agents, collect_agent_keys};
use crate::auth::{
    AddWebAddress, AppError, DeleteAccessKey, RegisterApp, RegisterOutcome, TakeBack,
    add_web_address, delete_access_key, register_app, take_back,
};

/// Whether `result` was refused with `code` at `field`.
fn refused_at<T>(result: &Result<T, AppError>, code: ErrorCode, field: &str) -> bool {
    matches!(result, Err(AppError::RefusedAtField { code: found, field: at })
        if *found == code && at == field)
}

/// The ID of the access key `secret` stands for, among the person's keys.
fn key_id(world: &World, secret: &str) -> Result<bb_core::id::AccessKeyId, String> {
    let prefix = bb_auth::AccessKeySecret::parse(secret)
        .ok_or("not a key")?
        .prefix()
        .to_owned();
    world
        .store
        .lock()
        .state
        .access_keys
        .iter()
        .find(|record| record.key.prefix == prefix)
        .map(|record| record.key.id)
        .ok_or_else(|| "no such key".to_owned())
}

async fn take_back_key(given: &Asking, secret: &str) -> Outcome {
    delete_access_key(
        &given.world.ctx(),
        DeleteAccessKey {
            session_token: given.session.clone(),
            id: key_id(&given.world, secret)?,
            agent: None,
        },
    )
    .await?;
    Ok(())
}

/// The agent keys that exist.
fn agent_keys(world: &World) -> usize {
    world
        .store
        .lock()
        .state
        .access_keys
        .iter()
        .filter(|record| record.key.agent.is_some())
        .count()
}

#[tokio::test]
async fn ai_30_no_request_while_agents_are_off() -> Outcome {
    let given = asking().await?;
    agents_off(&given.world).await?;
    let direct = ask_for_agents(
        &given.world.ctx(),
        crate::agents::AskForAgents {
            session_token: given.key.clone(),
            reason: "A team".to_owned(),
            agents: vec![crate::agents::AgentAsked {
                name: "Planner".to_owned(),
                purpose: "plans".to_owned(),
                key_days: None,
            }],
        },
    )
    .await;
    assert!(refused(&error_of(direct), ErrorCode::AiOff));
    assert!(
        program(
            &given.world,
            &given.key,
            "ask_for_agents",
            super::agent_requests::team_of_two()
        )
        .await
        .is_err()
    );
    assert!(given.world.store.lock().state.agent_requests.is_empty());
    Ok(())
}

/// How a key or grant can stop working after its request was approved.
#[derive(Clone, Copy, Debug)]
enum Gone {
    /// The person takes the key back.
    TakenBack,
    /// The person takes the key back and makes a new one for the same program.
    Renewed,
    /// The key runs out.
    RanOut,
}

#[tokio::test]
async fn ai_30_a_key_taken_back_renewed_or_run_out_never_collects() -> Outcome {
    for gone in [Gone::TakenBack, Gone::Renewed, Gone::RanOut] {
        let given = asking().await?;
        let id = ask(&given.world, &given.key).await?;
        approve(&given.world, &given.session, vec![id]).await?;
        let replacement = match gone {
            Gone::TakenBack => {
                take_back_key(&given, &given.key).await?;
                None
            }
            Gone::Renewed => {
                take_back_key(&given, &given.key).await?;
                Some(a_person_key(&given.world, &given.session, KeyChoice::SeeAndChange).await?)
            }
            Gone::RanOut => {
                let at = given.world.ctx().now();
                for record in &mut given.world.store.lock().state.access_keys {
                    record.key.expires_at = at;
                }
                None
            }
        };
        let old = collect_agent_keys(&given.world.ctx(), collect(&given.key, id)).await;
        assert!(old.is_err(), "{gone:?}: the old key collects nothing");
        if let Some(new) = replacement {
            let by_new = collect_agent_keys(&given.world.ctx(), collect(&new, id)).await;
            assert!(
                refused(&error_of(by_new), ErrorCode::NotFound),
                "{gone:?}: the new key did not ask"
            );
        }
        assert_eq!(agent_keys(&given.world), 0, "{gone:?}: no key is stored");
    }
    Ok(())
}

#[tokio::test]
async fn ai_30_a_grant_taken_back_never_collects() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    a_project(&world, "WEB").await?;
    let (_, tokens) = super::oauth::signed_in(&world, KeyChoice::SeeAndChange).await?;
    let grant = tokens.access_token.to_string();
    let id = ask(&world, &grant).await?;
    approve(&world, &token(&world), vec![id]).await?;
    let held = world.store.lock().state.oauth.grants[0].grant.id;
    take_back(
        &world.ctx(),
        TakeBack {
            session_token: token(&world),
            id: held,
        },
    )
    .await?;
    let collected = collect_agent_keys(&world.ctx(), collect(&grant, id)).await;
    assert!(collected.is_err());
    assert_eq!(agent_keys(&world), 0);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_request_whose_key_no_longer_works_is_not_approved() -> Outcome {
    let given = asking().await?;
    let id = ask(&given.world, &given.key).await?;
    take_back_key(&given, &given.key).await?;
    let refusal = approve(&given.world, &given.session, vec![id]).await;
    assert!(refused_at(
        &refusal,
        ErrorCode::InvalidRequest,
        &format!("requests/{id}")
    ));
    assert_eq!(agent_count(&given.world, &given.session).await?, 0);
    Ok(())
}

/// Asks for `count` agents named after `stem` with `key`.
async fn ask_for(
    given: &Asking,
    key: &str,
    stem: &str,
    count: usize,
) -> Result<AgentRequestId, AppError> {
    let agents: Vec<_> = (0..count)
        .map(|row| json!({"name": format!("{stem} {row}"), "purpose": "works"}))
        .collect();
    program(
        &given.world,
        key,
        "ask_for_agents",
        json!({"reason": "A big team", "agents": agents}),
    )
    .await?;
    given
        .world
        .store
        .lock()
        .state
        .agent_requests
        .last()
        .map(|request| request.id)
        .ok_or(AppError::Internal("no request".to_owned()))
}

#[tokio::test]
async fn ai_30_an_approval_stays_within_the_most_at_once_and_names_the_request_of_a_row() -> Outcome
{
    let given = asking().await?;
    let world = &given.world;
    let other = a_person_key(world, &given.session, KeyChoice::SeeAndChange).await?;
    let first = ask_for(&given, &given.key, "First", MOST_AGENTS_AT_ONCE).await?;
    let second = ask_for(&given, &other, "Second", 1).await?;
    // Each request alone fits; together they are one too many for one list.
    let both = approve(world, &given.session, vec![first, second]).await;
    assert!(refused(&error_of(both), ErrorCode::InvalidRequest));
    assert_eq!(agent_count(world, &given.session).await?, 0);
    // A name the person used already is named in its request.
    an_agent(world, &given.session, "Second 0").await?;
    let taken = approve(world, &given.session, vec![second]).await;
    assert!(refused_at(
        &taken,
        ErrorCode::NameTaken,
        &format!("requests/{second}/agents/0/name")
    ));
    Ok(())
}

#[tokio::test]
async fn ai_30_the_person_sees_where_the_app_that_asked_lives() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    a_project(&world, "WEB").await?;
    let web = "https://app.example.com/callback";
    add_web_address(
        &world.ctx(),
        AddWebAddress {
            session_token: token(&world),
            address: web.to_owned(),
        },
    )
    .await?;
    let RegisterOutcome::Registered(app) = register_app(
        &world.ctx(),
        RegisterApp {
            name: "Helper".to_owned(),
            return_addresses: vec![super::oauth::LOOPBACK.to_owned(), web.to_owned()],
        },
    )
    .await?
    else {
        return Err("the app was not registered".into());
    };
    let code = super::oauth::allowed(
        &world,
        app.client_id,
        "see-and-change",
        KeyChoice::SeeAndChange,
    )
    .await?;
    let tokens = super::oauth::issued(
        &world,
        super::oauth::swap_code(app.client_id, &code, super::oauth::VERIFIER),
    )
    .await?;
    ask(&world, &tokens.access_token).await?;
    // A key of the person shows no address.
    let key = a_person_key(&world, &token(&world), KeyChoice::SeeAndChange).await?;
    ask(&world, &key).await?;
    let listed = agent_requests(
        &world.ctx(),
        AgentRequestsQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value()
    .items;
    let by_app = listed
        .iter()
        .find(|item| item.by_app)
        .ok_or("the app's request")?;
    assert_eq!(by_app.asked_by, "Helper");
    assert_eq!(by_app.web_address.as_deref(), Some(web));
    assert_eq!(by_app.stands, RequestStands::Waiting);
    let by_key = listed
        .iter()
        .find(|item| !item.by_app)
        .ok_or("the key's request")?;
    assert_eq!(by_key.web_address, None);
    Ok(())
}
