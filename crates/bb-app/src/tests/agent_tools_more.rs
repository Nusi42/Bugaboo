//! The edit locks of agents, *changes since*, the limits of an agent and the standing rules at work
//! (AI-32, AI-36, AI-37, ADR-0019, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::edit::{DESCRIPTION_PART, EditWhat};
use bb_core::event::RecordKind;
use bb_core::id::{EditorId, IssueId, LockToken};
use bb_core::technical::TechnicalName;
use serde_json::{Value, json};

use super::agent_tools::{previewed, used};
use super::agent_work::{Handed, handed, handed_with, plain_hand_over, query, start_attempt};
use super::agents::refused;
use super::agents::{an_agent, key_of};
use super::ai_tools::without_nonces;
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::agent_work::hand_over;
use crate::auth::{AppError, entity};
use crate::editing::{TakeLockQuery, take_lock};
use crate::issues::{ChangeIssue, SaveDescription, change_issue, issue, save_description};

/// A hand-over that lets the agent change the description.
async fn handed_with_description(world: &World) -> Result<Handed, AppError> {
    let given = handed_with(world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["description".to_owned()];
    })
    .await?;
    start_attempt(world, &given.key, given.issue).await?;
    Ok(given)
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

pub(super) async fn version_of(world: &World, given: &Handed) -> Result<i64, AppError> {
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
async fn ai_36_a_report_never_starts_an_attempt_only_take_does() -> Outcome {
    let world = World::new().await?;
    let first = handed(&world).await?;
    let session = first.team.session_token.clone();
    let helper = an_agent(&world, &session, "Helper").await?;
    let helper_key = key_of(&world, &session, helper).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&session, helper, &[first.issue]),
    )
    .await?;
    let address = first.issue.to_string();
    let report = json!({"issue": address, "minutes": 1, "cost_cents": 0});
    // Waiting for the agent: a report is turned away and nothing starts.
    let before = format!("{:?}", world.store.lock().state.work.attempts);
    let early = used(&world, &first.key, "report", report.clone()).await;
    assert!(refused(&early.unwrap_err(), ErrorCode::AgentWorkNotReady));
    assert_eq!(
        before,
        format!("{:?}", world.store.lock().state.work.attempts)
    );
    // A works on the issue; B reports on it and is turned away, and B cannot take it either.
    used(&world, &first.key, "take", json!({"issue": address})).await?;
    let other = used(&world, &helper_key, "report", report).await;
    assert!(refused(&other.unwrap_err(), ErrorCode::AgentWorkNotReady));
    let taken = used(&world, &helper_key, "take", json!({"issue": address})).await;
    assert!(refused(&taken.unwrap_err(), ErrorCode::AgentWorkNotReady));
    assert_eq!(world.store.lock().state.work.attempts.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_36_an_agents_save_leaves_the_draft_of_its_person() -> Outcome {
    let world = World::new().await?;
    let given = handed_with_description(&world).await?;
    let version = version_of(&world, &given).await?;
    crate::editing::save_draft(
        &world.ctx(),
        crate::editing::SaveDraftQuery {
            session_token: given.team.session_token.clone(),
            target: bb_core::edit::DraftTarget::Editing(description_of(given.issue)?),
            draft: crate::editing::DraftText {
                text: "The unsaved words of the person".to_owned(),
                base_version: Some(version),
            },
        },
    )
    .await?;
    assert_eq!(world.store.lock().state.drafts.len(), 1);
    save_as_agent(&world, &given, version, "Steps: press the button").await?;
    assert_eq!(world.store.lock().state.drafts.len(), 1, "the draft stays");
    Ok(())
}

#[tokio::test]
async fn ai_32_an_agent_comments_only_on_an_issue_it_was_handed() -> Outcome {
    use super::agent_work_rules::{found_by_search, made, rule_world};
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    made(&world, &given).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    // The rule found the issue and the agent reads it, but it was not handed over.
    let comment = json!({"issue": given.issue.to_string(), "text": "A comment"});
    let refusal = used(&world, &given.key, "add_comment", comment).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    // An issue it is handed, it may comment on.
    let other = an_issue(&world, given.project).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[other]),
    )
    .await?;
    let comment = json!({"issue": other.to_string(), "text": "A comment"});
    used(&world, &given.key, "add_comment", comment).await?;
    Ok(())
}

