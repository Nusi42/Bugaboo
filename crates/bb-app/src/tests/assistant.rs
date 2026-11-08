//! The assistant on the server (AI-10, AI-05, COMP-08, COMP-10, PRIV-08), run against the in-memory
//! store and the fake provider: it answers from what the person may read, every change comes back as
//! a proposal and nothing is saved until *Apply*, *Apply* saves exactly what was shown as the person,
//! and the answer shows no outside picture and every link in full.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::ai::{
    ChatFrom, Destination, LookingAtKind, PROPOSAL_LIFETIME_SECONDS, QUESTION_CHARS_MAX,
    ROUNDS_MAX, TOOL_CALLS_MAX,
};
use bb_core::event::Channel;
use bb_core::fields::Field;
use serde_json::{Value, json};

use super::ai_tools::{issue_with, mark, read};
use super::scenarios::{a_project, token};
use super::scenarios_ai::{a_local_model, setup};
use super::search_fields::{bob_with_a_role, every_field};
use super::world::{Outcome, PASSWORD, World};
use crate::ai::assistant::{
    ApplyProposal, AskAssistantQuery, AssistantAnswer, ChatTurn, LookingAt, Proposal,
    apply_proposal, ask_assistant,
};
use crate::ai::{
    AiAnswer, AiPart, AiRole, AiToolCall, ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement,
    NeverSentTarget, SetAiKey, change_ai_setup, change_project_ai, confirm_ai_agreement,
    set_ai_key,
};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, change_issue};

/// A local model, with AI on.
pub(super) async fn ai_on(world: &World) -> Outcome {
    a_local_model(world).await?;
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(world)
        },
    )
    .await?;
    Ok(())
}

/// The model's next answer asks for a tool.
pub(super) fn will_call(world: &World, tool: &str, arguments: Value) {
    world.ai.will_answer(AiAnswer {
        tool_calls: vec![AiToolCall {
            id: format!("call-{tool}"),
            name: tool.to_owned(),
            arguments,
        }],
        ..AiAnswer::default()
    });
}

/// The model's next answer is this text.
pub(super) fn will_say(world: &World, text: &str) {
    world.ai.will_answer(AiAnswer {
        text: text.to_owned(),
        ..AiAnswer::default()
    });
}

pub(super) fn question(who: &str, text: &str) -> AskAssistantQuery {
    AskAssistantQuery {
        session_token: who.to_owned(),
        text: text.to_owned(),
        looking_at: Vec::new(),
        turns: Vec::new(),
    }
}

pub(super) async fn ask(
    world: &World,
    query: AskAssistantQuery,
) -> Result<AssistantAnswer, AppError> {
    Ok(ask_assistant(&world.ctx(), query).await?.into_value())
}

