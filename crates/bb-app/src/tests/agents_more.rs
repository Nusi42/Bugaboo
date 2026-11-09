//! Agents (AI-30, ADR-0029), the rest: the switches and maximums of the settings, the most agents of
//! a person, a person who is gone, renewing keys, the own data of a person and what an agent is
//! accepted with.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::settings::SettingValue;
use bb_policy::StartingRole;

use super::scenarios::{a_project, token};
use super::scenarios_issues::a_type;
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::agents::{AcceptAgent, AgentQuery, HandAgent, accept_agent, agent, hand_agent};
use crate::auth::{AppError, create_access_key};
use crate::history::{HistoryQuery, HistoryWhat, history};
use crate::issues::{ChangeIssue, CreateIssue, IssueQuery, change_issue, create_issue, issue};
use crate::projects::{CreateProject, create_project};
use crate::projects::{ProjectsQuery, projects};
use crate::settings::{ChangeSetting, SettingsQuery, SettingsScope, change_setting, settings};
use bb_core::history::HistoryTab;

use super::agents::{a_team_agent, an_agent, give_work, key_of, key_request, list, refused};

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

#[tokio::test]
async fn ai_30_work_on_the_parent_does_not_reach_a_child_where_agents_are_off() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (web, child) = web_with_api(&world).await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    give_work(&world, fixer, web);
    let inside = |key: &str| ProjectsQuery {
        inside: Some(web),
        ..list(key)
    };
    assert_eq!(
        projects(&world.ctx(), inside(&key))
            .await?
            .into_value()
            .items
            .len(),
        1
    );
    // Off for the child only: the work names the parent, the child stays out.
    set(
        &world,
        SettingsScope::Project(child),
        "agents.enabled",
        SettingValue::Bool(false),
    )
    .await?;
    let seen = projects(&world.ctx(), inside(&key)).await;
    assert!(seen.is_err() || seen?.into_value().items.is_empty());
    // Off for the parent and on again for the child: the nearest value decides.
    set(
        &world,
        SettingsScope::Project(child),
        "agents.enabled",
        SettingValue::Bool(true),
    )
    .await?;
    set(
        &world,
        SettingsScope::Project(web),
        "agents.enabled",
        SettingValue::Bool(false),
    )
    .await?;
    let off_above = projects(&world.ctx(), list(&key)).await?.into_value();
    let names: Vec<&str> = off_above
        .items
        .iter()
        .map(|i| i.short_name.as_str())
        .collect();
    assert!(!names.contains(&"WEB"), "the parent is off: {names:?}");
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_never_reaches_the_own_data_of_its_person() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "WEB").await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    let read = settings(
        &world.ctx(),
        SettingsQuery {
            session_token: key.clone(),
            scope: SettingsScope::Person,
            topic: None,
            project: None,
        },
    )
    .await;
    assert!(refused(&read.unwrap_err(), ErrorCode::Forbidden));
    let write = change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: key.clone(),
            scope: SettingsScope::Person,
            key: "appearance.mode".to_owned(),
            value: SettingValue::Text("dark".to_owned()),
        },
    )
    .await;
    assert!(refused(&write.unwrap_err(), ErrorCode::Forbidden));
    // The person themselves still read and change theirs.
    assert!(
        settings(
            &world.ctx(),
            SettingsQuery {
                session_token: admin,
                scope: SettingsScope::Person,
                topic: None,
                project: None,
            },
        )
        .await
        .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn ai_30_an_agent_is_not_accepted_by_a_person_who_has_the_most_agents() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "WEB").await?;
    let team = member(
        &world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    for number in 0..bb_core::agent::MOST_AGENTS {
        an_agent(&world, &team.session_token, &format!("Agent {number}")).await?;
    }
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    hand_agent(
        &world.ctx(),
        HandAgent {
            session_token: admin.clone(),
            id: fixer,
            to: team.person.id,
            version: 1,
        },
    )
    .await?;
    let accepted = accept_agent(
        &world.ctx(),
        AcceptAgent {
            session_token: team.session_token.clone(),
            id: fixer,
        },
    )
    .await;
    assert!(accepted.is_err(), "the person has the most agents already");
    let still = agent(
        &world.ctx(),
        AgentQuery {
            session_token: admin,
            id: fixer,
        },
    )
    .await?
    .into_value();
    assert!(still.offered, "the agent stays with its person");
    Ok(())
}

/// The project WEB and API inside it.
async fn web_with_api(
    world: &World,
) -> Result<(ProjectId, ProjectId), Box<dyn std::error::Error + Send + Sync>> {
    let web = a_project(world, "WEB").await?;
    let api = create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(world),
            parent: Some(web),
            short_name: "API".to_owned(),
            name: "API".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    Ok((web, api))
}

/// WEB with the sub-project API; the agent of the administrator has work on WEB. The names of the
/// projects the agent sees inside WEB, after the switches in `switches` (project, on or off).
async fn seen_inside_web(
    switches: &[(&str, bool)],
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let world = World::new().await?;
    let admin = token(&world);
    let (web, api) = web_with_api(&world).await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    give_work(&world, fixer, web);
    for (name, on) in switches {
        let project = if *name == "WEB" { web } else { api };
        set(
            &world,
            SettingsScope::Project(project),
            "agents.enabled",
            SettingValue::Bool(*on),
        )
        .await?;
    }
    // Searched by name, since a project whose parent is off cannot be listed through it.
    let query = ProjectsQuery {
        search: Some("API".to_owned()),
        ..list(&key)
    };
    Ok(match projects(&world.ctx(), query).await {
        Ok(seen) => seen
            .into_value()
            .items
            .into_iter()
            .map(|item| item.short_name.to_string())
            .collect(),
        // A project that the agent may not reach is not found, like one that does not exist.
        Err(AppError::Refused(ErrorCode::NotFound)) => Vec::new(),
        Err(other) => return Err(other.into()),
    })
}

