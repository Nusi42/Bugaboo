//! What an agent may reach and write, and what ends its work (AI-31, AI-33, ADR-0029, decision 3;
//! COMP-10): the members a hand-over can name, the writes beside the change of an issue, the end
//! of the work when its person loses the right to agents, and the reviews that race.

use bb_core::ErrorCode;
use bb_core::agent_records::GivingState;
use bb_core::agent_work::{AgentWorkState, CheckResult, HandOverPower, ReviewAnswer};
use bb_core::event::RecordKind;
use bb_core::id::{AssignmentId, IssueId};
use bb_core::notification::NotificationKind;
use bb_core::own_fields::FieldType;
use bb_policy::StartingRole;
use serde_json::json;

use super::agent_work::report_attempt;
use super::agent_work::{Handed, handed, handed_with, plain_hand_over, query};
use super::agent_work_review::{ready, report, review, show};
use super::agents::{an_agent, key_of, refused};
use super::scenarios::{a_project, token};
use super::scenarios_fields::a_field;
use super::scenarios_issues::{a_type, an_issue};
use super::workflows_rules::starting_role;
use super::world::{Outcome, World};
use crate::agent_work::{
    AgentWorkQuery, end_attempt, event_member, hand_over, issue_agent_work, look_at_work,
    review_work,
};
use crate::auth::AppError;
use crate::comments::{AddComment, DeleteComment, add_comment, delete_comment};
use crate::issues::{ChangeIssue, CreateIssue, change_issue, create_issue, issue};
use crate::roles::{AssignRole, Given, RemoveAssignment, Subject, assign_role, remove_assignment};

/// The hand-over that lets the agent change these members.
fn ticking(fields: &[&str]) -> impl FnOnce(&mut crate::agent_work::HandOver) {
    let fields: Vec<String> = fields.iter().map(|each| (*each).to_owned()).collect();
    move |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = fields;
    }
}

