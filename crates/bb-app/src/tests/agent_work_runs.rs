//! Throw away, the move of the status after an accept by itself, and the summary comment
//! (AI-33, AI-35, ADR-0029, decisions 4, 6 and 7).

use bb_core::agent_work::{AgentWorkState, CheckResult, HandOverPower, ReviewAnswer};
use bb_core::id::{IssueId, IssueTypeId, ProjectId};
use bb_core::message::Message;
use bb_policy::StartingRole;

use super::agent_work::{Handed, handed, plain_hand_over, query};
use super::agent_work_review::{check, ready, report, review, show};
use super::agents::{an_agent, key_of};
use super::runs::open;
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{go, member, status_of};
use super::world::{Outcome, World};
use crate::agent_work::{attach_result, end_attempt};
use crate::agent_work::{hand_over, report_attempt, review_work};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, IssueTypesQuery, change_issue, issue, issue_types};
use crate::runs::{RunQuery, get_run, preview_undo_run};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_core::settings::SettingValue;

/// An agent of a team member that may change the title, handed one issue.
async fn titled(world: &World, issue_id: IssueId, project: ProjectId) -> Result<Handed, AppError> {
    let team = member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let agent = an_agent(world, &team.session_token, "Fixer").await?;
    let key = key_of(world, &team.session_token, agent).await?;
    let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
    asked.powers = vec![HandOverPower::ChangeFields];
    asked.fields = vec!["title".to_owned()];
    asked.accept_by_itself = true;
    let hand_over = hand_over(&world.ctx(), asked).await?.into_value();
    Ok(Handed {
        project,
        team,
        agent,
        key,
        issue: issue_id,
        hand_over,
    })
}

async fn allow_accepting(world: &World, project: ProjectId) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope: SettingsScope::Project(project),
            key: "agents.accept-by-itself".to_owned(),
            value: SettingValue::Bool(true),
        },
    )
    .await?;
    Ok(())
}

async fn title_of(world: &World, session: &str, id: IssueId) -> Result<String, AppError> {
    Ok(issue(&world.ctx(), query(session, id))
        .await?
        .into_value()
        .issue
        .title)
}

#[tokio::test]
async fn ai_35_throw_away_undoes_every_attempt_in_one_step() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    let issue_id = an_issue(&world, project).await?;
    let given = titled(&world, issue_id, project).await?;
    let before = title_of(&world, &given.team.session_token, issue_id).await?;
    // The agent starts an attempt, changes the title and says it is ready.
    report_attempt(&world.ctx(), report(&given.key, issue_id, None, 5)).await?;
    let version = issue(&world.ctx(), query(&given.key, issue_id))
        .await?
        .into_value()
        .issue
        .version;
    let title = ChangeIssue {
        session_token: given.key.clone(),
        issue: issue_id.to_string(),
        version,
        issue_type: None,
        title: Some("Changed by an agent".to_owned()),
        status: None,
        priority: None,
        estimated_time: None,
        time_left: None,
        assigned_to: None,
        fields: std::collections::BTreeMap::default(),
    };
    change_issue(&world.ctx(), title).await?;
    assert_eq!(
        title_of(&world, &given.team.session_token, issue_id).await?,
        "Changed by an agent"
    );
    end_attempt(&world.ctx(), ready(&given.key, issue_id)).await?;
    // Throw away: the title goes back, the hand-over ends, and the agent has lost the issue.
    let work = show(&world, &given).await?;
    let thrown = review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::ThrowAway),
    )
    .await?
    .into_value();
    assert_eq!(thrown.state, AgentWorkState::Stopped);
    assert!(!thrown.open);
    assert_eq!(
        title_of(&world, &given.team.session_token, issue_id).await?,
        before
    );
    Ok(())
}

async fn bug_type(world: &World) -> Result<IssueTypeId, AppError> {
    let page = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            project: None,
            session_token: token(world),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    let bug = Message::from(bb_core::message::ISSUE_TYPE_BUG).render();
    page.items
        .into_iter()
        .find(|found| found.name == bug)
        .map(|found| found.id)
        .ok_or_else(|| AppError::Internal("no issue type Bug".to_owned()))
}