#[tokio::test]
async fn ai_36_more_matches_than_a_page_all_reach_the_agent() -> Outcome {
    use super::agent_work_rules::{made, rule_world};
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    let first = used(&world, &given.key, "changes_since", json!({})).await?;
    let mut mark = mark_of(&without_nonces(&first.text));
    let mut found = Vec::new();
    for _ in 0..230 {
        found.push(an_issue(&world, given.project).await?);
    }
    world.clock.advance(10);
    {
        let now = world.ctx().now();
        let mut shared = world.store.lock();
        for issue in &found {
            shared
                .state
                .work
                .matches
                .push(bb_core::agent_records::RuleMatch {
                    rule: rule.id,
                    issue: *issue,
                    at: now,
                });
        }
    }
    let mut seen = 0;
    for _ in 0..10 {
        let page = used(
            &world,
            &given.key,
            "changes_since",
            json!({"since": mark, "limit": 50}),
        )
        .await?;
        let text = without_nonces(&page.text);
        seen += text.matches("match:").count();
        mark = mark_of(&text);
        if !text.contains("more: true") {
            break;
        }
    }
    assert_eq!(seen, 230, "every match reached the agent, the newest too");
    Ok(())
}

#[tokio::test]
async fn ai_36_only_a_web_address_is_attached_as_a_link() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let address = given.issue.to_string();
    used(&world, &given.key, "take", json!({"issue": address})).await?;
    for bad in [
        "javascript:alert(1)",
        "data:text/html,x",
        "file:///etc/passwd",
        "//evil.example/x",
        "https://",
        "https://a b",
        "not a link",
    ] {
        let refusal = used(
            &world,
            &given.key,
            "attach",
            json!({"issue": address, "kind": "link", "label": "Branch", "address": bad}),
        )
        .await;
        assert!(
            refused(&refusal.unwrap_err(), ErrorCode::InvalidRequest),
            "{bad}"
        );
    }
    used(
        &world,
        &given.key,
        "attach",
        json!({"issue": address, "kind": "link", "label": "Branch", "address": "https://example.test/pull/1"}),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_36_the_answer_to_an_old_version_leaves_out_a_field_kept_from_ai() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let version = version_of(&world, &given).await?;
    retitle(&world, given.issue, "A title that changed").await?;
    crate::ai::set_never_sent(
        &world.ctx(),
        crate::ai::SetNeverSent {
            session_token: token(&world),
            target: crate::ai::NeverSentTarget::Field(bb_core::fields::Field::Title),
            marked: true,
            version: 0,
        },
    )
    .await?;
    let stale = used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string(), "version": version}),
    )
    .await;
    let Err(AppError::Conflict(conflict)) = stale else {
        return Err("a stale take is answered with what changed".into());
    };
    assert!(
        !conflict.fields.iter().any(|field| field == "title"),
        "{:?}",
        conflict.fields
    );
    assert!(!conflict.current.contains_key("title"));
    Ok(())
}

#[tokio::test]
async fn ai_36_the_answer_to_an_old_version_is_judged_for_the_agent_not_its_person() -> Outcome {
    use super::search_fields::set_field;
    use super::workflows_rules::starting_role;
    let world = World::new().await?;
    let given = handed(&world).await?;
    let version = version_of(&world, &given).await?;
    // The issue changes: its title and its priority.
    let admin = token(&world);
    let current = issue(&world.ctx(), query(&admin, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: admin,
            issue: given.issue.to_string(),
            version: current,
            issue_type: None,
            title: Some("A new title".to_owned()),
            status: None,
            priority: Some(bb_core::issue::Priority::High),
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::default(),
        },
    )
    .await?;
    // The role of the person cannot see the priority.
    let role = starting_role(&world, bb_policy::StartingRole::TeamMember).await?;
    set_field(
        &world,
        role,
        None,
        bb_core::fields::FieldKey::BuiltIn(bb_core::fields::Field::Priority),
        Some(bb_core::access::FieldLevel::Hidden),
    )
    .await?;
    let stale = used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string(), "version": version}),
    )
    .await;
    let Err(AppError::Conflict(conflict)) = stale else {
        return Err("a stale take is answered with what changed".into());
    };
    assert!(conflict.fields.iter().any(|field| field == "title"));
    assert!(!conflict.fields.iter().any(|field| field == "priority"));
    assert!(!conflict.current.contains_key("priority"));
    Ok(())
}
