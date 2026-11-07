//! What became of the changes of a run once it was undone (AI-38): who undid it, what each field
//! says, an undo that stopped half way, and what the undo leaves for lack of a right.

use bb_core::id::{IssueId, RunId};
use bb_core::issue::Priority;
use bb_core::run::UndoSkipKind;
use bb_policy::StartingRole;

use super::runs::{change_in_run, open, preview_of, priority_change, priority_of, undo};
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{assign, member, starting_role};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{IssueQuery, change_issue, issue};
use crate::runs::{RunChangesQuery, RunQuery, RunsOf, run_changes};

/// Once a run is undone it says who undid it and when, and each field says whether it went back or
/// stayed, and who changed it afterwards.
#[tokio::test]
async fn ai_38_an_undone_run_says_who_undid_it_and_what_became_of_each_field() -> Outcome {
    use crate::runs::{FieldOutcome, get_run};
    let (world, admin, run, [_, second]) = a_run_of_two().await?;
    let query = || RunQuery {
        session_token: admin.clone(),
        run,
    };
    assert!(
        get_run(&world.ctx(), query())
            .await?
            .into_value()
            .undone
            .is_none()
    );
    change_issue(
        &world.ctx(),
        priority_change(&admin, second, 2, Priority::Low),
    )
    .await?;
    let preview = preview_of(&world, &admin, run).await?;
    undo(&world, &admin, run, preview).await?;
    let seen = get_run(&world.ctx(), query()).await?.into_value();
    assert!(seen.undone.is_some());
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
    let outcomes: Vec<_> = page
        .items
        .iter()
        .map(|item| item.changes[0].outcome.clone())
        .collect();
    assert_eq!(outcomes[0], Some(FieldOutcome::WentBack));
    assert!(matches!(
        outcomes[1],
        Some(FieldOutcome::Kept {
            kind: UndoSkipKind::ChangedSince,
            by: Some(_),
            ..
        })
    ));
    Ok(())
}

/// The history says which changes were made in a run and which are the undo of it.
#[tokio::test]
async fn ai_38_the_history_marks_the_changes_of_a_run_and_the_undo() -> Outcome {
    use crate::history::{HistoryQuery, history};
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let preview = preview_of(&world, &admin, run).await?;
    undo(&world, &admin, run, preview).await?;
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: admin.clone(),
            issue: id.to_string(),
            tab: bb_core::history::HistoryTab::Changes,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    assert!(page.items.iter().any(
        |row| row.run_mark == Some(crate::history::RunMarkKind::Made) && row.run == Some(run)
    ));
    assert!(page.items.iter().any(
        |row| row.run_mark == Some(crate::history::RunMarkKind::Undid) && row.run == Some(run)
    ));
    Ok(())
}

/// The preview counts and names only the fields the reader may read, and a comment of the run is
/// one field that cannot be put back yet, named under its issue (M7-F60).
#[tokio::test]
async fn ai_38_the_preview_leaves_out_unreadable_fields_and_names_a_comment() -> Outcome {
    use super::history_cursor::reader_of_titles;
    use crate::comments::{AddComment, add_comment};

    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let reader = reader_of_titles(&world).await?;
    let person = world.ctx().session_actor(&reader).await?.person.id;
    let run = open(&world, &reader, person).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let ctx = world.ctx();
    ctx.set_run(Some(run));
    let mut title = priority_change(&admin, id, 2, Priority::Critical);
    title.priority = None;
    title.title = Some("A new title".to_owned());
    change_issue(&ctx, title).await?;
    // Two comments on one issue are one line under it.
    for text in ["Looked at it", "Looked again"] {
        add_comment(
            &ctx,
            AddComment {
                session_token: admin.clone(),
                issue: id.to_string(),
                text: text.to_owned(),
            },
        )
        .await?;
    }
    let preview = preview_of(&world, &reader, run).await?;
    // The priority is the reader's blind spot: only the title goes back, the comment stays.
    assert_eq!(preview.goes_back, 1);
    assert_eq!(preview.skipped, 1);
    assert_eq!(preview.skips[0].field, "comment");
    assert_eq!(preview.skips[0].kind, UndoSkipKind::Comment);
    assert!(preview.skips[0].issue.is_some());
    Ok(())
}

