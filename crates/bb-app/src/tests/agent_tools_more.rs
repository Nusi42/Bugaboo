//! The edit locks of agents, *changes since*, the limits of an agent and the standing rules at work
//! (AI-32, AI-36, AI-37, ADR-0019, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent::{AgentState, LimitKind};
use bb_core::agent_work::{AgentWorkState, HandOverPower};
use bb_core::edit::{DESCRIPTION_PART, EditWhat};
use bb_core::event::RecordKind;
use bb_core::id::{EditorId, IssueId, LockToken};
use bb_core::technical::TechnicalName;
use serde_json::{Value, json};

use super::agent_tools::{previewed, used};
use super::agent_work::{Handed, handed, handed_with, query};
use super::agents::refused;
use super::ai_tools::without_nonces;
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::{AppError, entity};
use crate::editing::{TakeLockQuery, take_lock};
use crate::issues::{ChangeIssue, SaveDescription, change_issue, issue, save_description};

/// A hand-over that lets the agent change the description.
async fn handed_with_description(world: &World) -> Result<Handed, AppError> {
    handed_with(world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["description".to_owned()];
    })
    .await
}

fn description_of(issue: IssueId) -> Result<EditWhat, AppError> {
    Ok(EditWhat {
        record: entity(RecordKind::Issue, issue),
        part: TechnicalName::new(DESCRIPTION_PART)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    })
}

async fn save_as_agent(
    world: &World,
    given: &Handed,
    version: i64,
    text: &str,
) -> Result<bb_policy::Authorized<crate::issues::IssueView>, AppError> {
    save_description(
        &world.ctx(),
        SaveDescription {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            version,
            // The agent has no editor open: whatever it sends here, the save takes its own lock.
            lock_token: LockToken::generate(&mut world.ctx().ids()),
            text: text.to_owned(),
        },
    )
    .await
}

async fn version_of(world: &World, given: &Handed) -> Result<i64, AppError> {
    Ok(issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version)
}

#[tokio::test]
async fn ai_36_an_agent_never_takes_over_a_lock() -> Outcome {
    let world = World::new().await?;
    let given = handed_with_description(&world).await?;
    let version = version_of(&world, &given).await?;
    // The person is editing the description in a window of their own.
    let editing = take_lock(
        &world.ctx(),
        TakeLockQuery {
            session_token: given.team.session_token.clone(),
            what: description_of(given.issue)?,
            editor: EditorId::generate(&mut world.ctx().ids()),
        },
    )
    .await?
    .into_value();
    assert!(matches!(editing, crate::editing::LockOutcome::Taken(_)));
    // The agent does not take their text, nor their lock, and writes nothing.
    let events = world.store.lock().events.len();
    let blocked = save_as_agent(&world, &given, version, "The agent's text").await;
    assert!(
        matches!(
            blocked,
            Err(AppError::TakenOver { .. } | AppError::Refused(ErrorCode::EditTakenOver))
        ),
        "{blocked:?}"
    );
    assert_eq!(world.store.lock().events.len(), events);
    assert!(
        world
            .store
            .lock()
            .state
            .issues
            .iter()
            .all(|stored| stored.description != "The agent's text")
    );
    // The lock is still the person's.
    let still = world.store.lock().state.locks.clone();
    assert_eq!(still.len(), 1);
    assert_eq!(still[0].holder, given.team.person.id);
    Ok(())
}

#[tokio::test]
async fn ai_36_an_agent_holds_a_lock_for_one_save_and_no_longer() -> Outcome {
    let world = World::new().await?;
    let given = handed_with_description(&world).await?;
    let version = version_of(&world, &given).await?;
    save_as_agent(&world, &given, version, "Steps: press the button").await?;
    assert!(world.store.lock().state.locks.is_empty());
    // A save against an older version is refused with the newer text, and holds no lock after.
    let refusal = save_as_agent(&world, &given, version, "Another text").await;
    assert!(matches!(refusal, Err(AppError::Stale(_))), "{refusal:?}");
    assert!(world.store.lock().state.locks.is_empty());
    // The agent's refused text is nobody's draft: not its person's.
    assert!(world.store.lock().state.drafts.is_empty());
    Ok(())
}