#[tokio::test]
async fn ai_31_a_hand_over_names_only_members_an_agent_may_be_given() -> Outcome {
    let world = World::new().await?;
    let text = a_field(&world, "Customer", FieldType::Text, &[]).await?;
    let people = a_field(&world, "Owner", FieldType::Person, &[]).await?;
    let (text, people) = (text.id.to_string(), people.id.to_string());
    for refused_field in [
        people.as_str(),
        "assigned-to",
        "reported-by",
        "followers",
        "nope",
    ] {
        // Each try in a world of its own: a hand-over makes its person and agent.
        let world = World::new().await?;
        let result = handed_with(&world, ticking(&["title", refused_field])).await;
        assert!(
            refused(&result.err().ok_or("accepted")?, ErrorCode::InvalidRequest),
            "{refused_field}"
        );
    }
    // A text field of the catalogue is a member an agent can be given, and it changes it.
    let given = handed_with(&world, ticking(&[&text])).await?;
    let version = issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    let mut change = ChangeIssue::of(&given.key, given.issue.to_string(), version);
    change.fields.insert(text.clone(), json!("Acme"));
    change_issue(&world.ctx(), change).await?;
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_never_changes_an_own_field_that_holds_people_whatever_is_stored() -> Outcome
{
    let world = World::new().await?;
    let people = a_field(&world, "Owner", FieldType::Person, &[]).await?;
    let given = handed_with(&world, ticking(&["title"])).await?;
    // A hand-over stored before the check existed names the field: the kind still refuses it.
    world.store.lock().state.work.hand_overs[0]
        .fields
        .push(people.id.to_string());
    let version = issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    let mut change = ChangeIssue::of(&given.key, given.issue.to_string(), version);
    change.fields.insert(
        people.id.to_string(),
        json!(given.team.person.id.to_string()),
    );
    let refusal = change_issue(&world.ctx(), change).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_edits_and_deletes_a_comment_only_when_it_was_given_comments() -> Outcome {
    let world = World::new().await?;
    let without = handed_with(&world, ticking(&["title"])).await?;
    let comment = add_comment(
        &world.ctx(),
        AddComment {
            session_token: without.team.session_token.clone(),
            issue: without.issue.to_string(),
            text: "The person wrote this".to_owned(),
        },
    )
    .await?
    .into_value();
    // The agent acts as its person, whose comment this is; the hand-over did not give comments.
    let refusal = delete_comment(
        &world.ctx(),
        DeleteComment {
            session_token: without.key.clone(),
            comment: comment.id,
            version: None,
        },
    )
    .await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    // Its own comment, the summary of an attempt, is another thing: it is written, not edited.
    report_attempt(&world.ctx(), report(&without.key, without.issue, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&without.key, without.issue)).await?;
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_with_comments_given_deletes_the_comment() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, ticking(&["comments"])).await?;
    let comment = add_comment(
        &world.ctx(),
        AddComment {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
            text: "Written by the person".to_owned(),
        },
    )
    .await?
    .into_value();
    delete_comment(
        &world.ctx(),
        DeleteComment {
            session_token: given.key.clone(),
            comment: comment.id,
            version: None,
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_33_an_agent_sees_only_its_own_work_on_an_issue() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    // A second agent of the same person has work on the same issue.
    let other = an_agent(&world, &given.team.session_token, "Helper").await?;
    let other_key = key_of(&world, &given.team.session_token, other).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, other, &[given.issue]),
    )
    .await?;
    let ask = |session: &str| AgentWorkQuery {
        session_token: session.to_owned(),
        issue: given.issue.to_string(),
    };
    let person = issue_agent_work(&world.ctx(), ask(&given.team.session_token))
        .await?
        .into_value();
    assert_eq!(person.work.len(), 2, "the person sees the work of both");
    for (key, agent) in [(&given.key, given.agent), (&other_key, other)] {
        let own = issue_agent_work(&world.ctx(), ask(key)).await?.into_value();
        assert_eq!(own.work.len(), 1);
        assert_eq!(own.work[0].agent, agent);
        assert_eq!(own.worked_on_by.len(), 1);
        assert_eq!(own.worked_on_by[0].agent, agent);
    }
    Ok(())
}

/// A team member with an assignment in each of two projects, and an agent with a hand-over of an
/// issue in each.
struct TwoProjects {
    key: String,
    team: crate::auth::Started,
    in_first: IssueId,
    in_second: IssueId,
    first: AssignmentId,
    second: AssignmentId,
}

async fn two_projects(world: &World) -> Result<TwoProjects, AppError> {
    let first = a_project(world, "ONE").await.map_err(AppError::Internal)?;
    let second = a_project(world, "TWO").await.map_err(AppError::Internal)?;
    let team = world.person("team@example.test").await?;
    let role = starting_role(world, StartingRole::TeamMember)
        .await
        .map_err(AppError::Internal)?;
    let mut rows = Vec::new();
    for project in [first, second] {
        rows.push(
            assign_role(
                &world.ctx(),
                AssignRole {
                    session_token: token(world),
                    subject: Subject::Person(team.person.id),
                    given: Given::Role(role),
                    project: Some(project),
                    applies_to: bb_core::access::AppliesTo::ProjectAndSubProjects,
                },
            )
            .await?
            .into_value()
            .id,
        );
    }
    let agent = an_agent(world, &team.session_token, "Fixer").await?;
    let key = key_of(world, &team.session_token, agent).await?;
    let in_first = an_issue(world, first).await.map_err(AppError::Internal)?;
    let in_second = an_issue(world, second).await.map_err(AppError::Internal)?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&team.session_token, agent, &[in_first, in_second]),
    )
    .await?;
    Ok(TwoProjects {
        key,
        team,
        in_first,
        in_second,
        first: rows[0],
        second: rows[1],
    })
}

/// The events that told an agent was stopped, and the notifications of them for the person.
async fn stops_told(
    world: &World,
    person: bb_core::id::UserId,
) -> Result<(usize, usize), AppError> {
    world.ctx().catch_up_notifications().await?;
    let shared = world.store.lock();
    let state = &shared.state;
    let events = shared
        .events
        .iter()
        .filter(|event| event.entity.kind.as_str() == RecordKind::Agent.code())
        .filter(|event| {
            event
                .changes
                .iter()
                .any(|change| change.field == event_member::WORK_STATE)
        })
        .count();
    let told = state
        .notifications
        .iter()
        .filter(|row| row.recipient == person && row.kind == NotificationKind::AgentStopped)
        .count();
    Ok((events, told))
}

