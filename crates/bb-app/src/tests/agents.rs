//! Agents (AI-30, ADR-0029): an agent has one responsible person, never does more than that person,
//! holds no role, stops when its person loses access or an administrator stops it, and its key
//! always runs out.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::access_key::KeyChoice;
use bb_core::agent::AgentState;
use bb_core::event::Actor;
use bb_core::id::{AccessKeyId, AgentId, ProjectId, UserId};
use bb_core::settings::SettingValue;
use bb_policy::StartingRole;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{member, starting_role};
use super::world::{Outcome, PASSWORD, World};
use crate::agents::{
    AgentAction, AgentQuery, AgentWork, AgentsQuery, ChangeAgent, CreateAgent, agent, change_agent,
    create_agent, my_agents, organization_agents,
};
use crate::auth::{AppError, CreateAccessKey, create_access_key};
use crate::history::{HistoryQuery, history};
use crate::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use crate::projects::{ChangeProject, ProjectsQuery, change_project, projects};
use crate::roles::{AssignRole, Given, RemoveAssignment, Subject, assign_role, remove_assignment};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_core::history::HistoryTab;

pub(super) async fn an_agent(
    world: &World,
    session: &str,
    name: &str,
) -> Result<AgentId, AppError> {
    Ok(create_agent(
        &world.ctx(),
        CreateAgent {
            session_token: session.to_owned(),
            name: name.to_owned(),
        },
    )
    .await?
    .into_value()
    .id)
}

/// The request that makes a key of an agent, or renews one of its keys.
fn key_request(session: &str, agent: AgentId, renews: Option<AccessKeyId>) -> CreateAccessKey {
    CreateAccessKey {
        session_token: session.to_owned(),
        password: PASSWORD.to_owned(),
        name: "Fixer's key".to_owned(),
        choice: KeyChoice::SeeAndChange,
        permissions: Vec::new(),
        projects: Vec::new(),
        expires_at: None,
        agent: Some(agent),
        renews,
    }
}

/// The key of an agent: its secret, shown once.
pub(super) async fn key_of(
    world: &World,
    session: &str,
    agent: AgentId,
) -> Result<String, AppError> {
    Ok(
        create_access_key(&world.ctx(), key_request(session, agent, None))
            .await?
            .into_value()
            .secret,
    )
}

/// Gives the agent work in a project, as a hand-over will.
pub(super) fn give_work(world: &World, agent: AgentId, project: ProjectId) {
    let mut shared = world.store.lock();
    let work = &mut shared.state.agent_work;
    match work.iter_mut().find(|(id, _)| *id == agent) {
        Some((_, held)) => {
            held.projects.insert(project);
        }
        None => work.push((
            agent,
            AgentWork {
                projects: [project].into(),
                ..AgentWork::default()
            },
        )),
    }
}

fn list(session: &str) -> ProjectsQuery {
    ProjectsQuery {
        session_token: session.to_owned(),
        inside: None,
        search: None,
        outside: None,
        movable: None,
        cursor: None,
        limit: 100,
    }
}

/// A project, a team member of it, and an agent of theirs with its key and work in the project.
async fn a_team_agent(
    world: &World,
) -> Result<(ProjectId, crate::auth::Started, AgentId, String), AppError> {
    let project = a_project(world, "WEB").await.map_err(AppError::Internal)?;
    let team = member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let fixer = an_agent(world, &team.session_token, "Fixer").await?;
    let key = key_of(world, &team.session_token, fixer).await?;
    give_work(world, fixer, project);
    Ok((project, team, fixer, key))
}

fn refused(error: &AppError, code: ErrorCode) -> bool {
    matches!(error, AppError::Refused(found) if *found == code)
}

fn page(session: &str) -> AgentsQuery {
    AgentsQuery {
        session_token: session.to_owned(),
        cursor: None,
        limit: 50,
    }
}