/// What the tools answered in the last call to the provider, as one text.
pub(super) fn tool_results(world: &World) -> String {
    let calls = world.ai.calls();
    let Some(last) = calls.last() else {
        return String::new();
    };
    last.request
        .turns
        .iter()
        .flat_map(|turn| turn.parts.iter())
        .filter_map(|part| match part {
            AiPart::ToolResult { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A project with an issue, AI on: the key of the issue.
async fn with_an_issue() -> Result<(World, String), String> {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await.map_err(|error| error.to_string())?;
    let key = issue_with(&world, "WEB", "Login", "plain text", BTreeMap::new()).await?;
    Ok((world, key))
}

async fn issue_text(world: &World, key: &str) -> Result<String, AppError> {
    read(world, &token(world), "get_issue", json!({"issue": key})).await
}

#[tokio::test]
async fn ai_10_the_assistant_answers_from_what_the_person_may_read() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "kept-description", BTreeMap::new()).await?;
    mark(&world, NeverSentTarget::Field(Field::Description)).await?;
    will_call(&world, "get_issue", json!({"issue": key}));
    will_say(&world, "It is about the login.");
    let answer = ask(&world, question(&token(&world), "What is WEB-1 about?")).await?;
    assert_eq!(answer.text, "It is about the login.");
    assert!(answer.proposals.is_empty());
    // The provider got two calls; the second holds what the tool read, without the field that is
    // kept from AI, and the first holds nothing of the issue at all.
    let calls = world.ai.calls();
    assert_eq!(calls.len(), 2);
    assert!(!format!("{:?}", calls[0].request).contains("Login"));
    let read = tool_results(&world);
    assert!(read.contains("Login"), "{read}");
    assert!(!read.contains("kept-description"), "{read}");
    assert!(
        calls[0]
            .request
            .tools
            .iter()
            .any(|tool| tool.name == "get_issue")
    );
    assert!(format!("{:?}", calls[0].request.turns).contains("What is WEB-1 about?"));
    Ok(())
}

#[tokio::test]
async fn ai_10_a_person_who_cannot_see_an_issue_gets_nothing_of_it_sent() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let key = issue_with(&world, "WEB", "Secret plans", "private", BTreeMap::new()).await?;
    // Bob has no role at all.
    let bob = world.person("bob@example.test").await?.session_token;
    will_call(&world, "get_issue", json!({"issue": key}));
    will_say(&world, "I cannot find it.");
    ask(&world, question(&bob, "Tell me about WEB-1")).await?;
    let sent = format!("{:?}", world.ai.calls());
    assert!(!sent.contains("Secret plans"), "{sent}");
    assert!(!sent.contains("private"), "{sent}");
    Ok(())
}

#[tokio::test]
async fn ai_10_what_the_panel_looks_at_is_checked_before_anything_is_sent() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let bob = world.person("bob@example.test").await?.session_token;
    let looking = |value: &str| AskAssistantQuery {
        looking_at: vec![LookingAt {
            kind: LookingAtKind::Issue,
            value: value.to_owned(),
        }],
        ..question(&bob, "Summarize this")
    };
    // An issue the person cannot see, and one that does not exist, answer the same.
    let hidden = ask(&world, looking(&key)).await.unwrap_err();
    let missing = ask(&world, looking("WEB-999")).await.unwrap_err();
    assert_eq!(hidden, AppError::Refused(ErrorCode::NotFound));
    assert_eq!(hidden, missing);
    assert!(world.ai.calls().is_empty());
    // A longer question than the limit is refused before anything is sent.
    let long = "x".repeat(QUESTION_CHARS_MAX + 1);
    let refused = ask(&world, question(&token(&world), &long))
        .await
        .unwrap_err();
    assert_eq!(refused, AppError::Refused(ErrorCode::InvalidRequest));
    assert!(world.ai.calls().is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_10_the_turns_so_far_come_from_the_browser_and_nothing_is_stored() -> Outcome {
    let (world, _) = with_an_issue().await?;
    let events = world.store.event_count();
    will_say(&world, "Done.");
    ask(
        &world,
        AskAssistantQuery {
            turns: vec![
                ChatTurn {
                    from: ChatFrom::Person,
                    text: "Earlier question".to_owned(),
                },
                ChatTurn {
                    from: ChatFrom::Assistant,
                    text: "Earlier answer".to_owned(),
                },
            ],
            ..question(&token(&world), "And now?")
        },
    )
    .await?;
    let sent = world.ai.calls()[0].request.turns.clone();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[0].role, AiRole::User);
    assert_eq!(sent[1].role, AiRole::Assistant);
    assert_eq!(
        world.store.event_count(),
        events,
        "a question saves nothing"
    );
    Ok(())
}

#[tokio::test]
async fn ai_10_one_question_runs_at_most_a_named_number_of_tools() -> Outcome {
    let (world, _) = with_an_issue().await?;
    // The model asks for two tools in every round and never answers.
    for _ in 0..ROUNDS_MAX + 2 {
        world.ai.will_answer(AiAnswer {
            tool_calls: (0..2)
                .map(|n| AiToolCall {
                    id: format!("c{n}"),
                    name: "list_projects".to_owned(),
                    arguments: json!({}),
                })
                .collect(),
            ..AiAnswer::default()
        });
    }
    let error = ask(&world, question(&token(&world), "Loop"))
        .await
        .unwrap_err();
    assert_eq!(error, AppError::Refused(ErrorCode::AiUnavailable));
    assert_eq!(world.ai.calls().len(), ROUNDS_MAX);
    // The last round is given no tools to ask for.
    assert!(world.ai.calls()[ROUNDS_MAX - 1].request.tools.is_empty());
    let rows = world.store.lock().tool_calls.len();
    assert_eq!(rows, TOOL_CALLS_MAX, "a call over the limit runs nothing");
    Ok(())
}

#[tokio::test]
async fn ai_05_a_change_comes_back_as_a_proposal_and_nothing_is_saved() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let events = world.store.event_count();
    will_call(
        &world,
        "update_issue",
        json!({"issue": key, "version": 1, "priority": "high"}),
    );
    will_say(&world, "I propose to make it High.");
    let answer = ask(&world, question(&token(&world), "Make WEB-1 high priority")).await?;
    assert_eq!(answer.proposals.len(), 1);
    let proposal = &answer.proposals[0];
    assert_eq!(proposal.tool, "update_issue");
    let change = proposal
        .changes
        .iter()
        .find(|change| change.field == "priority")
        .expect("the priority is listed");
    assert_eq!(change.after, Some(json!("high")));
    assert!(change.before.is_some());
    assert!(!proposal.reference.is_empty());
    // Nothing was saved: the same events, and the issue is as it was.
    assert_eq!(world.store.event_count(), events);
    let now = issue_text(&world, &key).await?;
    assert!(now.contains("version: 1"), "{now}");
    // The model was told what the change would be, and that the person decides.
    assert!(tool_results(&world).contains("would_change"));
    Ok(())
}

