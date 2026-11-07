//! Runs against the in-memory store (AI-38): a run lists what was changed inside it, *Undo the run*
//! puts the old values back in one step, leaves what somebody changed afterwards, and is judged
//! again as the person who undoes it.

use bb_core::id::{IssueId, RunId};
use bb_core::issue::Priority;
use bb_core::run::{RunActor, RunKind, UndoSkipKind};
use bb_policy::StartingRole;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use crate::runs::{
    RunChangesQuery, RunQuery, RunStore, RunsOf, UndoRun, UndoView, preview_undo_run, run_changes,
    undo_run,
};

fn priority_change(session: &str, issue: IssueId, version: i64, priority: Priority) -> ChangeIssue {
    ChangeIssue {
        session_token: session.to_owned(),
        issue: issue.to_string(),
        version,
        issue_type: None,
        title: None,
        status: None,
        priority: Some(priority),
        estimated_time: None,
        time_left: None,
        assigned_to: None,
        fields: std::collections::BTreeMap::new(),
    }
}

async fn priority_of(world: &World, session: &str, id: IssueId) -> Result<Priority, AppError> {
    Ok(issue(
        &world.ctx(),
        IssueQuery {
            session_token: session.to_owned(),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .priority)
}

/// Opens a run of an app session for `person` and returns its ID.
async fn open(world: &World, session: &str, person: bb_core::id::UserId) -> Result<RunId, String> {
    let ctx = world.ctx();
    let actor = ctx
        .session_actor(session)
        .await
        .map_err(|error| error.to_string())?;
    let run = RunId::generate(&mut ctx.ids()).map_err(|error| error.to_string())?;
    let who = RunActor {
        person,
        app: Some("Claude Code".to_owned()),
        agent: None,
    };
    RunsOf::new(&ctx, &actor, session)
        .open_run(run, RunKind::AppSession, &who, ctx.now())
        .await
        .map_err(|error| error.to_string())?;
    Ok(run)
}

/// Changes the priority of an issue inside a run, as the person of the session.
async fn change_in_run(
    world: &World,
    session: &str,
    run: RunId,
    id: IssueId,
    version: i64,
    priority: Priority,
) -> Result<(), AppError> {
    let ctx = world.ctx();
    ctx.set_run(Some(run));
    change_issue(&ctx, priority_change(session, id, version, priority)).await?;
    Ok(())
}

async fn preview_of(world: &World, session: &str, run: RunId) -> Result<UndoView, AppError> {
    Ok(preview_undo_run(
        &world.ctx(),
        RunQuery {
            session_token: session.to_owned(),
            run,
        },
    )
    .await?
    .into_value())
}

async fn undo(
    world: &World,
    session: &str,
    run: RunId,
    preview: UndoView,
) -> Result<UndoView, AppError> {
    Ok(undo_run(
        &world.ctx(),
        UndoRun {
            session_token: session.to_owned(),
            run,
            version: preview.version,
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn ai_38_a_run_is_undone_in_one_step_and_skips_later_changes() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let issues = [
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    ];
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    let was = priority_of(&world, &admin, issues[0]).await?;
    for each in issues {
        change_in_run(&world, &admin, run, each, 1, Priority::Critical).await?;
    }
    // Somebody changes the second issue again afterwards, outside the run.
    change_issue(
        &world.ctx(),
        priority_change(&admin, issues[1], 2, Priority::Low),
    )
    .await?;
    let page = run_changes(
        &world.ctx(),
        RunChangesQuery {
            session_token: admin.clone(),
            run,
            after: None,
            limit: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.items[0].changes[0].field, "priority");
    let preview = preview_of(&world, &admin, run).await?;
    assert_eq!((preview.goes_back, preview.skipped), (2, 1));
    assert_eq!(preview.skips[0].kind, UndoSkipKind::ChangedSince);
    assert_eq!(
        preview.skips[0].issue.as_ref().map(|line| line.id),
        Some(issues[1])
    );
    let done = undo(&world, &admin, run, preview).await?;
    assert!(!done.changed_since_preview);
    assert_eq!((done.goes_back, done.skipped), (2, 1));
    assert_eq!(priority_of(&world, &admin, issues[0]).await?, was);
    assert_eq!(priority_of(&world, &admin, issues[2]).await?, was);
    // The change somebody made afterwards is left as it is.
    assert_eq!(priority_of(&world, &admin, issues[1]).await?, Priority::Low);
    Ok(())
}

#[tokio::test]
async fn ai_38_an_undo_with_an_old_version_writes_nothing_and_shows_the_newer_preview() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    change_in_run(&world, &admin, run, first, 1, Priority::Critical).await?;
    let preview = preview_of(&world, &admin, run).await?;
    change_in_run(&world, &admin, run, second, 1, Priority::Critical).await?;
    let before = world.store.event_count();
    let answer = undo(&world, &admin, run, preview).await?;
    assert!(answer.changed_since_preview);
    assert_eq!(answer.goes_back, 2);
    assert_eq!(world.store.event_count(), before);
    Ok(())
}

#[tokio::test]
async fn ai_38_an_undo_writes_new_events_and_keeps_the_history() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let events_before = world.store.event_count();
    let preview = preview_of(&world, &admin, run).await?;
    undo(&world, &admin, run, preview).await?;
    // New events, and the one of the run is still there with the run.
    assert!(world.store.event_count() > events_before);
    let events = world.store.lock().events.clone();
    assert_eq!(
        events.iter().filter(|event| event.run == Some(run)).count(),
        1
    );
    let reason = format!("undo-run:{run}");
    let undone: Vec<_> = events
        .iter()
        .filter(|event| event.reason.as_deref() == Some(reason.as_str()))
        .collect();
    assert_eq!(undone.len(), 1);
    assert_eq!(undone[0].run, None);
    Ok(())
}

#[tokio::test]
async fn ai_38_the_changes_of_a_run_show_only_what_the_reader_may_see() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (open_project, shut) = (
        a_project(&world, "OPEN").await?,
        a_project(&world, "SHUT").await?,
    );
    let (seen, hidden) = (
        an_issue(&world, open_project).await?,
        an_issue(&world, shut).await?,
    );
    let bob = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        open_project,
    )
    .await?;
    // A run of Bob's app, whose changes were made in both projects.
    let run = open(&world, &bob.session_token, bob.person.id).await?;
    change_in_run(&world, &admin, run, seen, 1, Priority::Critical).await?;
    change_in_run(&world, &admin, run, hidden, 1, Priority::Critical).await?;
    let page = run_changes(
        &world.ctx(),
        RunChangesQuery {
            session_token: bob.session_token.clone(),
            run,
            after: None,
            limit: None,
        },
    )
    .await?
    .into_value();
    let lines: Vec<_> = page.items.iter().map(|item| item.issue.id).collect();
    assert_eq!(lines, vec![seen]);
    // Somebody else's run is not there at all.
    let other = run_changes(
        &world.ctx(),
        RunChangesQuery {
            session_token: admin,
            run,
            after: None,
            limit: None,
        },
    )
    .await;
    assert!(matches!(
        other,
        Err(AppError::Refused(bb_core::ErrorCode::NotFound))
    ));
    Ok(())
}

#[tokio::test]
async fn ai_38_an_undo_is_judged_again_as_the_person_who_undoes_it() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "OPEN").await?;
    let id = an_issue(&world, project).await?;
    let bob = member(&world, "bob@example.test", StartingRole::Viewer, project).await?;
    let was = priority_of(&world, &admin, id).await?;
    // Bob's run changed an issue (made with the rights of an administrator, as a test setup); Bob
    // himself may not change issues, so his undo leaves it and says why.
    let run = open(&world, &bob.session_token, bob.person.id).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let preview = preview_of(&world, &bob.session_token, run).await?;
    let done = undo(&world, &bob.session_token, run, preview).await?;
    assert_eq!(done.goes_back, 0);
    assert_eq!(done.skips[0].kind, UndoSkipKind::NotAllowed);
    assert_ne!(priority_of(&world, &admin, id).await?, was);
    Ok(())
}