#[tokio::test]
async fn ai_30_an_agent_needs_a_responsible_person() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    // A Viewer holds the permission to create agents nowhere, a Team member in the project.
    let viewer = member(&world, "viewer@example.test", StartingRole::Viewer, project).await?;
    let refusal = an_agent(&world, &viewer.session_token, "Fixer").await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    let team = member(
        &world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let made = an_agent(&world, &team.session_token, "Fixer").await?;
    let mine = my_agents(&world.ctx(), page(&team.session_token))
        .await?
        .into_value();
    assert_eq!(mine.items.len(), 1);
    assert_eq!(mine.items[0].id, made);
    assert_eq!(mine.items[0].state, AgentState::Active);
    // Nobody else sees it in their list, and a name is used once by one person.
    let others = my_agents(&world.ctx(), page(&token(&world)))
        .await?
        .into_value();
    assert!(others.items.is_empty());
    let again = an_agent(&world, &team.session_token, "Fixer").await;
    assert!(refused(&again.unwrap_err(), ErrorCode::NameTaken));
    Ok(())
}

#[tokio::test]
async fn ai_30_without_work_an_agent_reads_only_its_own_record() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let other = an_agent(&world, &admin, "Other").await?;
    let key = key_of(&world, &admin, fixer).await?;
    // The admin sees the project; the agent, with no work, sees none.
    let seen = projects(&world.ctx(), list(&admin)).await?.into_value();
    assert_eq!(seen.items.len(), 1);
    let seen = projects(&world.ctx(), list(&key)).await?.into_value();
    assert!(seen.items.is_empty());
    // Its own record it reads; another agent's it does not.
    let own = agent(
        &world.ctx(),
        AgentQuery {
            session_token: key.clone(),
            id: fixer,
        },
    )
    .await?;
    assert_eq!(own.into_value().name, "Fixer");
    let foreign = agent(
        &world.ctx(),
        AgentQuery {
            session_token: key.clone(),
            id: other,
        },
    )
    .await;
    assert!(refused(&foreign.unwrap_err(), ErrorCode::NotFound));
    // It cannot make agents: that starts at a session.
    assert!(an_agent(&world, &key, "Child").await.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_never_does_more_than_its_person() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let work = a_project(&world, "WORK").await?;
    let secret = a_project(&world, "SECRET").await?;
    // A Team member in WORK only.
    let team = member(&world, "team@example.test", StartingRole::TeamMember, work).await?;
    let fixer = an_agent(&world, &team.session_token, "Fixer").await?;
    let key = key_of(&world, &team.session_token, fixer).await?;
    // Work in both projects: the person's rights still cut it to WORK.
    give_work(&world, fixer, work);
    give_work(&world, fixer, secret);
    let seen = projects(&world.ctx(), list(&key)).await?.into_value();
    let names: Vec<&str> = seen
        .items
        .iter()
        .map(|item| item.short_name.as_str())
        .collect();
    assert_eq!(names, ["WORK"]);
    drop(admin);
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_never_holds_a_manage_permission() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let secret = a_project(&world, "SECRET").await?;
    let boss = an_agent(&world, &admin, "Boss").await?;
    let boss_key = key_of(&world, &admin, boss).await?;
    give_work(&world, boss, secret);
    // The administrator's agent sees the project, as its person does.
    let seen = projects(&world.ctx(), list(&boss_key)).await?.into_value();
    assert_eq!(seen.items.len(), 1);
    // But a change of the project's settings is a permission of kind Manage: refused.
    let rename = change_project(
        &world.ctx(),
        ChangeProject {
            session_token: boss_key,
            project: secret,
            version: 1,
            name: Some("Renamed".to_owned()),
            short_name: None,
            fields: BTreeMap::default(),
        },
    )
    .await;
    assert!(refused(&rename.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_stops_when_its_person_loses_access() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    let person = world.person("team@example.test").await?;
    let role = starting_role(&world, StartingRole::TeamMember).await?;
    let row = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(&world),
            subject: Subject::Person(person.person.id),
            given: Given::Role(role),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?
    .into_value()
    .id;
    let fixer = an_agent(&world, &person.session_token, "Fixer").await?;
    let key = key_of(&world, &person.session_token, fixer).await?;
    give_work(&world, fixer, project);
    let seen = projects(&world.ctx(), list(&key)).await?.into_value();
    assert_eq!(seen.items.len(), 1);
    // The administrator takes the person's role away: the agent's next request is refused.
    remove_assignment(
        &world.ctx(),
        RemoveAssignment {
            session_token: token(&world),
            assignment: row,
        },
    )
    .await?;
    let next = projects(&world.ctx(), list(&key)).await;
    assert!(refused(&next.unwrap_err(), ErrorCode::NotSignedIn));
    Ok(())
}

#[tokio::test]
async fn ai_30_an_administrator_stops_any_agent() -> Outcome {
    let world = World::new().await?;
    let (project, team, fixer, key) = a_team_agent(&world).await?;
    let other = member(
        &world,
        "other@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let stop = |session: &str| ChangeAgent {
        session_token: session.to_owned(),
        id: fixer,
        action: AgentAction::Stop,
        version: 1,
    };
    // Another team member cannot even see it; only an administrator lists every agent.
    let denied = change_agent(&world.ctx(), stop(&other.session_token)).await;
    assert!(refused(&denied.unwrap_err(), ErrorCode::NotFound));
    let listing = organization_agents(&world.ctx(), page(&team.session_token)).await;
    assert!(listing.is_err());
    let every = organization_agents(&world.ctx(), page(&token(&world)))
        .await?
        .into_value();
    assert_eq!(every.items.len(), 1);
    let logged = world.store.log_count();
    let stopped = change_agent(&world.ctx(), stop(&token(&world)))
        .await?
        .into_value();
    assert_eq!(stopped.state, AgentState::Stopped);
    assert!(
        world.store.log_count() > logged,
        "the stop is in the security log"
    );
    // Its next request is refused.
    let next = projects(&world.ctx(), list(&key)).await;
    assert!(refused(&next.unwrap_err(), ErrorCode::NotSignedIn));
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_key_always_runs_out_and_is_shown_once() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let too_long = create_access_key(
        &world.ctx(),
        CreateAccessKey {
            session_token: admin.clone(),
            password: PASSWORD.to_owned(),
            name: "Key".to_owned(),
            choice: KeyChoice::SeeAndChange,
            permissions: Vec::new(),
            projects: Vec::new(),
            expires_at: Some("2999-01-01T00:00:00Z".to_owned()),
            agent: Some(fixer),
            renews: None,
        },
    )
    .await;
    assert!(
        refused(&too_long.unwrap_err(), ErrorCode::InvalidRequest),
        "never past the longest time"
    );
    let key = key_of(&world, &admin, fixer).await?;
    let view = agent(
        &world.ctx(),
        AgentQuery {
            session_token: admin.clone(),
            id: fixer,
        },
    )
    .await?
    .into_value();
    assert_eq!(view.keys.len(), 1);
    assert!(view.keys[0].expires_at > world.ctx().now());
    // The secret is shown once, with the answer that makes the key; nothing shows it again.
    assert!(!format!("{view:?}").contains(&key));
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_has_no_role_and_cannot_be_assigned() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "WEB").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    // The ID of an agent names no person: a row of *Who has which role* for it is refused.
    let as_person = UserId::from_bytes(fixer.to_bytes())?;
    let row = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin.clone(),
            subject: Subject::Person(as_person),
            given: Given::NoAccess,
            project: Some(project),
            applies_to: AppliesTo::Project,
        },
    )
    .await;
    assert!(row.is_err());
    // And an issue cannot be assigned to it.
    let issue_id = an_issue(&world, project).await?;
    let shown = issue(
        &world.ctx(),
        IssueQuery {
            session_token: admin.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value();
    let assigned = change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: admin,
            issue: issue_id.to_string(),
            version: shown.issue.version,
            issue_type: None,
            title: None,
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: Some(vec![as_person]),
            fields: BTreeMap::default(),
        },
    )
    .await;
    assert!(assigned.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_04_an_event_of_an_agent_names_the_agent_and_its_person() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "WEB").await?;
    let issue_id = an_issue(&world, project).await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    give_work(&world, fixer, project);
    let shown = issue(
        &world.ctx(),
        IssueQuery {
            session_token: key.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value();
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: key,
            issue: issue_id.to_string(),
            version: shown.issue.version,
            issue_type: None,
            title: Some("Fixed by an agent".to_owned()),
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: BTreeMap::default(),
        },
    )
    .await?;
    let events = world.store.lock().events.clone();
    let last = events.last().expect("the change wrote an event");
    assert!(
        matches!(last.actor, Actor::Agent { agent, .. } if agent == fixer),
        "the event names the agent, not its person: {:?}",
        last.actor
    );
    assert!(last.actor.person().is_some());
    // The history says who did it: the agent by its name, and the person responsible for it.
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: admin,
            issue: issue_id.to_string(),
            tab: HistoryTab::Changes,
            cursor: None,
            limit: 20,
        },
    )
    .await?
    .into_value();
    let by_the_agent = page
        .items
        .iter()
        .find(|item| item.agent.is_some())
        .expect("a row of the history names the agent");
    assert_eq!(
        by_the_agent.agent.as_ref().map(|a| a.name.as_str()),
        Some("Fixer")
    );
    assert!(by_the_agent.by.is_some());
    Ok(())
}