/// The proposal of a priority change.
async fn a_proposal(world: &World, key: &str) -> Result<Proposal, AppError> {
    will_call(
        world,
        "update_issue",
        json!({"issue": key, "version": 1, "priority": "high"}),
    );
    will_say(world, "Proposed.");
    let mut answer = ask(world, question(&token(world), "Make it high")).await?;
    answer
        .proposals
        .pop()
        .ok_or(AppError::Refused(ErrorCode::NotFound))
}

fn apply_of(world: &World, proposal: &Proposal) -> ApplyProposal {
    ApplyProposal {
        session_token: token(world),
        reference: proposal.reference.clone(),
        tool: proposal.tool.clone(),
        input: proposal.input.clone(),
    }
}

#[tokio::test]
async fn ai_05_apply_saves_exactly_the_proposed_change_as_the_person() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let proposal = a_proposal(&world, &key).await?;
    let events = world.store.event_count();
    let applied = apply_proposal(&world.ctx(), apply_of(&world, &proposal))
        .await?
        .into_value();
    assert_eq!(applied.changes, proposal.changes);
    assert_eq!(world.store.event_count(), events + 1);
    let now = issue_text(&world, &key).await?;
    assert!(now.contains("version: 2"), "{now}");
    assert!(now.contains("priority: high"), "{now}");
    // As the person: the actor of the event is Ada.
    let event = world.store.lock().events.last().cloned().expect("an event");
    assert_eq!(event.actor.person(), Some(world.admin.person.id));
    Ok(())
}

#[tokio::test]
async fn comp_08_text_applied_from_the_assistant_is_marked_written_with_the_assistant() -> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "add_comment",
        json!({"issue": key, "text": "A summary of the thread."}),
    );
    will_say(&world, "Here is a comment.");
    let answer = ask(&world, question(&token(&world), "Write a comment")).await?;
    apply_proposal(&world.ctx(), apply_of(&world, &answer.proposals[0])).await?;
    let event = world.store.lock().events.last().cloned().expect("an event");
    assert_eq!(event.channel, Channel::Assistant);
    // The channel belongs to that one apply, not to the next request of the person.
    let next = change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: token(&world),
            issue: key,
            version: 1,
            issue_type: None,
            title: Some("By hand".to_owned()),
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await;
    assert!(next.is_ok());
    let by_hand = world.store.lock().events.last().cloned().expect("an event");
    assert_eq!(by_hand.channel, Channel::Ui);
    Ok(())
}

