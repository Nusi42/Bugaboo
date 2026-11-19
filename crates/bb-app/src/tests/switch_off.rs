//! *Switch off* and *Switch on* (PRIV-04, ADR-0044, decision 5), run against the in-memory store:
//! every way in ends at once, a switched-off person is offered nowhere and told nothing, *Switch
//! on* gives back the sign-in only, and every switch-off and taking back writes a line of the
//! replay list.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::agent::AgentState;
use bb_core::ai::{ToolCaller, ToolScope};
use bb_core::backup::replay::{self, ReplayKind};
use bb_core::id::UserId;
use bb_policy::{PersonRef, StartingRole};

use super::agents::{a_team_agent, person_key_request};
use super::notifications::{Change, Crew};
use super::oauth::{LOOPBACK, VERIFIER, ai_on, allowed_by, issued, registered, swap_code};
use super::scenarios::{Added, a_project, measured, text, token};
use super::workflows_rules::member;
use super::world::{Outcome, PASSWORD, World};
use crate::agents::{AgentQuery, agent};
use crate::ai::tools::{ToolListQuery, list_tools};
use crate::auth::{AppError, SignIn, create_access_key, sign_in};
use crate::comments::{MentionChoicesQuery, mention_choices};
use crate::issues::{AssignablePeopleQuery, assignable_people};
use crate::people::{SwitchOff, SwitchOn, switch_off, switch_on};

/// The administrator switches `person` off.
pub(super) async fn switched_off(world: &World, person: UserId) -> Result<(), AppError> {
    switch_off(
        &world.ctx(),
        SwitchOff {
            session_token: token(world),
            person,
        },
    )
    .await
    .map(|_| ())
}

async fn switched_on(world: &World, person: UserId) -> Result<(), AppError> {
    switch_on(
        &world.ctx(),
        SwitchOn {
            session_token: token(world),
            person,
        },
    )
    .await
    .map(|_| ())
}

pub(crate) async fn switch_off_scenario(world: &World) -> Added {
    let bob = world.person("bob@example.test").await.map_err(text)?;
    measured(&world.store, switched_off(world, bob.person.id)).await
}

pub(crate) async fn switch_on_scenario(world: &World) -> Added {
    let bob = world.person("bob@example.test").await.map_err(text)?;
    switched_off(world, bob.person.id).await.map_err(text)?;
    measured(&world.store, switched_on(world, bob.person.id)).await
}

/// Whether the secret (a session, an access key or the token of an AI app) is refused as a way in.
async fn refused_way_in(world: &World, secret: &str) -> bool {
    let answer = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: secret.to_owned(),
            granted: ToolScope::SeeAndChange,
            caller: ToolCaller::AiApp,
        },
    )
    .await;
    answer.err().and_then(|error| error.code()) == Some(ErrorCode::NotSignedIn)
}