#[tokio::test]
async fn ai_33_losing_one_project_cuts_the_work_there_and_stops_nothing() -> Outcome {
    let world = World::new().await?;
    let given = two_projects(&world).await?;
    remove_assignment(
        &world.ctx(),
        RemoveAssignment {
            session_token: token(&world),
            assignment: given.first,
        },
    )
    .await?;
    // The agent reaches the issue of the project its person still has, and not the other.
    issue(&world.ctx(), query(&given.key, given.in_second)).await?;
    let gone = issue(&world.ctx(), query(&given.key, given.in_first)).await;
    assert!(refused(&gone.unwrap_err(), ErrorCode::NotFound));
    let stored = world.store.lock().state.work.clone();
    assert!(
        stored.works.iter().all(|work| work.state.is_open()),
        "the work is cut, not stopped"
    );
    assert_eq!(stored.hand_overs[0].state, GivingState::Open);
    assert_eq!(stops_told(&world, given.team.person.id).await?, (0, 0));
    Ok(())
}

#[tokio::test]
async fn ai_33_losing_every_project_stops_the_work_once_and_tells_the_person_once() -> Outcome {
    let world = World::new().await?;
    let given = two_projects(&world).await?;
    for row in [given.first, given.second] {
        remove_assignment(
            &world.ctx(),
            RemoveAssignment {
                session_token: token(&world),
                assignment: row,
            },
        )
        .await?;
    }
    // The agent's next requests are refused, and the first of them stops the work.
    for _ in 0..3 {
        let refusal = issue(&world.ctx(), query(&given.key, given.in_first)).await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::NotSignedIn));
    }
    let stored = world.store.lock().state.work.clone();
    assert!(
        stored
            .works
            .iter()
            .all(|work| work.state == AgentWorkState::Stopped)
    );
    assert_eq!(stored.hand_overs[0].state, GivingState::Ended);
    assert_eq!(
        stops_told(&world, given.team.person.id).await?,
        (1, 1),
        "one event and one notification, however often the agent asks again"
    );
    Ok(())
}