#[tokio::test]
async fn ai_30_the_nearest_switch_decides_when_the_parent_is_on_and_the_child_off() -> Outcome {
    // WEB on, API off, work on WEB: API is not reached.
    let seen = seen_inside_web(&[("WEB", true), ("API", false)]).await?;
    assert!(seen.is_empty(), "{seen:?}");
    Ok(())
}

#[tokio::test]
async fn ai_30_the_nearest_switch_decides_when_the_parent_is_off_and_the_child_on() -> Outcome {
    // WEB off, API on, work on WEB: the child is listed, the nearest switch is its own.
    let seen = seen_inside_web(&[("WEB", false), ("API", true)]).await?;
    assert_eq!(seen, ["API"]);
    Ok(())
}

/// An issue the person made, for the tests of what a reader who made it is told.
async fn an_issue_of(
    world: &World,
    project: ProjectId,
    session: &str,
) -> Result<bb_core::id::IssueId, Box<dyn std::error::Error + Send + Sync>> {
    Ok(create_issue(
        &world.ctx(),
        CreateIssue {
            session_token: session.to_owned(),
            project: project.to_string(),
            issue_type: a_type(world).await?,
            title: "Nothing happens".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
            estimated_time: None,
            fields: BTreeMap::new(),
            parent: None,
        },
    )
    .await?
    .into_value()
    .id)
}

#[tokio::test]
async fn ai_04_a_reader_who_sees_the_person_only_as_a_role_is_not_told_the_agent() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "WEB").await?;
    // A guest sees people by their role only, and sees the issues they reported.
    let guest = member(&world, "guest@example.test", StartingRole::Guest, project).await?;
    let issue_id = an_issue_of(&world, project, &guest.session_token).await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    super::agents::hand_title_and_take(&world, &admin, fixer, &key, issue_id).await?;
    let version = issue(
        &world.ctx(),
        IssueQuery {
            session_token: key.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .version;
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: key,
            issue: issue_id.to_string(),
            version,
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
    let asked = |session: &str| HistoryQuery {
        session_token: session.to_owned(),
        issue: issue_id.to_string(),
        tab: HistoryTab::Changes,
        cursor: None,
        limit: 20,
    };
    // The row of the agent's change is the one that changed the title (the creation set it).
    let title_change = |page: &crate::history::HistoryPage| {
        page.items
            .iter()
            .find(|item| match &item.what {
                HistoryWhat::Changes { fields } => fields
                    .iter()
                    .any(|field| field.field == "title" && field.before.is_some()),
                _ => false,
            })
            .cloned()
            .expect("the history has the row of the change of the title")
    };
    // The administrator is told the agent; the guest sees the person as a role, and no agent.
    let named = title_change(&history(&world.ctx(), asked(&admin)).await?.into_value());
    assert_eq!(
        named.agent.as_ref().map(|agent| agent.name.as_str()),
        Some("Fixer")
    );
    let by_role = title_change(
        &history(&world.ctx(), asked(&guest.session_token))
            .await?
            .into_value(),
    );
    assert!(by_role.agent.is_none());
    assert!(matches!(
        by_role.by,
        Some(bb_policy::PersonRef::Role { .. })
    ));
    Ok(())
}

/// The mark of a text that an agent wrote names the agent only to a reader who is told who the
/// person behind it is, and reads each agent once for a request (ADR-0035).
#[tokio::test]
async fn ai_04_the_mark_of_a_text_names_the_agent_only_to_a_reader_told_its_person() -> Outcome {
    use crate::comments::{AddComment, add_comment};
    use bb_core::mark::WrittenBy;

    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "WEB").await?;
    let guest = member(&world, "guest@example.test", StartingRole::Guest, project).await?;
    let issue_id = an_issue_of(&world, project, &guest.session_token).await?;
    let fixer = an_agent(&world, &admin, "Fixer").await?;
    let key = key_of(&world, &admin, fixer).await?;
    super::agents::hand_title_and_take(&world, &admin, fixer, &key, issue_id).await?;
    for text in ["First", "Second"] {
        add_comment(
            &world.ctx(),
            AddComment {
                session_token: key.clone(),
                issue: issue_id.to_string(),
                text: text.to_owned(),
            },
        )
        .await?;
    }
    let marks = |session: &str| {
        let session = session.to_owned();
        let world = &world;
        async move {
            let page = history(
                &world.ctx(),
                HistoryQuery {
                    session_token: session,
                    issue: issue_id.to_string(),
                    tab: HistoryTab::Comments,
                    cursor: None,
                    limit: 20,
                },
            )
            .await?
            .into_value();
            Ok::<_, AppError>(
                page.items
                    .iter()
                    .filter_map(|row| match &row.what {
                        HistoryWhat::Comment { comment } => comment.written_by.clone(),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            )
        }
    };
    let seen_by_guest = marks(&guest.session_token).await?;
    assert_eq!(seen_by_guest.len(), 2);
    assert!(
        seen_by_guest
            .iter()
            .all(|mark| mark.kind == WrittenBy::Agent)
    );
    assert!(seen_by_guest.iter().all(|mark| mark.agent.is_none()));
    let seen_by_admin = marks(&admin).await?;
    assert!(
        seen_by_admin
            .iter()
            .all(|mark| mark.agent.as_deref() == Some("Fixer"))
    );
    Ok(())
}
