//! *Create several agents* (AI-30, ADR-0029 amendment M9): a list of agents, each with one key, made
//! in one change with the password once, all or nothing, by the same plan as one agent and one key.

use bb_core::ErrorCode;
use bb_core::agent::{MOST_AGENTS, MOST_AGENTS_AT_ONCE};
use bb_core::event::RecordKind;
use bb_core::id::{AgentId, EntityId};
use bb_core::sign_in::SecurityAction;
use bb_policy::StartingRole;

use super::agents::{an_agent, key_of, page, refused};
use super::scenarios::{Added, a_project, measured, token};
use super::workflows_rules::member;
use super::world::{Outcome, PASSWORD, World};
use crate::agents::{
    AgentQuery, CreateSeveralAgents, CreatedAgents, SeveralAgent, agent, create_several_agents,
    my_agents,
};
use crate::auth::AppError;

/// A row of the list: an agent with this name and the organization's default end of its key.
pub(super) fn row(name: &str) -> SeveralAgent {
    SeveralAgent {
        name: name.to_owned(),
        expires_at: None,
    }
}

/// The request that makes these agents, with the person's password.
pub(super) fn several(session: &str, rows: Vec<SeveralAgent>) -> CreateSeveralAgents {
    CreateSeveralAgents {
        session_token: session.to_owned(),
        password: PASSWORD.to_owned(),
        agents: rows,
    }
}

async fn create_list(
    world: &World,
    session: &str,
    names: &[&str],
) -> Result<CreatedAgents, AppError> {
    let rows = names.iter().map(|name| row(name)).collect();
    Ok(create_several_agents(&world.ctx(), several(session, rows))
        .await?
        .into_value())
}

/// A team member of a new project WEB, who may create agents there.
async fn a_team_member(world: &World) -> Result<String, AppError> {
    let project = a_project(world, "WEB").await.map_err(AppError::Internal)?;
    Ok(member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?
    .session_token)
}

/// The refusal of a request that should have been refused; a request that made something is
/// told apart as an internal error (the answer holds secrets, so it is not printed).
fn error_of<T>(result: Result<T, AppError>) -> AppError {
    match result {
        Ok(_) => AppError::Internal("it was made".to_owned()),
        Err(error) => error,
    }
}

async fn count_of_agents(world: &World, session: &str) -> Result<usize, AppError> {
    Ok(my_agents(&world.ctx(), page(session))
        .await?
        .into_value()
        .items
        .len())
}

fn keys_stored(world: &World) -> usize {
    world.store.lock().state.access_keys.len()
}

#[tokio::test]
async fn ai_30_several_agents_are_made_with_one_key_each_and_the_password_once() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    let made = create_list(&world, &team, &["Planner", "Builder", "Checker"]).await?;
    // The answer holds the agents in the order of the rows, each with its one key, shown once.
    let names: Vec<&str> = made
        .items
        .iter()
        .map(|item| item.agent.name.as_str())
        .collect();
    assert_eq!(names, ["Planner", "Builder", "Checker"]);
    let mut secrets: Vec<&str> = made
        .items
        .iter()
        .map(|item| item.key.secret.as_str())
        .collect();
    secrets.sort_unstable();
    secrets.dedup();
    assert_eq!(secrets.len(), 3, "every agent has a key of its own");
    for item in &made.items {
        assert_eq!(
            item.key.key.name, item.agent.name,
            "the key is called after its agent"
        );
        assert_eq!(item.agent.keys.len(), 1);
        // The key is the agent's: the agent reads its own record with it.
        let own = agent(
            &world.ctx(),
            AgentQuery {
                session_token: item.key.secret.clone(),
                id: item.agent.id,
            },
        )
        .await?
        .into_value();
        assert_eq!(own.id, item.agent.id);
    }
    assert_eq!(count_of_agents(&world, &team).await?, 3);
    Ok(())
}

#[tokio::test]
async fn ai_30_several_agents_are_all_or_nothing() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    an_agent(&world, &team, "Fixer").await?;
    let keys_before = keys_stored(&world);
    // A name twice in the list: the second row is named, and nothing is made.
    let twice = create_list(&world, &team, &["Planner", "Planner"]).await;
    assert!(
        matches!(&twice, Err(AppError::RefusedAtField { code: ErrorCode::NameTaken, field }) if field == "agents/1/name"),
        "{:?}",
        twice.as_ref().err()
    );
    // A name the person used already: that row is named.
    let taken = create_list(&world, &team, &["Planner", "Fixer"]).await;
    assert!(
        matches!(&taken, Err(AppError::RefusedAtField { code: ErrorCode::NameTaken, field }) if field == "agents/1/name"),
        "{:?}",
        taken.as_ref().err()
    );
    // A name that is no name, and an end that has passed.
    let empty = create_list(&world, &team, &["Planner", "  "]).await;
    assert!(
        matches!(&empty, Err(AppError::RefusedAtField { code: ErrorCode::InvalidRequest, field }) if field == "agents/1/name"),
        "{:?}",
        empty.as_ref().err()
    );
    let mut rows = vec![row("Planner"), row("Builder")];
    rows[1].expires_at = Some("2000-01-01T00:00:00Z".to_owned());
    let past = create_several_agents(&world.ctx(), several(&team, rows)).await;
    assert!(
        matches!(&past, Err(AppError::RefusedAtField { code: ErrorCode::InvalidRequest, field }) if field == "agents/1/expires_at"),
        "{:?}",
        past.as_ref().err()
    );
    // An empty list makes nothing either.
    assert!(refused(
        &error_of(create_list(&world, &team, &[]).await),
        ErrorCode::InvalidRequest
    ));
    assert_eq!(count_of_agents(&world, &team).await?, 1, "only Fixer");
    assert_eq!(keys_stored(&world), keys_before, "no key was made");
    Ok(())
}

