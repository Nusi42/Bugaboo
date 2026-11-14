//! What a tool takes from an AI app and what it answers for it (AI-04, AI-21): a limit is a number
//! in the range of a page, or the call is told the range; a search that cannot be read is told its
//! mistake, the same to an app and to the assistant; a stopped timer says whether time was booked;
//! and a tool is offered only to a person who may use it somewhere outside what is kept from AI.

use bb_core::ErrorCode;
use bb_core::ai::ToolScope;
use bb_policy::StartingRole;
use serde_json::json;

use super::ai_tools::{call, issue_with, mark};
use super::assistant::{ai_on, ask, question, tool_results, will_call, will_say};
use super::scenarios::{a_project, token};
use super::scenarios_searches::with_role;
use super::world::{Outcome, World};
use crate::ai::NeverSentTarget;
use crate::ai::tools::{ToolListQuery, ToolMode, ToolSpec, call_tool, list_tools, search_refusal};
use crate::auth::AppError;

#[tokio::test]
async fn ai_21_a_limit_outside_one_to_a_hundred_is_refused_with_the_range() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    issue_with(
        &world,
        "WEB",
        "Brakes",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    issue_with(
        &world,
        "WEB",
        "Lights",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    let who = token(&world);
    let tools = [
        ("list_projects", json!({})),
        ("get_my_work", json!({})),
        ("search_issues", json!({"query": "title contains \"a\""})),
    ];
    for (tool, input) in tools {
        for limit in [json!(0), json!(-1), json!(101), json!(2.5)] {
            let mut asked = input.clone();
            asked["limit"] = json!(limit);
            let refused = call_tool(&world.ctx(), call(&who, tool, asked)).await;
            assert!(
                matches!(refused, Err(AppError::Refused(ErrorCode::LimitOutOfRange))),
                "{tool} {limit}: {refused:?}"
            );
        }
        for limit in [1, 100] {
            let mut asked = input.clone();
            asked["limit"] = json!(limit);
            call_tool(&world.ctx(), call(&who, tool, asked)).await?;
        }
    }
    // The refusal says the range.
    let said = ErrorCode::LimitOutOfRange.text().render();
    assert!(said.contains('1') && said.contains("100"), "{said}");
    Ok(())
}

/// The tools that read, which every role that sees a project is offered.
const READS: [&str; 4] = ["search_issues", "get_issue", "list_projects", "get_my_work"];

#[tokio::test]
async fn ai_04_a_tool_is_offered_only_to_a_person_who_may_use_it_somewhere() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(
        &world,
        "WEB",
        "Brakes",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    let kind = super::scenarios_issues::a_type(&world).await?.to_string();
    // What each tool that changes something is asked, as its preview, which the use case judges.
    let asked = |tool: &str| match tool {
        "create_issue" => json!({"project": "WEB", "issue_type": kind, "title": "New"}),
        "update_issue" => json!({"issue": key, "version": 1, "title": "New"}),
        "change_status" => json!({"issue": key, "version": 1, "status": "done"}),
        "add_comment" => json!({"issue": key, "text": "A note"}),
        "log_time" => json!({"issue": key, "date": "2027-01-14", "minutes": 30}),
        "start_timer" => json!({"issue": key}),
        _ => json!({"version": 0}),
    };
    // The tools a person may be offered at all: every one but the tools of agents.
    let people_tools: Vec<&str> = crate::ai::tools::registry::<super::fake::FakeStore>()
        .iter()
        .filter(|spec| !spec.tool.is_for_agents())
        .map(ToolSpec::name)
        .collect();
    let changes_own = [
        "create_issue",
        "update_issue",
        "change_status",
        "add_comment",
    ];
    for (role, offered) in [
        (StartingRole::Administrator, people_tools.clone()),
        (StartingRole::ProjectLead, people_tools.clone()),
        (StartingRole::TeamMember, people_tools.clone()),
        (StartingRole::Contributor, people_tools.clone()),
        (StartingRole::Viewer, READS.to_vec()),
        (StartingRole::Guest, [&READS[..], &changes_own[..]].concat()),
        (StartingRole::Hr, Vec::new()),
    ] {
        let person = world
            .person(&format!("{}@example.test", role.code()))
            .await?;
        with_role(&world, &person, role, None).await?;
        let listed = list_tools(
            &world.ctx(),
            ToolListQuery {
                session_token: person.session_token.clone(),
                granted: ToolScope::SeeAndChange,
                caller: bb_core::ai::ToolCaller::AiApp,
            },
        )
        .await?
        .into_value();
        let mut names: Vec<&str> = listed.tools.iter().map(|tool| tool.name).collect();
        let mut expected = offered.clone();
        names.sort_unstable();
        expected.sort_unstable();
        assert_eq!(names, expected, "{role:?}");
        // A tool that changes something and is not offered is still refused by its use case when
        // it is called by its name.
        let left_out = people_tools
            .iter()
            .filter(|tool| !offered.contains(tool) && !READS.contains(tool));
        for tool in left_out {
            let mut query = call(&person.session_token, tool, asked(tool));
            query.mode = ToolMode::Preview;
            let refused = call_tool(&world.ctx(), query).await;
            assert!(
                matches!(refused, Err(AppError::Refused(_))),
                "{role:?} {tool}: {refused:?}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn ai_21_a_mistyped_search_is_told_its_mistake_and_plain_words_still_search() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let who = token(&world);
    for typo in ["title includes x", "status open"] {
        let refused = call_tool(
            &world.ctx(),
            call(&who, "search_issues", json!({ "query": typo })),
        )
        .await;
        let Err(AppError::Mistakes(mistakes)) = refused else {
            return Err(format!("{typo}: {refused:?}").into());
        };
        let said = search_refusal(&mistakes);
        // The real mistake, then how a search is written.
        assert!(said.contains("title contains \"brakes\""), "{said}");
        assert!(!said.starts_with("Write"), "{typo}: {said}");
        // The assistant reads the same sentence.
        will_call(&world, "search_issues", json!({ "query": typo }));
        will_say(&world, "That search did not work.");
        ask(&world, question(&who, "Find it")).await?;
        assert!(tool_results(&world).contains(&said), "{typo}");
    }
    let words = call_tool(
        &world.ctx(),
        call(&who, "search_issues", json!({"query": "brakes"})),
    )
    .await?
    .into_value();
    assert!(words.text.contains("contains \"brakes\""), "{}", words.text);
    Ok(())
}

#[tokio::test]
async fn ai_21_a_timer_stopped_within_a_minute_says_that_nothing_was_booked() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let key = issue_with(
        &world,
        "WEB",
        "Brakes",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    let who = token(&world);
    let mut start = call(&who, "start_timer", json!({ "issue": key }));
    start.mode = ToolMode::Apply;
    let started = call_tool(&world.ctx(), start).await?.into_value();
    let version = started.undo.ok_or("no way back of the timer")?.version;
    let mut stop = call(&who, "stop_timer", json!({ "version": version }));
    stop.mode = ToolMode::Apply;
    let stopped = call_tool(&world.ctx(), stop).await?.into_value();
    assert_eq!(
        stopped.text.lines().last(),
        Some("The timer stopped. It ran for less than a minute, so no time was booked."),
        "{}",
        stopped.text
    );
    Ok(())
}

#[tokio::test]
async fn ai_04_a_tool_allowed_only_in_projects_kept_from_ai_is_not_offered() -> Outcome {
    let world = World::new().await?;
    let web = a_project(&world, "WEB").await?;
    // Tom works in WEB only; a role for the whole organization would count in every project.
    let tom = world.person("tom@example.test").await?;
    with_role(&world, &tom, StartingRole::TeamMember, Some(web)).await?;
    let list = async || -> Result<usize, AppError> {
        Ok(list_tools(
            &world.ctx(),
            ToolListQuery {
                session_token: tom.session_token.clone(),
                granted: ToolScope::SeeAndChange,
                caller: bb_core::ai::ToolCaller::AiApp,
            },
        )
        .await?
        .into_value()
        .tools
        .len())
    };
    assert!(list().await? > 0);
    mark(&world, NeverSentTarget::Project("WEB".to_owned())).await?;
    assert_eq!(list().await?, 0);
    Ok(())
}
