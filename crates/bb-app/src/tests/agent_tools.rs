//! The tools of agents (AI-36, ADR-0029, decision 9): *take*, *report*, *ask*, *attach*, *ready*,
//! *give up*, *changes since* and *propose*, called as the agent behind a key. They are tools of the
//! one registry, so each is judged by the use case of agent work and answers a preview that saves
//! nothing.

use bb_core::ErrorCode;
use bb_core::agent_work::AgentWorkState;
use bb_core::ai::{AuthorKind, Tool, ToolCaller, ToolEffect, ToolScope};
use bb_core::id::IssueId;
use serde_json::{Value, json};

use super::agent_work::{Handed, handed, plain_hand_over, query};
use super::agents::{an_agent, key_of, refused};
use super::ai_tools::without_nonces;
use super::world::{Outcome, World};
use crate::agent_work::{AgentWorkQuery, hand_over, issue_agent_work};
use crate::ai::context::{PeopleMode, fenced_lines, only_inside_a_fence};
use crate::ai::tools::{
    ToolAnswer, ToolCallQuery, ToolListQuery, ToolMode, call_tool, list_tools, registry,
};
use crate::auth::AppError;
use crate::issues::issue;

use super::fake::FakeStore;

/// A call of a tool as the agent behind `key`.
pub(super) fn as_agent(key: &str, tool: &str, input: Value, mode: ToolMode) -> ToolCallQuery {
    ToolCallQuery {
        session_token: key.to_owned(),
        caller: ToolCaller::Agent,
        app: None,
        granted: ToolScope::SeeAndChange,
        people: PeopleMode::AsSeen,
        tool: tool.to_owned(),
        input,
        mode,
    }
}

/// What a tool answers when the agent calls it for real (a tool that changes something saves).
pub(super) async fn used(
    world: &World,
    key: &str,
    tool: &str,
    input: Value,
) -> Result<ToolAnswer, AppError> {
    Ok(
        call_tool(&world.ctx(), as_agent(key, tool, input, ToolMode::Run))
            .await?
            .into_value(),
    )
}

/// What a tool answers when the agent only asks what it would do.
pub(super) async fn previewed(
    world: &World,
    key: &str,
    tool: &str,
    input: Value,
) -> Result<ToolAnswer, AppError> {
    Ok(
        call_tool(&world.ctx(), as_agent(key, tool, input, ToolMode::Preview))
            .await?
            .into_value(),
    )
}

/// The state of the work of the issue, as the person who answers for it reads it.
async fn work_state(world: &World, given: &Handed) -> Result<AgentWorkState, AppError> {
    Ok(issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value()
    .work[0]
        .state)
}

/// Everything an agent could have changed, to compare before and after a preview: the events, the
/// work, the attempts, the questions and the results.
fn stored(world: &World) -> String {
    let shared = world.store.lock();
    format!(
        "{} {:?} {:?} {:?} {:?} {:?}",
        shared.events.len(),
        shared.state.work.works,
        shared.state.work.attempts,
        shared.state.work.questions,
        shared.state.work.results,
        shared.state.work.proposals
    )
}

/// A second agent of the same person, handed the issue the first one was handed: its key.
async fn a_helper_with_the_same_issue(world: &World, first: &Handed) -> Result<String, AppError> {
    let session = first.team.session_token.clone();
    let helper = an_agent(world, &session, "Helper").await?;
    let key = key_of(world, &session, helper).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&session, helper, &[first.issue]),
    )
    .await?;
    Ok(key)
}

#[tokio::test]
async fn ai_36_two_agents_take_the_same_issue_and_only_one_gets_it() -> Outcome {
    let world = World::new().await?;
    let first = handed(&world).await?;
    let helper_key = a_helper_with_the_same_issue(&world, &first).await?;
    let input = json!({"issue": first.issue.to_string()});
    let got = used(&world, &first.key, "take", input.clone()).await?;
    assert!(got.text.contains("agent-working"), "{}", got.text);
    // The other agent finds the issue taken: nothing of its work changes.
    let before = stored(&world);
    let refusal = used(&world, &helper_key, "take", input.clone()).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::AgentWorkNotReady));
    assert_eq!(before, stored(&world));
    // When the first one lets go of it, the other may take it.
    used(
        &world,
        &first.key,
        "give_up",
        json!({"issue": first.issue.to_string(), "reason": "Too big"}),
    )
    .await?;
    used(&world, &helper_key, "take", input).await?;
    Ok(())
}

#[tokio::test]
async fn ai_36_two_agents_who_take_at_the_same_moment_are_judged_one_after_the_other() -> Outcome {
    let world = World::new().await?;
    let first = handed(&world).await?;
    let helper_key = a_helper_with_the_same_issue(&world, &first).await?;
    let input = json!({"issue": first.issue.to_string()});
    let (one, other) = tokio::join!(
        used(&world, &first.key, "take", input.clone()),
        used(&world, &helper_key, "take", input),
    );
    assert_eq!(
        usize::from(one.is_ok()) + usize::from(other.is_ok()),
        1,
        "exactly one agent gets the issue"
    );
    let in_attempt = world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.state.in_attempt())
        .count();
    assert_eq!(in_attempt, 1);
    Ok(())
}