// ---- Scenarios of the table of events: each says how many events its call appended ----

use super::scenarios::{Added, measured, text};
use crate::agents::{AcceptAgent, HandAgent, RenameAgent, StopAllAgents};
use crate::agents::{accept_agent, hand_agent, rename_agent, stop_all_agents};

pub(crate) async fn create_agent_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    measured(
        &world.store,
        create_agent(
            &world.ctx(),
            CreateAgent {
                session_token: token(world),
                name: "Fixer".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn rename_agent_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    let fixer = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    measured(
        &world.store,
        rename_agent(
            &world.ctx(),
            RenameAgent {
                session_token: token(world),
                id: fixer,
                name: "Finder".to_owned(),
                version: 1,
            },
        ),
    )
    .await
}

pub(crate) async fn change_agent_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    let fixer = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    measured(
        &world.store,
        change_agent(
            &world.ctx(),
            ChangeAgent {
                session_token: token(world),
                id: fixer,
                action: AgentAction::Pause,
                version: 1,
            },
        ),
    )
    .await
}

pub(crate) async fn stop_all_agents_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    measured(
        &world.store,
        stop_all_agents(
            &world.ctx(),
            StopAllAgents {
                session_token: token(world),
            },
        ),
    )
    .await
}

/// An agent of the administrator and a team member it can be handed to.
async fn to_hand(world: &World) -> Result<(AgentId, crate::auth::Started), String> {
    let project = a_project(world, "WEB").await?;
    let team = member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await
    .map_err(text)?;
    let fixer = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    Ok((fixer, team))
}