async fn signs_in(world: &World, email: &str) -> Result<bool, AppError> {
    match sign_in(
        &world.ctx(),
        SignIn {
            email: email.to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(AppError::Refused(ErrorCode::WrongCredentials)) => Ok(false),
        Err(error) => Err(error),
    }
}

/// The lines of the replay list of the world, by kind and ID.
fn replay_lines(world: &World) -> Vec<(ReplayKind, String)> {
    replay::read(&world.replay.text())
        .0
        .into_iter()
        .map(|line| (line.kind, line.id))
        .collect()
}

/// A team member with a session, an agent with its key, a key of their own and an AI app they
/// allowed: every way in a person has.
struct WaysIn {
    person: UserId,
    session: String,
    agent: bb_core::id::AgentId,
    agent_key: String,
    own_key: String,
    app_token: String,
}

async fn every_way_in(world: &World) -> Result<WaysIn, Box<dyn std::error::Error + Send + Sync>> {
    ai_on(world).await?;
    let (project, team, agent, agent_key) = a_team_agent(world).await?;
    let own_key = create_access_key(
        &world.ctx(),
        person_key_request(
            team.session_token.clone(),
            "Mine",
            KeyChoice::SeeAndChange,
            vec![project],
        ),
    )
    .await?
    .into_value()
    .secret;
    let client = registered(world, LOOPBACK).await?;
    let code = allowed_by(
        world,
        &team.session_token,
        client,
        "see-and-change",
        KeyChoice::SeeAndChange,
    )
    .await?;
    let tokens = issued(world, swap_code(client, &code, VERIFIER)).await?;
    Ok(WaysIn {
        person: team.person.id,
        session: team.session_token,
        agent,
        agent_key,
        own_key,
        app_token: tokens.access_token.to_string(),
    })
}

async fn agent_state(world: &World, ways: &WaysIn) -> Result<AgentState, AppError> {
    Ok(agent(
        &world.ctx(),
        AgentQuery {
            session_token: token(world),
            id: ways.agent,
        },
    )
    .await?
    .into_value()
    .state)
}

#[tokio::test]
async fn priv_04_switch_off_ends_sessions_keys_grants_and_agents_at_once() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let ways = every_way_in(&world).await?;
    for secret in [
        &ways.session,
        &ways.agent_key,
        &ways.own_key,
        &ways.app_token,
    ] {
        assert!(!refused_way_in(&world, secret).await, "works before");
    }
    switched_off(&world, ways.person).await?;
    for secret in [
        &ways.session,
        &ways.agent_key,
        &ways.own_key,
        &ways.app_token,
    ] {
        assert!(refused_way_in(&world, secret).await, "refused after");
    }
    assert_eq!(agent_state(&world, &ways).await?, AgentState::Stopped);
    assert!(!signs_in(&world, "team@example.test").await?);
    let lines = replay_lines(&world);
    assert!(lines.contains(&(ReplayKind::SwitchOff, ways.person.to_string())));
    let count = |kind| lines.iter().filter(|(found, _)| *found == kind).count();
    assert_eq!(count(ReplayKind::AccessKey), 2, "{lines:?}");
    assert_eq!(count(ReplayKind::AiAppGrant), 1, "{lines:?}");
    // A person is switched off once, and nobody switches themselves off.
    assert_eq!(
        switched_off(&world, ways.person).await.err(),
        Some(AppError::Refused(ErrorCode::VersionChanged))
    );
    assert_eq!(
        switched_off(&world, world.admin.person.id).await.err(),
        Some(AppError::Refused(ErrorCode::InvalidRequest))
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_switch_on_gives_back_the_sign_in_only() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let ways = every_way_in(&world).await?;
    switched_off(&world, ways.person).await?;
    switched_on(&world, ways.person).await?;
    assert!(signs_in(&world, "team@example.test").await?);
    // What the switch-off took back stays taken back.
    for secret in [
        &ways.session,
        &ways.agent_key,
        &ways.own_key,
        &ways.app_token,
    ] {
        assert!(refused_way_in(&world, secret).await);
    }
    assert_eq!(agent_state(&world, &ways).await?, AgentState::Stopped);
    // Only a switched-off person is switched on.
    assert_eq!(
        switched_on(&world, ways.person).await.err(),
        Some(AppError::Refused(ErrorCode::VersionChanged))
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_a_switched_off_person_is_offered_nowhere_and_told_nothing() -> Outcome {
    let mut crew = Crew::new().await?;
    crew.follow(&crew.carol).await?;
    let carol = crew.carol.person.id;
    let project = crew.project.to_string();
    assert!(offered(&crew.world, &project, carol).await?);
    assert!(mentioned(&crew).await?);
    // While she is active, a comment on the issue she follows tells her.
    crew.comment(&crew.bob.session_token, "Carol, have a look")
        .await?;
    super::data_exports::run_jobs(&crew.world).await?;
    assert!(told(&crew.world, carol) > 0);
    switched_off(&crew.world, carol).await?;
    assert!(!offered(&crew.world, &project, carol).await?);
    assert!(
        !mentioned(&crew).await?,
        "a switched-off person is no mention choice"
    );
    let bob = crew.bob.session_token.clone();
    assert!(
        crew.change(&bob, Change::Assign(vec![carol]))
            .await
            .is_err(),
        "a switched-off person is assigned nowhere"
    );
    let before = told(&crew.world, carol);
    crew.comment(&crew.bob.session_token, "Carol would want to know")
        .await?;
    super::data_exports::run_jobs(&crew.world).await?;
    assert_eq!(
        told(&crew.world, carol),
        before,
        "a switched-off person is told nothing"
    );
    Ok(())
}

/// Whether a comment of the administrator on the issue of the crew offers Carol to mention.
async fn mentioned(crew: &Crew) -> Result<bool, AppError> {
    Ok(mention_choices(
        &crew.world.ctx(),
        MentionChoicesQuery {
            session_token: token(&crew.world),
            issue: crew.issue.to_string(),
            search: Some("carol".to_owned()),
            limit: 20,
        },
    )
    .await?
    .into_value()
    .items
    .iter()
    .any(|choice| choice.name == "Carol Crane"))
}

/// Whether *Assigned to* in `project` offers `person`.
async fn offered(world: &World, project: &str, person: UserId) -> Result<bool, AppError> {
    let query = AssignablePeopleQuery {
        session_token: token(world),
        project: project.to_owned(),
        search: None,
        cursor: None,
        limit: 50,
        field: None,
        issue: None,
        issue_type: None,
    };
    Ok(assignable_people(&world.ctx(), query)
        .await?
        .into_value()
        .items
        .into_iter()
        .any(|shown| matches!(shown, PersonRef::Name { id, .. } if id == person)))
}

/// How many notifications `person` has.
fn told(world: &World, person: UserId) -> usize {
    world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .filter(|notification| notification.recipient == person)
        .count()
}

#[tokio::test]
async fn priv_04_only_manage_people_switches_off() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    let team = member(
        &world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let bob = world.person("bob@example.test").await?;
    let refused = switch_off(
        &world.ctx(),
        SwitchOff {
            session_token: team.session_token.clone(),
            person: bob.person.id,
        },
    )
    .await;
    assert_eq!(refused.err(), Some(AppError::Refused(ErrorCode::Forbidden)));
    assert_eq!(
        switched_off(&world, UserId::from_random([9; 16]))
            .await
            .err(),
        Some(AppError::Refused(ErrorCode::NotFound))
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_a_switch_off_between_the_password_and_the_second_step_opens_no_session() -> Outcome
{
    use super::scenarios::sign_in::{ada_waits_for_the_second_step, ada_with_a_code_app, code_now};
    use super::scenarios_searches::with_role;
    use crate::auth::{SecondStep, second_step};
    let world = World::new().await?;
    let ben = world.person("ben@example.test").await?;
    with_role(&world, &ben, StartingRole::Administrator, None).await?;
    ada_with_a_code_app(&world).await?;
    let pending = ada_waits_for_the_second_step(&world).await?;
    switch_off(
        &world.ctx(),
        SwitchOff {
            session_token: ben.session_token.clone(),
            person: world.admin.person.id,
        },
    )
    .await?;
    world.clock.advance(30);
    let refused = second_step(
        &world.ctx(),
        SecondStep {
            session_token: pending.session_token,
            code: code_now(&world).await?,
        },
    )
    .await;
    assert!(refused.is_err(), "the second step opened a session");
    assert!(!signs_in(&world, "ada@example.test").await?);
    Ok(())
}

#[tokio::test]
async fn sec_08_the_last_administrator_cannot_be_switched_off() -> Outcome {
    use super::rights_once::{give, role};
    use crate::roles::Subject;
    let world = World::new().await?;
    let carl = world.person("carl@example.test").await?;
    let people = role(&world, "People", vec![bb_policy::Permission::ManagePeople]).await?;
    give(
        &world,
        Subject::Person(carl.person.id),
        people,
        None,
        bb_core::access::AppliesTo::Organization,
    )
    .await?;
    let refused = switch_off(
        &world.ctx(),
        SwitchOff {
            session_token: carl.session_token.clone(),
            person: world.admin.person.id,
        },
    )
    .await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::WouldLockOut))
    );
    assert!(signs_in(&world, "ada@example.test").await?);
    Ok(())
}

#[tokio::test]
async fn priv_04_a_line_the_list_does_not_take_waits_and_goes_with_the_next_write() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let cid = world.person("cid@example.test").await?.person.id;
    // A list with a line nobody can read is left as it is; the switch-off stands, its line waits.
    world.replay.replace("not a line\n");
    switched_off(&world, bob).await?;
    assert_eq!(world.replay.text(), "not a line\n");
    assert_eq!(world.store.lock().state.replay_waiting.len(), 1);
    // Once the list can be read again, the next change brings the line that waited along.
    world.replay.replace("");
    switched_off(&world, cid).await?;
    let lines = replay_lines(&world);
    for person in [bob, cid] {
        assert!(lines.contains(&(ReplayKind::SwitchOff, person.to_string())));
    }
    assert!(world.store.lock().state.replay_waiting.is_empty());
    Ok(())
}