#[tokio::test]
async fn ai_36_a_take_with_an_old_version_of_the_issue_takes_nothing() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let before = stored(&world);
    let stale = used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string(), "version": 99}),
    )
    .await;
    assert!(
        matches!(stale, Err(AppError::Conflict(_))),
        "an issue that is not at the version the agent read is told what changed"
    );
    assert_eq!(before, stored(&world));
    // With the version it read, it takes the issue.
    let version = issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string(), "version": version}),
    )
    .await?;
    assert_eq!(
        work_state(&world, &given).await?,
        AgentWorkState::AgentWorking
    );
    Ok(())
}

/// For each tool of agents that works on a piece of work: the calls that bring the work to the state
/// the tool works in, and the call whose preview is asked.
fn preview_case(tool: &str, issue: IssueId) -> Option<(Vec<(&'static str, Value)>, Value)> {
    let issue = issue.to_string();
    let take = ("take", json!({"issue": issue}));
    Some(match tool {
        "take" => (Vec::new(), json!({"issue": issue})),
        "report" => (
            vec![take],
            json!({"issue": issue, "plan": "Fix the button", "minutes": 5, "cost_cents": 20}),
        ),
        "ask" => (
            vec![take],
            json!({"issue": issue, "question": "Which browser?"}),
        ),
        "attach" => (
            vec![take],
            json!({"issue": issue, "kind": "link", "label": "Branch", "address": "https://example.test/pull/1"}),
        ),
        "submit" => (vec![take], json!({"issue": issue})),
        "give_up" => (
            vec![take],
            json!({"issue": issue, "reason": "Cannot reproduce it"}),
        ),
        _ => return None,
    })
}

/// The preview of the tool says what the call changes and saves nothing, and the call then changes
/// what the preview said.
async fn the_preview_saves_nothing_and_the_call_does_what_it_said(tool: &str) -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let (first, input) = preview_case(tool, given.issue).ok_or("a tool without a case")?;
    for (earlier, earlier_input) in first {
        used(&world, &given.key, earlier, earlier_input).await?;
    }
    let before = stored(&world);
    let preview = previewed(&world, &given.key, tool, input.clone()).await?;
    assert!(
        preview.text.contains("would_change"),
        "{tool}: {}",
        preview.text
    );
    assert!(!preview.changes.is_empty(), "{tool}");
    assert_eq!(before, stored(&world), "{tool} saved something");
    let saved = used(&world, &given.key, tool, input).await?;
    assert_eq!(preview.changes, saved.changes, "{tool}");
    assert_ne!(before, stored(&world), "{tool} saved nothing");
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_take_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("take").await
}

#[tokio::test]
async fn ai_21_preview_report_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("report").await
}

#[tokio::test]
async fn ai_21_preview_ask_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("ask").await
}

#[tokio::test]
async fn ai_21_preview_attach_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("attach").await
}

#[tokio::test]
async fn ai_21_preview_submit_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("submit").await
}

#[tokio::test]
async fn ai_21_preview_give_up_answers_what_would_change_and_saves_nothing() -> Outcome {
    the_preview_saves_nothing_and_the_call_does_what_it_said("give_up").await
}

#[tokio::test]
async fn ai_36_every_tool_that_changes_answers_a_preview_and_saves_nothing() -> Outcome {
    let issue = IssueId::generate(&mut bb_core::id::SequentialIds::default())
        .map_err(|error| error.to_string())?;
    for spec in registry::<FakeStore>() {
        if spec.effect() != ToolEffect::Changes || !spec.tool.is_for_agents() {
            continue;
        }
        // `propose` works on a standing rule and is asked about in its own test.
        if spec.tool == Tool::Propose {
            continue;
        }
        assert!(
            preview_case(spec.name(), issue).is_some(),
            "{} has no case",
            spec.name()
        );
        the_preview_saves_nothing_and_the_call_does_what_it_said(spec.name()).await?;
    }
    Ok(())
}