#[tokio::test]
async fn ai_33_work_that_ran_out_is_not_reached_even_before_the_job_ends_it() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    issue(&world.ctx(), query(&given.key, given.issue)).await?;
    world.clock.advance(15 * 24 * 3600);
    let gone = issue(&world.ctx(), query(&given.key, given.issue)).await;
    assert!(refused(&gone.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

/// The work of the hand-over, ready for review.
async fn ready_for_review(world: &World) -> Result<Handed, AppError> {
    let given = handed(world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    Ok(given)
}

#[tokio::test]
async fn ai_35_two_reviews_at_the_same_moment_give_one_answer_and_one_conflict() -> Outcome {
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    let work = show(&world, &given).await?;
    let (first, second) = (world.ctx(), world.ctx());
    let (one, two) = tokio::join!(
        review_work(
            &first,
            review(&given.team.session_token, &work, ReviewAnswer::Accept)
        ),
        review_work(
            &second,
            review(&given.team.session_token, &work, ReviewAnswer::Accept)
        )
    );
    assert_eq!(u8::from(one.is_ok()) + u8::from(two.is_ok()), 1);
    let lost = one.err().or(two.err()).ok_or("both won")?;
    assert!(
        matches!(lost, AppError::Conflict(_))
            || matches!(lost, AppError::Stale(_))
            || refused(&lost, ErrorCode::VersionChanged)
            || refused(&lost, ErrorCode::AgentWorkNotReady),
        "{lost:?}"
    );
    assert_eq!(world.store.lock().state.work.reviews.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_35_two_ends_of_one_attempt_write_one_summary() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    let (first, second) = (world.ctx(), world.ctx());
    let (one, two) = tokio::join!(
        end_attempt(&first, ready(&given.key, given.issue)),
        end_attempt(&second, ready(&given.key, given.issue))
    );
    assert_eq!(u8::from(one.is_ok()) + u8::from(two.is_ok()), 1);
    let comments = world
        .store
        .lock()
        .events
        .iter()
        .filter(|event| event.entity.kind.as_str() == RecordKind::Comment.code())
        .count();
    assert_eq!(comments, 1, "one summary comment");
    Ok(())
}

#[tokio::test]
async fn ai_35_looking_at_work_counts_its_version_up_and_tells_nobody_again() -> Outcome {
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    world.ctx().catch_up_notifications().await?;
    let before = show(&world, &given).await?;
    let told = |world: &World| {
        world
            .store
            .lock()
            .state
            .notifications
            .iter()
            .filter(|row| row.kind == NotificationKind::ReadyForReview)
            .count()
    };
    assert_eq!(told(&world), 1);
    let after = look_at_work(
        &world.ctx(),
        crate::agent_work::LookAtWork {
            session_token: given.team.session_token.clone(),
            work: before.id,
        },
    )
    .await?
    .into_value();
    assert_eq!(after.version, before.version + 1);
    world.ctx().catch_up_notifications().await?;
    assert_eq!(told(&world), 1, "a look is not a new result to review");
    Ok(())
}

/// How many reads of the store a look at the agent work of an issue takes, with this many agents
/// working on the issue.
async fn reads_of_a_look_with(agents: usize) -> Result<usize, AppError> {
    let world = World::new().await.map_err(AppError::Internal)?;
    let given = handed(&world).await?;
    for number in 0..agents.saturating_sub(1) {
        let more = an_agent(
            &world,
            &given.team.session_token,
            &format!("Helper {number}"),
        )
        .await?;
        hand_over(
            &world.ctx(),
            plain_hand_over(&given.team.session_token, more, &[given.issue]),
        )
        .await?;
    }
    let before = world.store.lock().reads;
    let seen = issue_agent_work(
        &world.ctx(),
        AgentWorkQuery {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await?
    .into_value();
    assert_eq!(seen.work.len(), agents);
    Ok(world.store.lock().reads - before)
}

#[tokio::test]
async fn perf_05_the_agent_work_of_an_issue_costs_the_same_reads_for_one_agent_as_for_five()
-> Outcome {
    assert_eq!(
        reads_of_a_look_with(1).await?,
        reads_of_a_look_with(5).await?
    );
    Ok(())
}

/// The input of a new issue under `parent`, as the agent with this key sends it.
async fn new_under(
    world: &World,
    given: &Handed,
    parent: Option<IssueId>,
) -> Result<CreateIssue, AppError> {
    Ok(CreateIssue {
        session_token: given.key.clone(),
        project: given.project.to_string(),
        issue_type: a_type(world).await.map_err(AppError::Internal)?,
        title: "A step of the work".to_owned(),
        description: None,
        status: None,
        priority: None,
        assigned_to: None,
        estimated_time: None,
        fields: std::collections::BTreeMap::new(),
        parent: parent.map(|issue| issue.to_string()),
    })
}

#[tokio::test]
async fn ai_31_an_agent_creates_a_sub_issue_only_under_the_issue_it_was_handed() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::CreateSubIssues];
    })
    .await?;
    let sub = create_issue(
        &world.ctx(),
        new_under(&world, &given, Some(given.issue)).await?,
    )
    .await?
    .into_value();
    let parents = world.store.lock().state.parents.clone();
    assert!(
        parents
            .iter()
            .any(|row| row.child == sub.id && row.parent == given.issue),
        "the new issue sits under the handed one"
    );
    // A tick for sub-issues gives no top-level issue anywhere.
    let top = create_issue(&world.ctx(), new_under(&world, &given, None).await?).await;
    assert!(refused(&top.unwrap_err(), ErrorCode::Forbidden));
    // Under an issue it was not handed, it does not even see the parent.
    let other = an_issue(&world, given.project)
        .await
        .map_err(AppError::Internal)?;
    let elsewhere = create_issue(&world.ctx(), new_under(&world, &given, Some(other)).await?).await;
    assert!(refused(&elsewhere.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn ai_31_a_new_issue_of_an_agent_carries_only_members_the_hand_over_covers() -> Outcome {
    let world = World::new().await?;
    let people = a_field(&world, "Owner", FieldType::Person, &[]).await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::CreateSubIssues];
    })
    .await?;
    let mut refusals = Vec::new();
    let mut with_people = new_under(&world, &given, Some(given.issue)).await?;
    with_people.assigned_to = Some(vec![given.team.person.id]);
    refusals.push(with_people);
    let mut with_field = new_under(&world, &given, Some(given.issue)).await?;
    with_field.fields.insert(
        people.id.to_string(),
        json!(given.team.person.id.to_string()),
    );
    refusals.push(with_field);
    let mut with_status = new_under(&world, &given, Some(given.issue)).await?;
    with_status.status = Some(bb_core::workflow::starter::in_progress());
    refusals.push(with_status);
    let mut with_priority = new_under(&world, &given, Some(given.issue)).await?;
    with_priority.priority = Some(bb_core::issue::Priority::High);
    refusals.push(with_priority);
    for input in refusals {
        let refusal = create_issue(&world.ctx(), input).await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    }
    Ok(())
}

#[tokio::test]
async fn ai_31_without_the_tick_an_agent_creates_no_sub_issue() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let refusal = create_issue(
        &world.ctx(),
        new_under(&world, &given, Some(given.issue)).await?,
    )
    .await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn ai_35_a_throw_away_that_cannot_undo_stops_with_an_error_and_leaves_the_work() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, ticking(&["title"])).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    let version = issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    let mut change = ChangeIssue::of(&given.key, given.issue.to_string(), version);
    change.title = Some("Changed by the attempt".to_owned());
    change_issue(&world.ctx(), change).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    let work = show(&world, &given).await?;
    // Whatever is looked at from now on has changed again since the last look.
    world.store.lock().state.later_changes_keep_coming = true;
    let thrown = review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::ThrowAway),
    )
    .await;
    let error = thrown.err().ok_or("the work was thrown away")?;
    assert!(
        refused(&error, ErrorCode::VersionChanged) || matches!(error, AppError::Stale(_)),
        "{error:?}"
    );
    world.store.lock().state.later_changes_keep_coming = false;
    let after = show(&world, &given).await?;
    assert_ne!(
        after.state,
        AgentWorkState::Stopped,
        "never stopped with changes left"
    );
    assert!(after.reviews.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_35_a_throw_away_and_an_accept_with_a_status_at_the_same_moment_give_one_answer()
-> Outcome {
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    let work = show(&world, &given).await?;
    let mut accept = review(&given.team.session_token, &work, ReviewAnswer::Accept);
    accept.status = Some(bb_core::workflow::starter::in_progress());
    let throw = review(&given.team.session_token, &work, ReviewAnswer::ThrowAway);
    let (first, second) = (world.ctx(), world.ctx());
    let (one, two) = tokio::join!(review_work(&first, accept), review_work(&second, throw));
    assert_eq!(u8::from(one.is_ok()) + u8::from(two.is_ok()), 1);
    assert_eq!(world.store.lock().state.work.reviews.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_35_an_agent_cannot_ask_while_its_result_waits_for_the_review() -> Outcome {
    use crate::agent_work::{AskQuestion, ask_question};
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    let asked = ask_question(
        &world.ctx(),
        AskQuestion {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            text: "One more thing?".to_owned(),
        },
    )
    .await;
    assert!(refused(&asked.unwrap_err(), ErrorCode::AgentWorkNotReady));
    assert_eq!(
        show(&world, &given).await?.state,
        AgentWorkState::ReadyForReview
    );
    Ok(())
}

#[tokio::test]
async fn ai_31_a_new_issue_under_a_parent_is_judged_like_the_change_of_a_parent() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "DEEP").await?;
    let issue_type = a_type(&world).await.map_err(AppError::Internal)?;
    let make = |parent: Option<IssueId>| CreateIssue {
        session_token: token(&world),
        project: project.to_string(),
        issue_type,
        title: "A step down".to_owned(),
        description: None,
        status: None,
        priority: None,
        assigned_to: None,
        estimated_time: None,
        fields: std::collections::BTreeMap::new(),
        parent: parent.map(|issue| issue.to_string()),
    };
    let mut deepest = create_issue(&world.ctx(), make(None))
        .await?
        .into_value()
        .id;
    let mut levels = 0;
    let refusal = loop {
        match create_issue(&world.ctx(), make(Some(deepest))).await {
            Ok(next) => {
                deepest = next.into_value().id;
                levels += 1;
                assert!(
                    levels <= bb_core::relation::SUB_ISSUE_DEPTH_MAX + 1,
                    "no end"
                );
            }
            Err(error) => break error,
        }
    };
    assert!(
        refused(&refusal, ErrorCode::ParentTooDeep),
        "a chain too long is its own refusal: {refusal:?}"
    );
    assert!(levels >= 1);
    Ok(())
}

#[tokio::test]
async fn ai_35_two_accepts_by_itself_on_two_pieces_of_one_hand_over_close_both_and_end_it()
-> Outcome {
    use super::agent_work::{Ground, ground};
    use super::agent_work_review::check;
    use super::agent_work_runs::allow_accepting;
    use crate::agent_work::attach_result;
    let world = World::new().await?;
    let Ground {
        project,
        team,
        agent,
        key,
        issue: first,
    } = ground(&world).await?;
    let second = an_issue(&world, project)
        .await
        .map_err(AppError::Internal)?;
    allow_accepting(&world, project).await?;
    let mut asked = plain_hand_over(&team.session_token, agent, &[first, second]);
    asked.accept_by_itself = true;
    hand_over(&world.ctx(), asked).await?;
    for issue in [first, second] {
        report_attempt(&world.ctx(), report(&key, issue, None, 1)).await?;
        attach_result(&world.ctx(), check(&key, issue, CheckResult::Passed)).await?;
    }
    let (one, two) = (world.ctx(), world.ctx());
    let (a, b) = tokio::join!(
        end_attempt(&one, ready(&key, first)),
        end_attempt(&two, ready(&key, second))
    );
    a?;
    b?;
    let stored = world.store.lock().state.work.clone();
    assert!(stored.works.iter().all(|work| work.closed));
    assert_eq!(stored.hand_overs[0].state, GivingState::Ended);
    Ok(())
}

/// Tells the fake store that counting the version of a hand-over up finds it changed this many
/// more times.
fn touches_fail(world: &World, times: usize) {
    let times = u32::try_from(times).unwrap_or(u32::MAX);
    world
        .store
        .lock()
        .state
        .touches_fail
        .store(times, std::sync::atomic::Ordering::SeqCst);
}

#[tokio::test]
async fn ai_35_a_review_closes_again_when_a_sibling_closed_first_once() -> Outcome {
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    let work = show(&world, &given).await?;
    touches_fail(&world, 1);
    review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::Accept),
    )
    .await?;
    let stored = world.store.lock().state.work.clone();
    assert!(stored.works[0].closed);
    assert_eq!(stored.hand_overs[0].state, GivingState::Ended);
    assert_eq!(stored.reviews.len(), 1, "one review, written once");
    Ok(())
}

#[tokio::test]
async fn ai_35_a_review_that_always_finds_the_hand_over_changed_leaves_the_work_open() -> Outcome {
    let world = World::new().await?;
    let given = ready_for_review(&world).await?;
    let work = show(&world, &given).await?;
    touches_fail(&world, crate::agent_work::TRIES);
    let lost = review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::Accept),
    )
    .await;
    let error = lost.err().ok_or("the review went through")?;
    assert!(
        refused(&error, ErrorCode::VersionChanged) || matches!(error, AppError::Stale(_)),
        "{error:?}"
    );
    let stored = world.store.lock().state.work.clone();
    assert!(!stored.works[0].closed, "the work stays open");
    assert!(stored.reviews.is_empty(), "no review was written");
    assert_eq!(stored.hand_overs[0].state, GivingState::Open);
    // The person asks again, and it goes through.
    let again = show(&world, &given).await?;
    review_work(
        &world.ctx(),
        review(&given.team.session_token, &again, ReviewAnswer::Accept),
    )
    .await?;
    assert!(world.store.lock().state.work.works[0].closed);
    Ok(())
}
