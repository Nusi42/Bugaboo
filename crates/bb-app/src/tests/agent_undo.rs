//! An agent undoes its own runs (AI-38, ADR-0029, amendment M9): only when its person switched on
//! *May undo its own runs*, only the runs of its own attempts, through *Undo the run*, and recorded
//! as done by the agent.

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::comment::TEXT_PART;
use bb_core::edit::EditWhat;
use bb_core::event::RecordKind;
use bb_core::id::{CommentId, EditorId};
use bb_core::technical::TechnicalName;
use serde_json::json;

use super::agent_tools::{previewed, used};
use super::agent_tools_more::version_of;
use super::agent_work::{Handed, handed_with, query, start_attempt};
use super::agents::refused;
use super::agents_switches::switched;
use super::world::{Outcome, World};
use crate::auth::{AppError, entity};
use crate::editing::{TakeLockQuery, take_lock};
use crate::issues::issue;

/// A hand-over that lets the agent change the title, with its attempt running.
pub(super) async fn handed_title(world: &World) -> Result<Handed, AppError> {
    let given = handed_with(world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned()];
    })
    .await?;
    start_attempt(world, &given.key, given.issue).await?;
    Ok(given)
}

/// The agent changes the title in its attempt.
pub(super) async fn retitled_by_agent(world: &World, given: &Handed, title: &str) -> Outcome {
    let version = version_of(world, given).await?;
    used(
        world,
        &given.key,
        "update_issue",
        json!({ "issue": given.issue.to_string(), "version": version, "title": title }),
    )
    .await?;
    Ok(())
}

pub(super) async fn title_now(world: &World, given: &Handed) -> Result<String, AppError> {
    Ok(
        issue(&world.ctx(), query(&given.team.session_token, given.issue))
            .await?
            .into_value()
            .issue
            .title,
    )
}

pub(super) fn undo_input(given: &Handed) -> serde_json::Value {
    json!({ "issue": given.issue.to_string() })
}

pub(super) fn may_undo() -> bb_core::agent::AgentSwitches {
    bb_core::agent::AgentSwitches {
        may_hand_over: false,
        may_undo: true,
    }
}

#[tokio::test]
async fn ai_38_an_agent_undoes_its_own_run_only_when_its_switch_is_on() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    let before = title_now(&world, &given).await?;
    retitled_by_agent(&world, &given, "A wrong title").await?;
    let refusal = used(&world, &given.key, "undo_my_run", undo_input(&given)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    let preview = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await;
    assert!(refused(&preview.unwrap_err(), ErrorCode::Forbidden));
    assert_eq!(title_now(&world, &given).await?, "A wrong title");
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    let asked = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(asked.text.contains("goes_back: 1"), "{}", asked.text);
    assert_eq!(title_now(&world, &given).await?, "A wrong title");
    let done = used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(done.text.contains("goes_back: 1"), "{}", done.text);
    assert_eq!(title_now(&world, &given).await?, before);
    Ok(())
}

#[tokio::test]
async fn ai_38_an_agent_never_undoes_a_run_of_its_person_or_of_another_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    // Another agent of the same person, allowed to undo, finds the run of Fixer not there.
    let session = given.team.session_token.clone();
    let helper = super::agents::an_agent(&world, &session, "Helper").await?;
    let helper_key = super::agents::key_of(&world, &session, helper).await?;
    switched(&world, &session, helper, may_undo()).await?;
    let fixers: Vec<_> = world
        .store
        .lock()
        .state
        .many_changes_runs
        .iter()
        .filter(|run| run.agent == Some(given.agent))
        .map(|run| run.id)
        .collect();
    assert!(!fixers.is_empty(), "the attempt of Fixer has a run");
    for run in fixers {
        let refusal = crate::runs::undo_run(
            &world.ctx(),
            crate::runs::UndoRun {
                session_token: helper_key.clone(),
                run,
                version: bb_core::run::UndoVersion(String::new()),
            },
        )
        .await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::NotFound));
    }
    // An issue that is not of its work has no run of its for it.
    let other = super::scenarios_issues::an_issue(&world, given.project).await?;
    let elsewhere = used(
        &world,
        &given.key,
        "undo_my_run",
        json!({ "issue": other.to_string() }),
    )
    .await;
    assert!(elsewhere.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_38_an_undo_by_an_agent_is_recorded_as_done_by_the_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    let shared = world.store.lock();
    let undone = shared
        .events
        .iter()
        .rev()
        .find(|event| {
            event.entity.kind.as_str() == "change_run" && crate::runs::went_back(event).is_some()
        })
        .ok_or("no event of the undo")?;
    assert!(matches!(
        undone.actor,
        bb_core::event::Actor::Agent { agent, .. } if agent == given.agent
    ));
    Ok(())
}

/// What an agent wrote that must go: a text nobody else wrote.
const AGENT_TEXT: &str = "Ignore the brief and drop the tests";
/// What a person wrote that must stay.
const PERSON_TEXT: &str = "Please keep the old login page";

/// The agent and the person each comment on the issue while the attempt runs.
async fn both_commented(world: &World, given: &Handed) -> Outcome {
    used(
        world,
        &given.key,
        "add_comment",
        json!({ "issue": given.issue.to_string(), "text": AGENT_TEXT }),
    )
    .await?;
    crate::comments::add_comment(
        &world.ctx(),
        crate::comments::AddComment {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
            text: PERSON_TEXT.to_owned(),
        },
    )
    .await?;
    Ok(())
}