/// A second issue of the project that the agent was not handed, and one in a project the person
/// does not see.
struct Elsewhere {
    other: IssueId,
    hidden: IssueId,
}

async fn elsewhere(world: &World, given: &Handed) -> Result<Elsewhere, String> {
    let other = an_issue(world, given.project).await?;
    let outside = a_project(world, "HIDDEN").await?;
    let hidden = an_issue(world, outside).await?;
    Ok(Elsewhere { other, hidden })
}

pub(super) async fn retitle(world: &World, id: IssueId, title: &str) -> Result<(), AppError> {
    let version = issue(&world.ctx(), query(&token(world), id))
        .await?
        .into_value()
        .issue
        .version;
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: token(world),
            issue: id.to_string(),
            version,
            issue_type: None,
            title: Some(title.to_owned()),
            status: None,
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::default(),
        },
    )
    .await?;
    Ok(())
}

async fn key_of_issue(world: &World, id: IssueId) -> Result<String, AppError> {
    Ok(issue(&world.ctx(), query(&token(world), id))
        .await?
        .into_value()
        .issue
        .key)
}

pub(super) fn mark_of(text: &str) -> String {
    text.lines()
        .find_map(|line| line.trim().strip_prefix("mark: "))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

#[tokio::test]
async fn ai_36_changes_since_shows_only_what_the_agent_may_see() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let other = elsewhere(&world, &given).await?;
    // The first look tells nothing and gives the mark of now.
    let first = used(&world, &given.key, "changes_since", json!({})).await?;
    let first = without_nonces(&first.text);
    let mark = mark_of(&first);
    assert!(!mark.is_empty(), "{first}");
    assert!(!first.contains("change:"), "{first}");
    retitle(&world, given.issue, "Handed issue, new title").await?;
    retitle(&world, other.other, "Other issue of the project").await?;
    retitle(&world, other.hidden, "Issue nobody of the team sees").await?;
    let handed_key = key_of_issue(&world, given.issue).await?;
    let other_key = key_of_issue(&world, other.other).await?;
    let hidden_key = key_of_issue(&world, other.hidden).await?;
    let second = used(&world, &given.key, "changes_since", json!({"since": mark})).await?;
    let text = without_nonces(&second.text);
    assert!(text.contains(&handed_key), "{text}");
    assert!(text.contains("Handed issue, new title"), "{text}");
    assert!(
        !text.contains(&other_key),
        "an issue it was not handed: {text}"
    );
    assert!(!text.contains("Other issue of the project"), "{text}");
    assert!(!text.contains(&hidden_key), "an issue nobody sees: {text}");
    assert!(!text.contains("Issue nobody of the team sees"), "{text}");
    // Its work is told as well: where it stands.
    assert!(text.contains("waiting-for-the-agent"), "{text}");
    // The mark moved on: nothing new, nothing told again.
    let third = used(
        &world,
        &given.key,
        "changes_since",
        json!({"since": mark_of(&text)}),
    )
    .await?;
    assert!(!without_nonces(&third.text).contains("Handed issue, new title"));
    Ok(())
}

#[tokio::test]
async fn ai_36_changes_since_is_paged_and_a_mark_that_is_none_is_refused() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let first = used(&world, &given.key, "changes_since", json!({})).await?;
    let mark = mark_of(&without_nonces(&first.text));
    retitle(&world, given.issue, "First new title").await?;
    retitle(&world, given.issue, "Second new title").await?;
    let page = used(
        &world,
        &given.key,
        "changes_since",
        json!({"since": mark, "limit": 1}),
    )
    .await?;
    let page = without_nonces(&page.text);
    assert!(page.contains("First new title"), "{page}");
    assert!(!page.contains("Second new title"), "{page}");
    assert!(page.contains("more: true"), "{page}");
    let rest = used(
        &world,
        &given.key,
        "changes_since",
        json!({"since": mark_of(&page), "limit": 1}),
    )
    .await?;
    let rest = without_nonces(&rest.text);
    assert!(rest.contains("Second new title"), "{rest}");
    // Each page holds one change; the second one tells what the title was (the first change).
    assert_eq!(rest.matches("change:").count(), 1, "{rest}");
    assert!(!rest.contains("more: true"), "{rest}");
    let bad = used(&world, &given.key, "changes_since", json!({"since": "x"})).await;
    assert!(refused(&bad.unwrap_err(), ErrorCode::InvalidRequest));
    Ok(())
}