#[tokio::test]
async fn ai_05_apply_against_a_newer_version_is_refused_with_the_newer_value() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let proposal = a_proposal(&world, &key).await?;
    // Somebody else changes the title first.
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: token(&world),
            issue: key.clone(),
            version: 1,
            issue_type: None,
            title: Some("A newer title".to_owned()),
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?;
    let refused = apply_proposal(&world.ctx(), apply_of(&world, &proposal))
        .await
        .unwrap_err();
    assert!(
        matches!(refused, AppError::Stale(_) | AppError::Conflict(_)),
        "{refused:?}"
    );
    let now = issue_text(&world, &key).await?;
    assert!(now.contains("A newer title"), "{now}");
    assert!(!now.contains("priority: high"), "{now}");
    Ok(())
}

#[tokio::test]
async fn ai_05_a_reference_works_only_for_the_change_it_was_made_for() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let proposal = a_proposal(&world, &key).await?;
    let invalid = AppError::Refused(ErrorCode::AiProposalInvalid);
    // Another change with the same reference, another tool, another person, a made-up reference.
    let mut other = apply_of(&world, &proposal);
    other.input = json!({"issue": key, "version": 1, "priority": "critical"});
    assert_eq!(
        apply_proposal(&world.ctx(), other).await.unwrap_err(),
        invalid
    );
    let mut tool = apply_of(&world, &proposal);
    tool.tool = "change_status".to_owned();
    assert_eq!(
        apply_proposal(&world.ctx(), tool).await.unwrap_err(),
        invalid
    );
    let mut theirs = apply_of(&world, &proposal);
    theirs.session_token = world.person("bob@example.test").await?.session_token;
    assert_eq!(
        apply_proposal(&world.ctx(), theirs).await.unwrap_err(),
        invalid
    );
    let mut made_up = apply_of(&world, &proposal);
    made_up.reference = "AAAA".to_owned();
    assert_eq!(
        apply_proposal(&world.ctx(), made_up).await.unwrap_err(),
        invalid
    );
    // The order of the members of the input does not matter: the same change is the same.
    let mut reordered = apply_of(&world, &proposal);
    reordered.input = serde_json::from_str(&format!(
        r#"{{"priority": "high", "version": 1, "issue": "{key}"}}"#
    ))?;
    assert!(apply_proposal(&world.ctx(), reordered).await.is_ok());
    Ok(())
}

#[tokio::test]
async fn ai_05_a_reference_runs_out() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let proposal = a_proposal(&world, &key).await?;
    world.clock.advance(PROPOSAL_LIFETIME_SECONDS + 1);
    let late = apply_proposal(&world.ctx(), apply_of(&world, &proposal))
        .await
        .unwrap_err();
    assert_eq!(late, AppError::Refused(ErrorCode::AiProposalInvalid));
    let now = issue_text(&world, &key).await?;
    assert!(now.contains("version: 1"), "nothing was saved: {now}");
    Ok(())
}

