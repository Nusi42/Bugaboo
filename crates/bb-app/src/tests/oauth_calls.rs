//! What an AI app does with its token or key once it is signed in (AI-23, AI-38, ADR-0042): its
//! session is one run, its changes say which app made them, and an access key works as the person
//! narrowed by it.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::ai::ToolScope;
use serde_json::json;

use super::fake::FakeStore;
use super::oauth::{ai_on, app_call, signed_in};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::tools::{ToolListQuery, call_tool, list_tools};
use crate::auth::{AppError, Ctx, RequestInfo};

/// The context of a request that comes in the run of a session of an app (`RequestInfo::run`).
fn in_run(world: &World, run: Option<bb_core::id::RunId>) -> Ctx<'_, FakeStore> {
    world.services.ctx(RequestInfo {
        run,
        ..RequestInfo::default()
    })
}

#[tokio::test]
async fn ai_38_an_mcp_session_is_one_run_and_its_changes_say_which_app() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    a_project(&world, "WEB").await?;
    let kind = super::scenarios_issues::a_type(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let session = crate::auth::start_app_session(
        &world.ctx(),
        crate::auth::StartAppSession {
            token: tokens.access_token.to_string(),
        },
    )
    .await?;
    let run = session.run.ok_or("a session of an app has a run")?;
    for title in ["First", "Second"] {
        let _ = call_tool(
            &in_run(&world, Some(run)),
            app_call(
                &tokens.access_token,
                "create_issue",
                json!({"project": "WEB", "issue_type": kind.to_string(), "title": title}),
            ),
        )
        .await?
        .into_value();
    }
    let made: Vec<_> = world
        .store
        .lock()
        .events
        .iter()
        .filter(|event| event.run == Some(run))
        .cloned()
        .collect();
    let issues = made
        .iter()
        .filter(|event| event.entity.kind.as_str() == "issue")
        .count();
    assert_eq!(issues, 2, "both changes carry the run of the session");
    assert!(
        made.iter()
            .all(|event| event.channel == bb_core::event::Channel::Mcp),
        "an AI app is the channel mcp"
    );
    // The run is the person's, names the app and is open until the session ends.
    let seen = crate::runs::get_run(
        &world.ctx(),
        crate::runs::RunQuery {
            session_token: token(&world),
            run,
        },
    )
    .await?
    .into_value();
    assert_eq!(seen.app.as_deref(), Some("Claude Code"));
    assert!(!seen.status.is_over());
    crate::auth::end_app_runs(
        &world.ctx(),
        crate::auth::EndAppRuns {
            runs: vec![run],
            how: bb_core::run::RunEnd::Done,
        },
    )
    .await?;
    let ended = crate::runs::get_run(
        &world.ctx(),
        crate::runs::RunQuery {
            session_token: token(&world),
            run,
        },
    )
    .await?
    .into_value();
    assert!(ended.status.is_over());
    Ok(())
}

/// A key of the administrator: its secret.
async fn a_key(world: &World, choice: KeyChoice) -> Result<String, AppError> {
    Ok(crate::auth::create_access_key(
        &world.ctx(),
        crate::auth::CreateAccessKey {
            session_token: token(world),
            password: super::world::PASSWORD.to_owned(),
            name: "For an app".to_owned(),
            choice,
            permissions: Vec::new(),
            projects: Vec::new(),
            expires_at: None,
            agent: None,
            renews: None,
        },
    )
    .await?
    .into_value()
    .secret)
}

#[tokio::test]
async fn ai_23_an_access_key_works_as_the_person_narrowed_by_it() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    a_project(&world, "WEB").await?;
    let kind = super::scenarios_issues::a_type(&world).await?;
    let see = a_key(&world, KeyChoice::SeeOnly).await?;
    let change = a_key(&world, KeyChoice::SeeAndChange).await?;
    let names = |tools: &crate::ai::tools::ToolList| -> Vec<&'static str> {
        tools.tools.iter().map(|tool| tool.name).collect()
    };
    let looking = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: see.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?
    .into_value();
    let changing = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: change.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?
    .into_value();
    assert!(names(&looking).contains(&"search_issues"));
    assert!(!names(&looking).contains(&"create_issue"));
    assert!(names(&changing).contains(&"create_issue"));
    let input = json!({"project": "WEB", "issue_type": kind.to_string(), "title": "By a key"});
    let refused_call = call_tool(&world.ctx(), app_call(&see, "create_issue", input.clone())).await;
    assert_eq!(
        refused_call.err(),
        Some(AppError::Refused(ErrorCode::Forbidden))
    );
    call_tool(&world.ctx(), app_call(&change, "create_issue", input)).await?;
    // A key is no app: the history names the person, with no app.
    let id = world
        .store
        .lock()
        .events
        .iter()
        .find(|event| event.channel == bb_core::event::Channel::Mcp)
        .map(|event| event.actor);
    assert!(matches!(id, Some(bb_core::event::Actor::Person(_))));
    Ok(())
}

#[tokio::test]
async fn ai_23_a_change_through_mcp_says_person_via_app_in_the_history() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    a_project(&world, "WEB").await?;
    let kind = super::scenarios_issues::a_type(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let run = crate::auth::start_app_session(
        &world.ctx(),
        crate::auth::StartAppSession {
            token: tokens.access_token.to_string(),
        },
    )
    .await?
    .run;
    call_tool(
        &in_run(&world, run),
        app_call(
            &tokens.access_token,
            "create_issue",
            json!({"project": "WEB", "issue_type": kind.to_string(), "title": "By an app"}),
        ),
    )
    .await?;
    let issue = world
        .store
        .lock()
        .events
        .iter()
        .find(|event| event.run == run && event.entity.kind.as_str() == "issue")
        .map(|event| event.entity.id.to_string())
        .ok_or("no issue event")?;
    let shown = crate::history::history(
        &world.ctx(),
        crate::history::HistoryQuery {
            session_token: token(&world),
            issue: issue.clone(),
            tab: bb_core::history::HistoryTab::Everything,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    let made = shown.items.first().ok_or("no history")?;
    assert_eq!(made.app.as_deref(), Some("Claude Code"));
    assert!(made.agent.is_none());
    // What the person does in the browser has no app, a moment later, so the newest is the first.
    world.clock.advance(1);
    crate::issues::change_issue(
        &world.ctx(),
        crate::issues::ChangeIssue {
            session_token: token(&world),
            issue: issue.clone(),
            version: 1,
            issue_type: None,
            title: Some("By the person".to_owned()),
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await?;
    let shown = crate::history::history(
        &world.ctx(),
        crate::history::HistoryQuery {
            session_token: token(&world),
            issue: issue.clone(),
            tab: bb_core::history::HistoryTab::Everything,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    assert!(shown.items.first().ok_or("no history")?.app.is_none());
    assert_eq!(
        shown.items.iter().filter(|item| item.app.is_some()).count(),
        1
    );
    Ok(())
}