#[tokio::test]
async fn ai_36_give_up_sets_agent_gave_up_with_the_reason() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string()}),
    )
    .await?;
    let gave = used(
        &world,
        &given.key,
        "give_up",
        json!({"issue": given.issue.to_string(), "reason": "The test needs a server I cannot reach"}),
    )
    .await?;
    assert!(gave.text.contains("agent-gave-up"), "{}", gave.text);
    assert_eq!(
        work_state(&world, &given).await?,
        AgentWorkState::AgentGaveUp
    );
    let stored_reason = world.store.lock().state.work.works[0].gave_up.clone();
    assert_eq!(
        stored_reason.as_deref(),
        Some("The test needs a server I cannot reach")
    );
    // A reason is needed: the call without one saves nothing.
    let again = used(
        &world,
        &given.key,
        "give_up",
        json!({"issue": given.issue.to_string(), "reason": ""}),
    )
    .await;
    assert!(again.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_36_the_tools_of_agents_are_offered_to_agents_and_to_nobody_else() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let names = |listed: crate::ai::tools::ToolList| -> Vec<String> {
        listed
            .tools
            .into_iter()
            .map(|tool| tool.name.to_owned())
            .collect()
    };
    let list = |token: String| ToolListQuery {
        session_token: token,
        granted: ToolScope::SeeAndChange,
    };
    let agent = names(
        list_tools(&world.ctx(), list(given.key.clone()))
            .await?
            .into_value(),
    );
    let person = names(
        list_tools(&world.ctx(), list(given.team.session_token.clone()))
            .await?
            .into_value(),
    );
    for tool in registry::<FakeStore>().iter().map(|spec| spec.tool) {
        assert_eq!(
            agent.contains(&tool.code().to_owned()),
            !tool.is_for_people(),
            "{} for the agent",
            tool.code()
        );
        assert_eq!(
            person.contains(&tool.code().to_owned()),
            !tool.is_for_agents(),
            "{} for the person",
            tool.code()
        );
    }
    // A person who calls a tool of agents is turned away, and so is an agent that calls a tool
    // about a person's own time.
    let turned_away = call_tool(
        &world.ctx(),
        ToolCallQuery {
            caller: ToolCaller::AiApp,
            ..as_agent(
                &given.team.session_token,
                "take",
                json!({"issue": given.issue.to_string()}),
                ToolMode::Run,
            )
        },
    )
    .await;
    assert!(refused(&turned_away.unwrap_err(), ErrorCode::Forbidden));
    let time = used(
        &world,
        &given.key,
        "start_timer",
        json!({"issue": given.issue.to_string()}),
    )
    .await;
    assert!(refused(&time.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

/// The nearest line above the first line that holds `needle` that opens a fence, if the line is
/// inside one.
pub(super) fn fence_above(answer: &str, needle: &str) -> Option<String> {
    let lines = fenced_lines(answer);
    let at = lines.iter().position(|(line, _)| line.contains(needle))?;
    lines[..at]
        .iter()
        .rev()
        .find(|(line, inside)| !inside && line.contains("text written by "))
        .map(|(line, _)| (*line).to_owned())
}

#[tokio::test]
async fn ai_36_every_text_in_a_tool_answer_names_its_author() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let phrase = |kind: AuthorKind| format!("text written by {}", kind.phrase());
    let taken = used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string()}),
    )
    .await?;
    let text = without_nonces(&taken.text);
    // The note of the brief was written by the person who handed the issue over.
    let note = "Please look at the login button";
    assert!(only_inside_a_fence(&text, note), "{text}");
    assert!(
        fence_above(&text, note).is_some_and(|line| line.contains(&phrase(AuthorKind::Member))),
        "{text}"
    );
    // What the agent writes comes back as the word of an agent.
    let agent_words = [
        (
            "report",
            json!({"issue": given.issue.to_string(), "plan": "Replace the handler", "minutes": 3, "cost_cents": 1}),
            "Replace the handler",
        ),
        (
            "attach",
            json!({"issue": given.issue.to_string(), "kind": "check", "label": "Unit tests green", "outcome": "passed"}),
            "Unit tests green",
        ),
        (
            "ask",
            json!({"issue": given.issue.to_string(), "question": "Which browser is it?"}),
            "Which browser is it?",
        ),
    ];
    for (tool, input, needle) in agent_words {
        let answer = without_nonces(&used(&world, &given.key, tool, input).await?.text);
        assert!(only_inside_a_fence(&answer, needle), "{tool}: {answer}");
        assert!(
            fence_above(&answer, needle)
                .is_some_and(|line| line.contains(&phrase(AuthorKind::Agent))),
            "{tool}: {answer}"
        );
    }
    // The answer of a person comes back as theirs, in the brief.
    let work = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value()
    .work
    .remove(0);
    crate::agent_work::answer_question(
        &world.ctx(),
        crate::agent_work::AnswerQuestion {
            session_token: given.team.session_token.clone(),
            work: work.id,
            question: work.questions[0].id,
            reply: "Firefox, on a phone".to_owned(),
            version: work.version,
        },
    )
    .await?;
    let after = used(
        &world,
        &given.key,
        "report",
        json!({"issue": given.issue.to_string(), "minutes": 4, "cost_cents": 2}),
    )
    .await?;
    let text = without_nonces(&after.text);
    assert!(only_inside_a_fence(&text, "Firefox, on a phone"), "{text}");
    assert!(
        fence_above(&text, "Firefox, on a phone")
            .is_some_and(|line| line.contains(&phrase(AuthorKind::Member))),
        "{text}"
    );
    Ok(())
}

#[tokio::test]
async fn ai_36_the_tools_of_agents_are_declared_to_read_or_to_change() {
    for spec in registry::<FakeStore>() {
        if spec.tool.is_for_agents() {
            let expected = if spec.tool == Tool::ChangesSince {
                ToolEffect::Reads
            } else {
                ToolEffect::Changes
            };
            assert_eq!(spec.effect(), expected, "{}", spec.name());
            assert!(!spec.input_schema().is_null(), "{}", spec.name());
        }
    }
}