/// An issue of the type, in the status the workflow reaches after the given ones.
async fn issue_of_type(
    world: &World,
    project: ProjectId,
    issue_type: IssueTypeId,
    statuses: &[&str],
) -> Result<IssueId, AppError> {
    let id = crate::issues::create_issue(
        &world.ctx(),
        crate::issues::CreateIssue {
            session_token: token(world),
            project: project.to_string(),
            issue_type,
            title: "A bug".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
            estimated_time: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await?
    .into_value()
    .id;
    for status in statuses {
        let version = issue(&world.ctx(), query(&token(world), id))
            .await?
            .into_value()
            .issue
            .version;
        go(world, &token(world), id, version, status).await?;
    }
    Ok(id)
}

async fn finish_with_passed_check(world: &World, given: &Handed) -> Result<(), AppError> {
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    attach_result(
        &world.ctx(),
        check(&given.key, given.issue, CheckResult::Passed),
    )
    .await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    Ok(())
}

#[tokio::test]
async fn ai_35_after_an_accept_by_itself_the_issue_moves_to_the_one_done_status_the_workflow_allows()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    allow_accepting(&world, project).await?;
    // *Simple*: from *Open* exactly one status of the *Done* group follows.
    let issue_id = an_issue(&world, project).await?;
    let given = titled(&world, issue_id, project).await?;
    finish_with_passed_check(&world, &given).await?;
    assert_eq!(status_of(&world, &token(&world), issue_id).await?, "done");
    let work = show(&world, &given).await?;
    assert!(work.accepted_by_itself);
    assert!(!work.choose_next_status);
    Ok(())
}

#[tokio::test]
async fn ai_35_with_several_done_statuses_the_status_stays_and_the_person_is_told() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    allow_accepting(&world, project).await?;
    // *Bug*: from *Testing* both *Done* and *Won't fix* follow.
    let bug = bug_type(&world).await?;
    let issue_id = issue_of_type(&world, project, bug, &["in-progress", "testing"]).await?;
    let given = titled(&world, issue_id, project).await?;
    finish_with_passed_check(&world, &given).await?;
    assert_eq!(
        status_of(&world, &token(&world), issue_id).await?,
        "testing"
    );
    let work = show(&world, &given).await?;
    assert!(work.accepted_by_itself);
    assert!(work.choose_next_status, "Accepted; choose the next status");
    Ok(())
}

#[tokio::test]
async fn ai_33_a_ready_attempt_leaves_one_short_comment_written_by_an_agent() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    let issue_id = an_issue(&world, project).await?;
    let given = titled(&world, issue_id, project).await?;
    report_attempt(&world.ctx(), report(&given.key, issue_id, None, 5)).await?;
    end_attempt(&world.ctx(), ready(&given.key, issue_id)).await?;
    let shared = world.store.lock();
    assert_eq!(shared.state.comments.len(), 1);
    assert!(
        shared.state.comments[0].text.contains("/issues/"),
        "it links to the issue"
    );
    // The event names the agent, which is what the mark *Written by an agent* is set from.
    let written = shared
        .events
        .iter()
        .rev()
        .find(|event| event.entity.kind.as_str() == "comment")
        .ok_or("no event of a comment")?;
    assert!(matches!(
        written.actor,
        bb_core::event::Actor::Agent { agent, .. } if agent == given.agent
    ));
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_changes_nothing_its_hand_over_did_not_name_and_sees_only_its_work()
-> Outcome {
    use crate::issues::{CreateIssue, DeleteIssue, create_issue, delete_issue};
    use crate::search::{SearchQuery, SearchScope, search};

    let world = World::new().await?;
    let project = a_project(&world, "WEB").await?;
    let issue_id = an_issue(&world, project).await?;
    let other = an_issue(&world, project).await?;
    let given = titled(&world, issue_id, project).await?;
    // Deleting is never given; a new issue needs a sub-issue tick or a rule that creates.
    let refused_delete = delete_issue(
        &world.ctx(),
        DeleteIssue {
            session_token: given.key.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await;
    assert!(super::agents::refused(
        &refused_delete.unwrap_err(),
        bb_core::ErrorCode::Forbidden
    ));
    let made = create_issue(
        &world.ctx(),
        CreateIssue {
            session_token: given.key.clone(),
            project: project.to_string(),
            issue_type: super::scenarios_issues::a_type(&world).await?,
            title: "A sub-issue".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
            estimated_time: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await;
    assert!(super::agents::refused(
        &made.unwrap_err(),
        bb_core::ErrorCode::Forbidden
    ));
    // A list or a search of the agent is cut to its work; the person's is not.
    for (session, expected) in [
        (&given.key, Some(vec![issue_id])),
        (&given.team.session_token, None),
    ] {
        let _ = search(
            &world.ctx(),
            SearchQuery {
                session_token: session.clone(),
                text: String::new(),
                scope: SearchScope::Within(project),
                cursor: None,
                limit: 10,
                labels: false,
                time: false,
            },
        )
        .await?;
        let asked = world
            .store
            .lock()
            .searches
            .last()
            .cloned()
            .ok_or("no search")?;
        assert_eq!(asked.only, expected);
    }
    let _ = other;
    Ok(())
}

/// An agent that holds work reaches the run of its own attempt, and the runs of its person stay
/// closed to it, the preview and the undo too (AI-30, ADR-0029).
#[tokio::test]
async fn ai_30_an_agent_with_work_reaches_its_attempt_and_not_the_runs_of_its_person() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    let theirs = open(&world, &given.team.session_token, given.team.person.id).await?;
    let read = |session: &str, run| RunQuery {
        session_token: session.to_owned(),
        run,
    };
    let refusal = get_run(&world.ctx(), read(&given.key, theirs)).await;
    assert!(matches!(
        refusal,
        Err(AppError::Refused(bb_core::ErrorCode::NotFound))
    ));
    assert!(
        preview_undo_run(&world.ctx(), read(&given.key, theirs))
            .await
            .is_err()
    );
    let attempt = world
        .store
        .lock()
        .state
        .work
        .attempts
        .iter()
        .find_map(|attempt| attempt.run)
        .ok_or("the attempt has no run")?;
    get_run(&world.ctx(), read(&given.key, attempt)).await?;
    get_run(&world.ctx(), read(&given.team.session_token, attempt)).await?;
    Ok(())
}