#[tokio::test]
async fn ai_30_several_agents_stay_within_the_most_agents_and_the_most_at_once() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    // More than the most of one list.
    let names: Vec<String> = (0..=MOST_AGENTS_AT_ONCE)
        .map(|n| format!("Agent {n}"))
        .collect();
    let too_long: Vec<&str> = names.iter().map(String::as_str).collect();
    assert!(refused(
        &error_of(create_list(&world, &team, &too_long).await),
        ErrorCode::InvalidRequest
    ));
    // One short of the most agents a person has: a list of two would pass it.
    for n in 0..MOST_AGENTS - 1 {
        an_agent(&world, &team, &format!("Held {n}")).await?;
    }
    let past_most = create_list(&world, &team, &["One", "Two"]).await;
    assert!(refused(&error_of(past_most), ErrorCode::InvalidRequest));
    assert_eq!(count_of_agents(&world, &team).await?, MOST_AGENTS - 1);
    // A list of one still fits.
    create_list(&world, &team, &["One"]).await?;
    assert_eq!(count_of_agents(&world, &team).await?, MOST_AGENTS);
    Ok(())
}

#[tokio::test]
async fn ai_30_several_agents_need_the_password_and_a_session() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    let mut wrong = several(&team, vec![row("Planner")]);
    wrong.password = "not the password".to_owned();
    assert!(refused(
        &error_of(create_several_agents(&world.ctx(), wrong).await),
        ErrorCode::WrongCredentials
    ));
    // A key never makes agents or keys, whatever it is: a key of an agent, or of the person.
    let fixer = an_agent(&world, &team, "Fixer").await?;
    let agent_key = key_of(&world, &team, fixer).await?;
    let person_key = super::agents::person_key_request(
        team.clone(),
        "Script",
        bb_core::access_key::KeyChoice::SeeAndChange,
        Vec::new(),
    );
    let person_key = crate::auth::create_access_key(&world.ctx(), person_key)
        .await?
        .into_value()
        .secret;
    for key in [agent_key, person_key] {
        let by_key = create_several_agents(&world.ctx(), several(&key, vec![row("Planner")])).await;
        assert!(by_key.is_err(), "a key made agents");
    }
    // A Viewer may create agents nowhere.
    let project = a_project(&world, "DOCS").await?;
    let viewer = member(&world, "viewer@example.test", StartingRole::Viewer, project).await?;
    let by_viewer = create_list(&world, &viewer.session_token, &["Planner"]).await;
    assert!(refused(&error_of(by_viewer), ErrorCode::Forbidden));
    assert_eq!(count_of_agents(&world, &team).await?, 1, "only Fixer");
    Ok(())
}

#[tokio::test]
async fn ai_30_each_agent_and_key_of_several_has_its_event_and_its_security_log_entry() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    let made = create_list(&world, &team, &["Planner", "Builder"]).await?;
    let shared = world.store.lock();
    for item in &made.items {
        let of_agent = shared
            .events
            .iter()
            .filter(|event| {
                event.entity.kind == RecordKind::Agent.into()
                    && event.entity.id == EntityId::from(item.agent.id)
            })
            .count();
        assert_eq!(of_agent, 1, "one event makes the agent");
        let of_key = shared
            .events
            .iter()
            .filter(|event| {
                event.entity.kind == RecordKind::AccessKey.into()
                    && event.entity.id == EntityId::from(item.key.key.id)
            })
            .count();
        assert_eq!(of_key, 1, "one event makes the key");
        let logged = shared
            .log
            .iter()
            .filter(|entry| {
                entry.action == SecurityAction::AccessKeyCreated
                    && entry
                        .subject
                        .as_ref()
                        .is_some_and(|subject| subject.id == EntityId::from(item.key.key.id))
            })
            .count();
        assert_eq!(logged, 1, "each key has its entry in the security log");
    }
    Ok(())
}

#[tokio::test]
async fn ai_30_one_agent_and_one_key_are_made_by_the_same_plan_as_several() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let team = a_team_member(&world).await?;
    // A name made by the list is taken for one agent, and the other way round.
    let made = create_list(&world, &team, &["Planner"]).await?;
    assert!(refused(
        &an_agent(&world, &team, "Planner").await.unwrap_err(),
        ErrorCode::NameTaken
    ));
    an_agent(&world, &team, "Builder").await?;
    let again = create_list(&world, &team, &["Builder"]).await;
    assert!(
        matches!(
            &again,
            Err(AppError::RefusedAtField {
                code: ErrorCode::NameTaken,
                ..
            })
        ),
        "{:?}",
        again.as_ref().err()
    );
    // A key made alone for an agent of the list joins the key the list made.
    let planner: AgentId = made.items[0].agent.id;
    key_of(&world, &team, planner).await?;
    let read = agent(
        &world.ctx(),
        AgentQuery {
            session_token: team.clone(),
            id: planner,
        },
    )
    .await?
    .into_value();
    assert_eq!(read.keys.len(), 2);
    Ok(())
}

pub(crate) async fn create_several_agents_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    measured(
        &world.store,
        create_several_agents(
            &world.ctx(),
            several(&token(world), vec![row("Planner"), row("Builder")]),
        ),
    )
    .await
}