/// Lowers a limit of the agent in the store, as its person would on its page.
fn limit(world: &World, kind: LimitKind, value: i64) {
    let mut shared = world.store.lock();
    for agent in &mut shared.state.agents {
        match kind {
            LimitKind::ChangesAMinute => agent.changes_a_minute = value,
            LimitKind::IssuesAtOnce => agent.issues_at_once = value,
            LimitKind::AttemptsOnOneIssue => agent.attempts_on_one_issue = value,
        }
    }
}

fn the_agent(world: &World) -> bb_core::agent::Agent {
    world.store.lock().state.agents[0].clone()
}

#[tokio::test]
async fn ai_37_reaching_a_limit_pauses_the_agent_with_a_sentence_and_no_change_halfway() -> Outcome
{
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::ChangesAMinute, 3);
    let issue_text = given.issue.to_string();
    // The first three changes are within the limit, at the edge of it.
    used(&world, &given.key, "take", json!({"issue": issue_text})).await?;
    for minutes in [1, 2] {
        used(
            &world,
            &given.key,
            "report",
            json!({"issue": issue_text, "minutes": minutes, "cost_cents": 0}),
        )
        .await?;
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    // Reads count nothing.
    for _ in 0..5 {
        used(&world, &given.key, "changes_since", json!({})).await?;
        previewed(
            &world,
            &given.key,
            "report",
            json!({"issue": issue_text, "minutes": 9, "cost_cents": 0}),
        )
        .await?;
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    // The fourth does not start: nothing of it is written, and the agent is paused.
    let events = world.store.lock().events.len();
    let attempts = format!("{:?}", world.store.lock().state.work.attempts);
    let over = used(
        &world,
        &given.key,
        "report",
        json!({"issue": issue_text, "minutes": 3, "cost_cents": 0}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    let agent = the_agent(&world);
    assert_eq!(agent.state, AgentState::Paused);
    assert_eq!(agent.paused_for, Some(LimitKind::ChangesAMinute));
    // Only the pause itself was written.
    assert_eq!(world.store.lock().events.len(), events + 1);
    assert_eq!(
        attempts,
        format!("{:?}", world.store.lock().state.work.attempts)
    );
    // The sentence names the agent and the limit.
    assert_eq!(
        LimitKind::ChangesAMinute
            .sentence(&agent.name, agent.changes_a_minute)
            .render(),
        "Fixer paused: 3 changes in a minute"
    );
    // A paused agent is refused at its next request, whatever it asks.
    let after = used(&world, &given.key, "changes_since", json!({})).await;
    assert!(refused(&after.unwrap_err(), ErrorCode::NotSignedIn));
    // Its person is told.
    world.ctx().catch_up_notifications().await?;
    let told = world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .filter(|row| {
            row.recipient == given.team.person.id
                && row.kind == bb_core::notification::NotificationKind::AgentStopped
        })
        .count();
    assert_eq!(told, 1, "the person was not told");
    // Resuming starts the agent from nothing.
    crate::agents::change_agent(
        &world.ctx(),
        crate::agents::ChangeAgent {
            session_token: given.team.session_token.clone(),
            id: agent.id,
            action: crate::agents::AgentAction::Resume,
            version: agent.version,
        },
    )
    .await?;
    assert_eq!(the_agent(&world).paused_for, None);
    used(
        &world,
        &given.key,
        "report",
        json!({"issue": issue_text, "minutes": 4, "cost_cents": 0}),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_37_an_agent_works_on_as_many_issues_at_once_as_it_may_and_no_more() -> Outcome {
    let world = World::new().await?;
    let first = handed(&world).await?;
    let second = an_issue(&world, first.project).await?;
    crate::agent_work::hand_over(
        &world.ctx(),
        super::agent_work::plain_hand_over(&first.team.session_token, first.agent, &[second]),
    )
    .await?;
    limit(&world, LimitKind::IssuesAtOnce, 1);
    used(
        &world,
        &first.key,
        "take",
        json!({"issue": first.issue.to_string()}),
    )
    .await?;
    let over = used(
        &world,
        &first.key,
        "take",
        json!({"issue": second.to_string()}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    let agent = the_agent(&world);
    assert_eq!(agent.state, AgentState::Paused);
    assert_eq!(agent.paused_for, Some(LimitKind::IssuesAtOnce));
    // The work on the second issue did not start.
    let started = world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.issue == second && work.state != AgentWorkState::WaitingForTheAgent)
        .count();
    assert_eq!(started, 0);
    Ok(())
}

#[tokio::test]
async fn ai_37_an_agent_makes_as_many_attempts_on_an_issue_as_it_may_and_no_more() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::AttemptsOnOneIssue, 1);
    let address = given.issue.to_string();
    used(&world, &given.key, "take", json!({"issue": address})).await?;
    used(&world, &given.key, "submit", json!({"issue": address})).await?;
    // The person sends it back; a second attempt would pass the limit.
    let work = super::agent_work_review::show(&world, &given).await?;
    crate::agent_work::review_work(
        &world.ctx(),
        crate::agent_work::ReviewWork {
            note: Some("Try the other way".to_owned()),
            ..super::agent_work_review::review(
                &given.team.session_token,
                &work,
                bb_core::agent_work::ReviewAnswer::SendBack,
            )
        },
    )
    .await?;
    let over = used(&world, &given.key, "take", json!({"issue": address})).await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    assert_eq!(
        the_agent(&world).paused_for,
        Some(LimitKind::AttemptsOnOneIssue)
    );
    Ok(())
}

#[tokio::test]
async fn ai_37_a_preview_that_finds_a_limit_pauses_nobody() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    limit(&world, LimitKind::IssuesAtOnce, 0);
    let over = previewed(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string()}),
    )
    .await;
    assert!(refused(&over.unwrap_err(), ErrorCode::AgentLimitReached));
    assert_eq!(the_agent(&world).state, AgentState::Active);
    Ok(())
}

#[tokio::test]
async fn ai_37_the_changes_of_an_agent_over_the_api_count_against_its_limits() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned()];
    })
    .await?;
    limit(&world, LimitKind::ChangesAMinute, 1);
    let mut counts = Vec::new();
    for title in ["One", "Two"] {
        let version = version_of(&world, &given).await?;
        // The use cases of the API count a change when it is made through `AuthApi`: the table of
        // the use cases says which inputs change something.
        let ctx = world.ctx();
        ctx.expect_change();
        counts.push(
            change_issue(
                &ctx,
                ChangeIssue {
                    session_token: given.key.clone(),
                    issue: given.issue.to_string(),
                    version,
                    issue_type: None,
                    title: Some(title.to_owned()),
                    status: None,
                    priority: None,
                    estimated_time: None,
                    time_left: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::default(),
                },
            )
            .await,
        );
        // `AuthApi` settles a request that was made; here it is done by hand.
        if counts.last().is_some_and(Result::is_ok) {
            ctx.settle_agent_change();
        }
    }
    assert!(counts[0].is_ok());
    assert!(refused(
        counts[1].as_ref().unwrap_err(),
        ErrorCode::AgentLimitReached
    ));
    assert_eq!(the_agent(&world).state, AgentState::Paused);
    Ok(())
}