/// A run of an app session that changed the priority of two issues: the world, the session of the
/// administrator, the run and the issues.
async fn a_run_of_two()
-> Result<(World, String, RunId, [IssueId; 2]), Box<dyn std::error::Error + Send + Sync>> {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let first = an_issue(&world, project).await?;
    let second = an_issue(&world, project).await?;
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    for each in [first, second] {
        change_in_run(&world, &admin, run, each, 1, Priority::Critical).await?;
    }
    Ok((world, admin, run, [first, second]))
}

async fn outcomes_of(
    world: &World,
    session: &str,
    run: RunId,
) -> Result<Vec<Option<crate::runs::FieldOutcome>>, AppError> {
    let page = run_changes(
        &world.ctx(),
        RunChangesQuery {
            session_token: session.to_owned(),
            run,
            after: None,
            limit: None,
        },
    )
    .await?
    .into_value();
    Ok(page
        .items
        .iter()
        .map(|item| item.changes[0].outcome.clone())
        .collect())
}

/// A field that went back stays "went back" when somebody changes it after the undo.
#[tokio::test]
async fn ai_38_a_field_that_went_back_stays_so_when_it_is_changed_after_the_undo() -> Outcome {
    use crate::runs::FieldOutcome;
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let preview = preview_of(&world, &admin, run).await?;
    undo(&world, &admin, run, preview).await?;
    change_issue(&world.ctx(), priority_change(&admin, id, 3, Priority::Low)).await?;
    assert_eq!(
        outcomes_of(&world, &admin, run).await?,
        vec![Some(FieldOutcome::WentBack)]
    );
    Ok(())
}

/// A change made between the plan and the write is not written over: it is a conflict, kept and
/// named.
#[tokio::test]
async fn ai_38_a_change_between_the_plan_and_the_write_is_not_overwritten() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let id = an_issue(&world, project).await?;
    let person = world.ctx().session_actor(&admin).await?.person.id;
    let run = open(&world, &admin, person).await?;
    change_in_run(&world, &admin, run, id, 1, Priority::Critical).await?;
    let ctx = world.ctx();
    let actor = ctx.session_actor(&admin).await?;
    let runs = RunsOf::new(&ctx, &actor, &admin);
    let plan = runs.plan_for_reader(run).await?;
    // Somebody changes the issue after the plan looked at it.
    change_issue(&world.ctx(), priority_change(&admin, id, 2, Priority::Low)).await?;
    let done = runs.apply_plan(run, plan).await?;
    assert_eq!(done.undone, 0);
    assert_eq!(done.skips[0].kind, UndoSkipKind::ChangedSince);
    assert_eq!(priority_of(&world, &admin, id).await?, Priority::Low);
    Ok(())
}

/// A busy database in the middle of an undo is an error, never a skip. What went back before it is
/// recorded and what it did not reach is "not yet undone": the run goes on offering the undo, and a
/// second undo puts the rest back.
#[tokio::test]
async fn ai_38_a_busy_database_stops_the_undo_and_a_second_undo_puts_the_rest_back() -> Outcome {
    use crate::runs::FieldOutcome;
    let (world, admin, run, _) = a_run_of_two().await?;
    let preview = preview_of(&world, &admin, run).await?;
    world.store.keep_a_change_busy_after(1);
    assert!(undo(&world, &admin, run, preview).await.is_err());
    let seen = crate::runs::get_run(
        &world.ctx(),
        RunQuery {
            session_token: admin.clone(),
            run,
        },
    )
    .await?
    .into_value();
    assert!(seen.undone.as_ref().is_some_and(|done| done.interrupted));
    let outcomes = outcomes_of(&world, &admin, run).await?;
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.contains(&Some(FieldOutcome::WentBack)));
    assert!(outcomes.contains(&Some(FieldOutcome::NotYet)));
    // What it did not reach can still go back, and the second undo puts it back.
    let again = preview_of(&world, &admin, run).await?;
    assert_eq!(again.goes_back, 1);
    assert_eq!(again.skipped, 0);
    let done = undo(&world, &admin, run, again).await?;
    assert_eq!(done.goes_back, 1);
    assert_eq!(
        outcomes_of(&world, &admin, run).await?,
        vec![Some(FieldOutcome::WentBack), Some(FieldOutcome::WentBack)]
    );
    let seen = crate::runs::get_run(
        &world.ctx(),
        RunQuery {
            session_token: admin.clone(),
            run,
        },
    )
    .await?
    .into_value();
    assert!(seen.undone.is_some_and(|done| !done.interrupted));
    Ok(())
}