#[tokio::test]
async fn ai_05_apply_checks_again_that_the_issue_is_not_kept_from_ai() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "plain text", BTreeMap::new()).await?;
    let proposal = a_proposal(&world, &key).await?;
    mark(&world, NeverSentTarget::Project("WEB".to_owned())).await?;
    let refused = apply_proposal(&world.ctx(), apply_of(&world, &proposal))
        .await
        .unwrap_err();
    assert_eq!(refused, AppError::Refused(ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_05_an_answer_never_loads_an_outside_picture_and_shows_links_in_full() -> Outcome {
    let (world, _) = with_an_issue().await?;
    will_say(
        &world,
        "![x](https://evil.example/p.png) and [the docs](https://example.com/docs)",
    );
    let answer = ask(&world, question(&token(&world), "Show")).await?;
    assert!(!answer.html.contains("<img"), "{}", answer.html);
    assert!(
        answer
            .html
            .contains("the docs (https://example.com/docs)</a>"),
        "{}",
        answer.html
    );
    assert!(answer.html.contains("https://evil.example/p.png"));
    // The text is kept as the model wrote it.
    assert!(answer.text.contains("![x]"));
    Ok(())
}

#[tokio::test]
async fn comp_10_the_assistant_never_proposes_a_person_by_its_own_choice() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "plain", BTreeMap::new()).await?;
    bob_with_a_role(&world, &every_field()).await?;
    let bob_id = world
        .store
        .lock()
        .state
        .people
        .iter()
        .find(|(person, _)| person.email == "bob@example.test")
        .map(|(person, _)| person.id.to_string())
        .expect("bob exists");
    let assign = json!({"issue": key, "version": 1, "assigned_to": [bob_id]});
    // The model picks Bob by its own choice: the question does not name him.
    will_call(&world, "update_issue", assign.clone());
    will_say(&world, "Done.");
    let chosen = ask(&world, question(&token(&world), "Who should fix this?")).await?;
    assert!(chosen.proposals.is_empty());
    assert!(tool_results(&world).contains("never"), "the model is told");
    // Naming him is not enough without asking for the assignment.
    will_call(&world, "update_issue", assign.clone());
    will_say(&world, "Done.");
    let only_named = ask(
        &world,
        question(&token(&world), "Is Bob Builder on holiday?"),
    )
    .await?;
    assert!(only_named.proposals.is_empty());
    // The person asks for exactly this: the proposal is shown.
    will_call(&world, "update_issue", assign);
    will_say(&world, "Done.");
    let named = ask(
        &world,
        question(&token(&world), "Assign WEB-1 to Bob Builder"),
    )
    .await?;
    assert_eq!(named.proposals.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_02_the_assistant_answers_ai_is_off_and_sends_nothing_where_ai_is_off() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    a_local_model(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "plain", BTreeMap::new()).await?;
    // Off for the organization.
    let off = ask(&world, question(&token(&world), "Hello"))
        .await
        .unwrap_err();
    assert_eq!(off, AppError::Refused(ErrorCode::AiOff));
    // On for the organization, off for the project the panel looks at.
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "WEB".to_owned(),
            enabled: Some(false),
        },
    )
    .await?;
    let looking = AskAssistantQuery {
        looking_at: vec![LookingAt {
            kind: LookingAtKind::Issue,
            value: key,
        }],
        ..question(&token(&world), "Hello")
    };
    let off = ask(&world, looking).await.unwrap_err();
    assert_eq!(off, AppError::Refused(ErrorCode::AiOff));
    assert!(world.ai.calls().is_empty(), "nothing was sent");
    Ok(())
}

/// An outside provider (Anthropic) with its agreement, AI on.
pub(super) async fn outside_provider(world: &World) -> Outcome {
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            model: Some("claude-test".to_owned()),
            ..setup(world)
        },
    )
    .await?;
    set_ai_key(
        &world.ctx(),
        SetAiKey {
            session_token: token(world),
            key: Some("a made-up key".to_owned()),
            password: PASSWORD.to_owned(),
        },
    )
    .await?;
    confirm_ai_agreement(
        &world.ctx(),
        ConfirmAiAgreement {
            session_token: token(world),
            destination: Destination::Anthropic,
        },
    )
    .await?;
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(world)
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn priv_08_names_of_people_go_to_an_outside_provider_only_when_switched_on() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    outside_provider(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "plain", BTreeMap::new()).await?;
    let (bob, _) = bob_with_a_role(&world, &every_field()).await?;
    let sent_to_provider = async || -> Result<String, AppError> {
        will_call(&world, "get_issue", json!({"issue": key}));
        will_say(&world, "Fine.");
        ask(&world, question(&bob, "Who wrote WEB-1?")).await?;
        Ok(tool_results(&world))
    };
    // Ada wrote the issue: to Bob she is her role, never her name.
    let theirs = sent_to_provider().await?;
    assert!(theirs.contains("Administrator"), "{theirs}");
    assert!(!theirs.contains("Ada"), "{theirs}");
    // An administrator switches the names on.
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            send_names: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    let named = sent_to_provider().await?;
    assert!(named.contains("Ada"), "{named}");
    Ok(())
}

/// The scenario of `apply_proposal` for the table of events: a proposal is applied and one event
/// comes of it.
pub(crate) async fn apply_proposal_scenario(world: &World) -> super::scenarios::Added {
    a_project(world, "WEB").await?;
    ai_on(world).await.map_err(|error| error.to_string())?;
    let key = issue_with(world, "WEB", "Login", "plain text", BTreeMap::new()).await?;
    let proposal = a_proposal(world, &key)
        .await
        .map_err(|error| error.to_string())?;
    super::scenarios::measured(
        &world.store,
        apply_proposal(&world.ctx(), apply_of(world, &proposal)),
    )
    .await
}