/// The value of the title the person sees, to show that nothing changed.
async fn title_of(world: &World, given: &Handed) -> Result<(String, i64), AppError> {
    let view = issue(&world.ctx(), query(&given.team.session_token, given.issue))
        .await?
        .into_value()
        .issue;
    Ok((view.title, view.version))
}

fn the_text(value: &Value) -> String {
    value.to_string()
}

#[tokio::test]
async fn ai_32_a_standing_rule_that_proposes_changes_nothing_through_its_tools() -> Outcome {
    use super::agent_work_rules::{found_by_search, made, rule_world};
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let as_given = Handed {
        project: given.project,
        team: given.team.clone(),
        agent: given.agent,
        key: given.key.clone(),
        issue: given.issue,
        hand_over: crate::agent_work::HandOverView {
            id: bb_core::id::HandOverId::generate(&mut world.ctx().ids()),
            agent: given.agent,
            issues: Vec::new(),
            ends_at: world.ctx().now(),
            state: bb_core::agent_records::GivingState::Open,
            version: 1,
        },
    };
    let before = title_of(&world, &as_given).await?;
    let address = given.issue.to_string();
    // Its matches reach it through *changes since*.
    let first = used(&world, &given.key, "changes_since", json!({})).await?;
    assert!(!first.text.is_empty());
    // It is not handed the issue: every tool that works on an issue is turned away.
    for (tool, input) in [
        (
            "update_issue",
            json!({"issue": address, "version": before.1, "title": "Changed by the rule"}),
        ),
        (
            "change_status",
            json!({"issue": address, "version": before.1, "status": "done"}),
        ),
        ("take", json!({"issue": address})),
        (
            "report",
            json!({"issue": address, "minutes": 1, "cost_cents": 0}),
        ),
        ("ask", json!({"issue": address, "question": "Why?"})),
    ] {
        let turned_away = used(&world, &given.key, tool, input).await;
        assert!(turned_away.is_err(), "{tool} was not turned away");
    }
    assert_eq!(before, title_of(&world, &as_given).await?);
    // What it may do is propose; the proposal is a record and the issue stays as it was.
    let proposed = used(
        &world,
        &given.key,
        "propose",
        json!({"rule": rule.id.to_string(), "issue": address, "field": "labels", "value": ["bug"]}),
    )
    .await?;
    assert!(proposed.text.contains("labels"), "{}", proposed.text);
    assert_eq!(world.store.lock().state.work.proposals.len(), 1);
    assert_eq!(before, title_of(&world, &as_given).await?);
    // Only a field the rule is for, and never the status or a person.
    for field in ["status", "assigned-to"] {
        let refusal = used(
            &world,
            &given.key,
            "propose",
            json!({"rule": rule.id.to_string(), "issue": address, "field": field, "value": "x"}),
        )
        .await;
        assert!(
            refused(&refusal.unwrap_err(), ErrorCode::InvalidRequest),
            "{field}"
        );
    }
    assert!(the_text(&json!(world.store.lock().state.work.proposals.len())).contains('1'));
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_propose_answers_what_would_change_and_saves_nothing() -> Outcome {
    use super::agent_work_rules::{found_by_search, made, rule_world};
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let input = json!({"rule": rule.id.to_string(), "issue": given.issue.to_string(), "field": "labels", "value": ["bug"]});
    let events = world.store.lock().events.len();
    let preview = previewed(&world, &given.key, "propose", input.clone()).await?;
    assert!(preview.text.contains("would_change"), "{}", preview.text);
    assert_eq!(world.store.lock().events.len(), events);
    assert!(world.store.lock().state.work.proposals.is_empty());
    let saved = used(&world, &given.key, "propose", input).await?;
    assert_eq!(preview.changes, saved.changes);
    assert_eq!(world.store.lock().state.work.proposals.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_37_a_change_that_is_refused_counts_nothing() -> Outcome {
    let world = World::new().await?;
    // Nothing ticked: every change of a field is refused.
    let given = handed(&world).await?;
    limit(&world, LimitKind::ChangesAMinute, 1);
    for title in ["One", "Two", "Three"] {
        let version = version_of(&world, &given).await?;
        let ctx = world.ctx();
        ctx.expect_change();
        let refusal = change_issue(
            &ctx,
            ChangeIssue {
                session_token: given.key.clone(),
                issue: given.issue.to_string(),
                version,
                issue_type: None,
                title: Some(title.to_owned()),
                status: None,
                priority: None,
                estimated_time: None,
                time_left: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::default(),
            },
        )
        .await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    }
    assert_eq!(the_agent(&world).state, AgentState::Active);
    Ok(())
}