/// A mixed undo: the person may change one issue and only see the other, so one goes back and the
/// other stays for the missing right, and the page says so.
#[tokio::test]
async fn ai_38_a_field_kept_for_a_missing_right_says_so_after_a_mixed_undo() -> Outcome {
    use crate::runs::FieldOutcome;
    let world = World::new().await?;
    let admin = token(&world);
    let editable = a_project(&world, "ALPHA").await?;
    let visible = a_project(&world, "BETA").await?;
    let first = an_issue(&world, editable).await?;
    let second = an_issue(&world, visible).await?;
    let bob = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        editable,
    )
    .await?;
    let viewer = starting_role(&world, StartingRole::Viewer).await?;
    assign(&world, &bob, viewer, visible).await?;
    let run = open(&world, &bob.session_token, bob.person.id).await?;
    for each in [first, second] {
        change_in_run(&world, &admin, run, each, 1, Priority::Critical).await?;
    }
    let preview = preview_of(&world, &bob.session_token, run).await?;
    assert_eq!(preview.goes_back, 2);
    let done = undo(&world, &bob.session_token, run, preview).await?;
    assert_eq!(done.goes_back, 1);
    let outcomes = outcomes_of(&world, &bob.session_token, run).await?;
    assert_eq!(outcomes[0], Some(FieldOutcome::WentBack));
    assert!(matches!(
        outcomes[1],
        Some(FieldOutcome::Kept {
            kind: UndoSkipKind::NotAllowed,
            ..
        })
    ));
    Ok(())
}

/// An issue that is in the trash by the time of the write stays, and the page says it is gone.
#[tokio::test]
async fn ai_38_a_field_of_an_issue_deleted_after_the_plan_stays_as_gone() -> Outcome {
    use crate::issues::{DeleteIssue, delete_issue};
    let (world, admin, run, [_, second]) = a_run_of_two().await?;
    let ctx = world.ctx();
    let actor = ctx.session_actor(&admin).await?;
    let runs = RunsOf::new(&ctx, &actor, &admin);
    let plan = runs.plan_for_reader(run).await?;
    delete_issue(
        &world.ctx(),
        DeleteIssue {
            session_token: admin.clone(),
            issue: second.to_string(),
        },
    )
    .await?;
    let done = runs.apply_plan(run, plan).await?;
    assert_eq!(done.undone, 1);
    assert_eq!(done.skips[0].kind, UndoSkipKind::Gone);
    Ok(())
}

/// An undo that stopped half way waits for what it did not reach. When that can no longer go back,
/// the next undo says why and the run stops waiting.
#[tokio::test]
async fn ai_38_what_an_interrupted_undo_did_not_reach_gets_its_reason_when_it_cannot_go_back()
-> Outcome {
    use crate::runs::{FieldOutcome, get_run};
    let (world, admin, run, [first, second]) = a_run_of_two().await?;
    let preview = preview_of(&world, &admin, run).await?;
    world.store.keep_a_change_busy_after(1);
    assert!(undo(&world, &admin, run, preview).await.is_err());
    // The issue the undo did not reach is changed by somebody before the next undo.
    let waiting = outcomes_of(&world, &admin, run).await?;
    let not_reached = if waiting[0] == Some(FieldOutcome::NotYet) {
        first
    } else {
        second
    };
    let version = issue(
        &world.ctx(),
        IssueQuery {
            session_token: admin.clone(),
            issue: not_reached.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .version;
    change_issue(
        &world.ctx(),
        priority_change(&admin, not_reached, version, Priority::Low),
    )
    .await?;
    let again = preview_of(&world, &admin, run).await?;
    assert_eq!(again.goes_back, 0);
    let done = undo(&world, &admin, run, again).await?;
    assert_eq!(done.goes_back, 0);
    let seen = get_run(
        &world.ctx(),
        RunQuery {
            session_token: admin.clone(),
            run,
        },
    )
    .await?
    .into_value();
    assert!(seen.undone.is_some_and(|done| !done.interrupted));
    let outcomes = outcomes_of(&world, &admin, run).await?;
    assert!(!outcomes.contains(&Some(FieldOutcome::NotYet)));
    assert!(outcomes.contains(&Some(FieldOutcome::WentBack)));
    assert!(outcomes.iter().any(|outcome| matches!(
        outcome,
        Some(FieldOutcome::Kept {
            kind: UndoSkipKind::ChangedSince,
            ..
        })
    )));
    Ok(())
}