/// The scenario of the table of command use cases: undoing a run is one event of the run, and each
/// issue put back has its own (the run of this scenario changed nothing).
pub(crate) async fn undo_run_scenario(world: &World) -> super::scenarios::Added {
    let admin = token(world);
    let person = world
        .ctx()
        .session_actor(&admin)
        .await
        .map_err(|error| error.to_string())?
        .person
        .id;
    let run = open(world, &admin, person).await?;
    let preview = preview_of(world, &admin, run)
        .await
        .map_err(|error| error.to_string())?;
    super::scenarios::measured(
        &world.store,
        undo_run(
            &world.ctx(),
            UndoRun {
                session_token: admin,
                run,
                version: preview.version,
            },
        ),
    )
    .await
}

#[tokio::test]
async fn comp_09_a_comment_written_by_an_agent_carries_the_mark_in_the_api_and_the_history()
-> Outcome {
    use bb_core::history::HistoryTab;
    use bb_core::mark::WrittenBy;

    use super::agents::a_team_agent;
    use crate::comments::{AddComment, add_comment};
    use crate::history::{HistoryQuery, HistoryWhat, history};

    let world = World::new().await?;
    let (project, team, _, key) = a_team_agent(&world).await?;
    let id = an_issue(&world, project).await?;
    let comment = |session: &str, text: &str| AddComment {
        session_token: session.to_owned(),
        issue: id.to_string(),
        text: text.to_owned(),
    };
    let by_agent = add_comment(&world.ctx(), comment(&key, "Fixed by the agent"))
        .await?
        .into_value();
    let mark = by_agent.written_by.as_ref().ok_or("no mark")?;
    assert_eq!(mark.kind, WrittenBy::Agent);
    assert_eq!(mark.agent.as_deref(), Some("Fixer"));
    // The person's own comment has none: a mark is set from the actor, never asked for.
    let by_hand = add_comment(&world.ctx(), comment(&team.session_token, "By hand"))
        .await?
        .into_value();
    assert!(by_hand.written_by.is_none());
    // The history shows the same marks.
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: team.session_token.clone(),
            issue: id.to_string(),
            tab: HistoryTab::Comments,
            cursor: None,
            limit: 20,
        },
    )
    .await?
    .into_value();
    let marks: Vec<_> = page
        .items
        .iter()
        .filter_map(|row| match &row.what {
            HistoryWhat::Comment { comment } => Some(comment.written_by.as_ref().map(|m| m.kind)),
            _ => None,
        })
        .collect();
    assert_eq!(marks.len(), 2);
    assert!(marks.contains(&Some(WrittenBy::Agent)));
    assert!(marks.contains(&None));
    Ok(())
}