#[tokio::test]
async fn ai_02_a_project_with_ai_switched_off_is_kept_from_every_tool() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    a_project(&world, "OPS").await?;
    ai_on(&world).await.map_err(|error| error.to_string())?;
    let web = issue_with(&world, "WEB", "Login", "plain", BTreeMap::new()).await?;
    let ops = issue_with(&world, "OPS", "Pager duty", "plain", BTreeMap::new()).await?;
    change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "OPS".to_owned(),
            enabled: Some(false),
        },
    )
    .await?;
    // The read tools, the search and the list of projects leave the project out; a preview of a
    // change in it is not found; the other project is as before.
    let admin = token(&world);
    assert!(
        read(&world, &admin, "get_issue", json!({"issue": ops}))
            .await
            .is_err()
    );
    // The search is narrowed to the projects the AI may look at.
    let ops_id = world
        .store
        .lock()
        .state
        .projects
        .iter()
        .find(|project| project.short_name.as_str() == "OPS")
        .map(|project| project.id)
        .ok_or("no OPS")?;
    read(&world, &admin, "search_issues", json!({"query": ""})).await?;
    let request = world.store.lock().searches.pop().ok_or("no search ran")?;
    assert!(request.projects.iter().all(|link| link.id != ops_id));
    let listed = read(&world, &admin, "list_projects", json!({})).await?;
    assert!(!listed.contains("OPS"), "{listed}");
    let preview = {
        let mut query = super::ai_tools::call(
            &admin,
            "update_issue",
            json!({"issue": ops, "version": 1, "priority": "high"}),
        );
        query.mode = crate::ai::tools::ToolMode::Preview;
        crate::ai::tools::call_tool(&world.ctx(), query)
            .await
            .map(|_| ())
    };
    assert_eq!(preview, Err(AppError::Refused(ErrorCode::NotFound)));
    assert!(
        read(&world, &admin, "get_issue", json!({"issue": web}))
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn ai_05_a_reference_works_once() -> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "add_comment",
        json!({"issue": key, "text": "A summary of the thread."}),
    );
    will_say(&world, "Here is a comment.");
    let answer = ask(&world, question(&token(&world), "Write a comment")).await?;
    let events = world.store.event_count();
    apply_proposal(&world.ctx(), apply_of(&world, &answer.proposals[0])).await?;
    assert_eq!(world.store.event_count(), events + 1);
    // A retry or a second click saves nothing again.
    let again = apply_proposal(&world.ctx(), apply_of(&world, &answer.proposals[0]))
        .await
        .unwrap_err();
    assert_eq!(again, AppError::Refused(ErrorCode::AiProposalInvalid));
    assert_eq!(world.store.event_count(), events + 1);
    // The same change proposed again is a new reference and works.
    will_call(
        &world,
        "add_comment",
        json!({"issue": key, "text": "A summary of the thread."}),
    );
    will_say(&world, "Again.");
    let second = ask(&world, question(&token(&world), "Write it again")).await?;
    assert_ne!(second.proposals[0].reference, answer.proposals[0].reference);
    apply_proposal(&world.ctx(), apply_of(&world, &second.proposals[0])).await?;
    assert_eq!(world.store.event_count(), events + 2);
    Ok(())
}

#[tokio::test]
async fn ai_05_a_change_too_long_to_be_applied_is_not_proposed() -> Outcome {
    let (world, key) = with_an_issue().await?;
    let long = "x".repeat(bb_core::ai::PROPOSAL_INPUT_BYTES_MAX + 1);
    will_call(&world, "add_comment", json!({"issue": key, "text": long}));
    will_say(&world, "Done.");
    let answer = ask(&world, question(&token(&world), "Write a long comment")).await?;
    assert!(answer.proposals.is_empty());
    assert!(tool_results(&world).contains("not valid"));
    Ok(())
}