fn hand(world: &World, fixer: AgentId, to: &crate::auth::Started) -> HandAgent {
    HandAgent {
        session_token: token(world),
        id: fixer,
        to: to.person.id,
        version: 1,
    }
}

pub(crate) async fn hand_agent_scenario(world: &World) -> Added {
    let (fixer, team) = to_hand(world).await?;
    measured(
        &world.store,
        hand_agent(&world.ctx(), hand(world, fixer, &team)),
    )
    .await
}

pub(crate) async fn accept_agent_scenario(world: &World) -> Added {
    let (fixer, team) = to_hand(world).await?;
    hand_agent(&world.ctx(), hand(world, fixer, &team))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        accept_agent(
            &world.ctx(),
            AcceptAgent {
                session_token: team.session_token,
                id: fixer,
            },
        ),
    )
    .await
}

// ---- Switches, maximums, the most agents and a person who is gone ----

async fn set(world: &World, scope: SettingsScope, key: &str, value: SettingValue) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope,
            key: key.to_owned(),
            value,
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_30_agents_off_for_a_project_refuses_its_agents_there() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let web = a_project(&world, "WEB").await?;
    let api = a_project(&world, "API").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    give_work(&world, fixer, web);
    give_work(&world, fixer, api);
    let seen = projects(&world.ctx(), list(&key)).await?.into_value();
    assert_eq!(seen.items.len(), 2);
    // Off for one project: the agent works nowhere in it, and still in the other.
    set(
        &world,
        SettingsScope::Project(web),
        "agents.enabled",
        SettingValue::Bool(false),
    )
    .await?;
    let seen = projects(&world.ctx(), list(&key)).await?.into_value();
    let names: Vec<&str> = seen.items.iter().map(|i| i.short_name.as_str()).collect();
    assert_eq!(names, ["API"]);
    // Off for the organization: every agent is refused at its next request.
    set(
        &world,
        SettingsScope::Organization,
        "agents.enabled",
        SettingValue::Bool(false),
    )
    .await?;
    let next = projects(&world.ctx(), list(&key)).await;
    assert!(refused(&next.unwrap_err(), ErrorCode::NotSignedIn));
    Ok(())
}