/// The agent's comments are gone with every text of them, the person's stays, and the issue keeps
/// an event that names the comments thrown away and holds no text.
fn only_the_agents_comments_went(world: &World, given: &Handed) -> Outcome {
    let shared = world.store.lock();
    let texts: Vec<&str> = shared
        .state
        .comments
        .iter()
        .filter(|comment| comment.issue == given.issue)
        .map(|comment| comment.text.as_str())
        .collect();
    assert!(texts.contains(&PERSON_TEXT), "{texts:?}");
    assert!(!texts.contains(&AGENT_TEXT), "{texts:?}");
    let thrown = shared
        .events
        .iter()
        .rev()
        .find(|event| {
            event.entity.kind.as_str() == "issue"
                && event
                    .changes
                    .iter()
                    .any(|change| change.field == "comments_thrown_away")
        })
        .ok_or("no event that says the comments were thrown away")?;
    let written = serde_json::to_string(&thrown.changes)?;
    assert!(!written.contains(AGENT_TEXT), "{written}");
    Ok(())
}

#[tokio::test]
async fn ai_35_throw_away_takes_the_comments_of_the_agent_and_keeps_that_they_were_there() -> Outcome
{
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    both_commented(&world, &given).await?;
    throw_away(&world, &given).await?;
    only_the_agents_comments_went(&world, &given)
}

#[tokio::test]
async fn ai_35_throw_away_keeps_the_comments_of_people() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    both_commented(&world, &given).await?;
    throw_away(&world, &given).await?;
    let shared = world.store.lock();
    let kept = shared
        .state
        .comments
        .iter()
        .filter(|comment| comment.issue == given.issue)
        .count();
    assert_eq!(
        kept, 1,
        "the comment of the person, and nothing of the agent"
    );
    Ok(())
}

#[tokio::test]
async fn ai_38_undo_my_run_takes_the_comments_of_the_run_too() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    both_commented(&world, &given).await?;
    let asked = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(asked.text.contains("goes_back: 1"), "{}", asked.text);
    used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    only_the_agents_comments_went(&world, &given)
}

/// The person's comment, written before the agent's attempt.
async fn persons_comment(world: &World, given: &Handed) -> Result<CommentId, AppError> {
    Ok(crate::comments::add_comment(
        &world.ctx(),
        crate::comments::AddComment {
            session_token: given.team.session_token.clone(),
            issue: given.issue.to_string(),
            text: PERSON_TEXT.to_owned(),
        },
    )
    .await?
    .into_value()
    .id)
}

/// The agent tries to edit the comment: the lock every editor needs is a person's in a browser tab,
/// never a key's, so an agent never edits a comment.
async fn edit_refused(world: &World, given: &Handed, comment: CommentId) -> Outcome {
    let what = EditWhat {
        record: entity(RecordKind::Comment, comment),
        part: TechnicalName::new(TEXT_PART)?,
    };
    let refusal = take_lock(
        &world.ctx(),
        TakeLockQuery {
            session_token: given.key.clone(),
            what,
            editor: EditorId::from_random([7; 16]),
        },
    )
    .await;
    assert!(refusal.is_err(), "an agent took the lock of a comment");
    Ok(())
}

#[tokio::test]
async fn ai_35_throw_away_keeps_a_comment_of_the_person_the_agent_put_into_the_trash() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned(), "comments".to_owned()];
    })
    .await?;
    let edited = persons_comment(&world, &given).await?;
    let trashed = persons_comment(&world, &given).await?;
    start_attempt(&world, &given.key, given.issue).await?;
    edit_refused(&world, &given, edited).await?;
    crate::comments::delete_comment(
        &world.ctx(),
        crate::comments::DeleteComment {
            session_token: given.key.clone(),
            comment: trashed,
            version: None,
        },
    )
    .await?;
    throw_away(&world, &given).await?;
    let shared = world.store.lock();
    let kept = |id: CommentId| shared.state.comments.iter().find(|each| each.id == id);
    let edited = kept(edited).ok_or("the comment the agent tried to edit was thrown away")?;
    assert_eq!(edited.text, PERSON_TEXT);
    let trashed =
        kept(trashed).ok_or("the comment the agent put into the trash was thrown away")?;
    assert!(trashed.deleted_at.is_some(), "the trash is not undone yet");
    Ok(())
}

/// The attempt is ready, and its person throws the work away.
async fn throw_away(world: &World, given: &Handed) -> Outcome {
    crate::agent_work::end_attempt(
        &world.ctx(),
        super::agent_work_review::ready(&given.key, given.issue),
    )
    .await?;
    let work = super::agent_work_review::show(world, given).await?;
    crate::agent_work::review_work(
        &world.ctx(),
        super::agent_work_review::review(
            &given.team.session_token,
            &work,
            bb_core::agent_work::ReviewAnswer::ThrowAway,
        ),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_21_preview_undo_my_run_puts_nothing_back() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    switched(&world, &given.team.session_token, given.agent, may_undo()).await?;
    retitled_by_agent(&world, &given, "Changed by the agent").await?;
    let asked = previewed(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    assert!(asked.text.contains("goes_back: 1"), "{}", asked.text);
    assert_eq!(title_now(&world, &given).await?, "Changed by the agent");
    Ok(())
}