#[tokio::test]
async fn ai_30_a_new_agent_starts_within_the_maximums_of_the_organization() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    set(
        &world,
        SettingsScope::Organization,
        "agents.maximum-issues-at-once",
        SettingValue::Integer(2),
    )
    .await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let view = agent(
        &world.ctx(),
        AgentQuery {
            session_token: admin,
            id: fixer,
        },
    )
    .await?
    .into_value();
    assert_eq!(view.limits.issues_at_once, 2);
    assert_eq!(view.limits.changes_a_minute, 60);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_person_has_at_most_fifty_agents() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    for number in 0..bb_core::agent::MOST_AGENTS {
        an_agent(&world, &admin, &format!("Agent {number}")).await?;
    }
    let one_more = an_agent(&world, &admin, "One more").await;
    assert!(refused(&one_more.unwrap_err(), ErrorCode::InvalidRequest));
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_of_a_person_who_is_gone_is_refused_at_once() -> Outcome {
    let world = World::new().await?;
    let (_, team, _, key) = a_team_agent(&world).await?;
    assert!(projects(&world.ctx(), list(&key)).await.is_ok());
    // The person is removed from the organization.
    world
        .store
        .lock()
        .state
        .people
        .retain(|(person, _)| person.id != team.person.id);
    let next = projects(&world.ctx(), list(&key)).await;
    assert!(refused(&next.unwrap_err(), ErrorCode::NotSignedIn));
    Ok(())
}

#[tokio::test]
async fn ai_30_renewing_an_agent_key_replaces_it_in_one_change() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let old = key_of(&world, &admin, fixer).await?;
    let before = agent(
        &world.ctx(),
        AgentQuery {
            session_token: admin.clone(),
            id: fixer,
        },
    )
    .await?
    .into_value();
    let old_id = before.keys[0].id;
    let events = world.store.event_count();
    let renewed = create_access_key(&world.ctx(), key_request(&admin, fixer, Some(old_id)))
        .await?
        .into_value();
    assert_eq!(world.store.event_count(), events + 1, "one change");
    assert_eq!(renewed.key.name, "Fixer's key");
    // The old key is gone, the new one works, and the agent has one key.
    assert!(projects(&world.ctx(), list(&old)).await.is_err());
    assert!(projects(&world.ctx(), list(&renewed.secret)).await.is_ok());
    let after = agent(
        &world.ctx(),
        AgentQuery {
            session_token: admin,
            id: fixer,
        },
    )
    .await?
    .into_value();
    assert_eq!(after.keys.len(), 1);
    Ok(())
}
