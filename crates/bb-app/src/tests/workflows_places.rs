//! Where a workflow counts and how it changes (WF-01, WF-02, WF-03, ISS-09, ADR-0031): a project
//! finds its workflow like a layout, publishing a workflow that drops a status moves its issues,
//! changing many issues follows each issue's workflow, a place uses another workflow, and *Try it*.

use bb_core::ErrorCode;
use bb_core::id::{RoleId, UserId};
use bb_core::many_changes::SkipReason;
use bb_core::workflow::starter::{DONE, IN_PROGRESS, OPEN};
use bb_core::workflow::{StatusRules, WhoCan};
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, published_with};
use super::workflows::change;
use super::workflows_rules::{key, member, offers, status_of, with_rules};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::history::{HistoryQuery, HistoryWhat, history};
use crate::issues::change_issue;
use crate::many_changes::{ManyChange, ManyChangeAction};
use crate::workflows::{
    DeleteWorkflow, PublishWorkflow, StatusMoveInput, TriedVerdict, TryWorkflowQuery, UseWorkflow,
    WorkflowEditorQuery, delete_workflow, try_workflow, use_workflow, workflow_editor,
};

#[tokio::test]
async fn wf_01_a_project_uses_the_workflow_of_its_parent_until_it_has_its_own() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let other = a_project(&world, "OTHER").await?;
    let child = crate::projects::create_project(
        &world.ctx(),
        crate::projects::CreateProject {
            session_token: world.admin.session_token.clone(),
            parent: Some(acme),
            short_name: "KID".to_owned(),
            name: "A child".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let editor = a_workflow(&world, "ACME").await?;
    // The project drops the way back from Done: it changes the workflow it inherited.
    published_with(&world, &editor, |sent| {
        sent.changes
            .retain(|each| !(each.from == DONE && each.to == OPEN));
    })
    .await?;
    let admin = world.admin.session_token.clone();
    for (project, offered) in [(acme, false), (child, false), (other, true)] {
        let id = an_issue(&world, project).await?;
        change_issue(&world.ctx(), change(&admin, &id.to_string(), 1, DONE)?).await?;
        assert_eq!(
            offers(&world, &admin, id, OPEN).await?,
            offered,
            "{project}: the way back from Done"
        );
    }
    // Another project sees the editor with the workflow it inherits, set elsewhere.
    let shown = workflow_editor(
        &world.ctx(),
        WorkflowEditorQuery {
            session_token: admin.clone(),
            project: Some("KID".to_owned()),
            issue_type: None,
        },
    )
    .await?
    .into_value();
    assert!(shown.workflow.is_none());
    assert_eq!(
        shown
            .inherited_from
            .and_then(|from| from.project)
            .map(|each| each.id),
        Some(acme)
    );
    Ok(())
}

#[tokio::test]
async fn wf_02_publishing_without_a_status_moves_its_issues_with_history() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let id = an_issue(&world, project).await?;
    let address = id.to_string();
    change_issue(&world.ctx(), change(&admin, &address, 1, IN_PROGRESS)?).await?;
    let editor = a_workflow(&world, "ACME").await?;
    let without = |sent: &mut PublishWorkflow| {
        sent.statuses
            .retain(|each| each.key.as_ref() != Some(&key(IN_PROGRESS)));
        sent.changes
            .retain(|each| each.from != IN_PROGRESS && each.to != IN_PROGRESS);
    };
    // No place to go to for the issue: nothing is published.
    let refused = published_with(&world, &editor, without).await.err();
    assert_eq!(
        refused,
        Some(AppError::Refused(ErrorCode::WorkflowStatusInUse))
    );
    assert_eq!(status_of(&world, &admin, id).await?, IN_PROGRESS);
    // With the place named, the workflow is published and the issue goes there.
    published_with(&world, &editor, |sent| {
        without(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: key(OPEN),
        }];
    })
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, OPEN);
    // The move has its history: an event with the status before and after.
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: admin,
            issue: address,
            tab: bb_core::history::HistoryTab::Changes,
            cursor: None,
            limit: 20,
        },
    )
    .await?
    .into_value();
    let moved = page.items.iter().any(|item| match &item.what {
        HistoryWhat::Changes { fields } => fields.iter().any(|each| {
            each.field == "status"
                && matches!(
                    &each.after,
                    Some(crate::history::ChangeValue::Text { text }) if text == OPEN
                )
        }),
        _ => false,
    });
    assert!(moved, "the history names the move");
    Ok(())
}

#[tokio::test]
async fn iss_09_many_changes_skip_an_issue_whose_workflow_does_not_allow_it() -> Outcome {
    use super::many_changes::{apply, preview, targets};

    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let other = a_project(&world, "OTHER").await?;
    // ACME has no way from Open straight to Done.
    let editor = a_workflow(&world, "ACME").await?;
    published_with(&world, &editor, |sent| {
        sent.changes
            .retain(|each| !(each.from == OPEN && each.to == DONE));
    })
    .await?;
    let (blocked, free) = (
        an_issue(&world, acme).await?,
        an_issue(&world, other).await?,
    );
    let admin = world.admin.session_token.clone();
    let action = ManyChangeAction::Change {
        change: ManyChange {
            status: Some(key(DONE)),
            ..ManyChange::default()
        },
    };
    let shown = preview(&world, &admin, &[blocked, free], &action).await?;
    let skipped: Vec<Option<SkipReason>> = shown.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(
        skipped,
        [Some(SkipReason::Workflow), None],
        "the preview says so"
    );
    // The change does what the preview said.
    let done = apply(&world, &admin, targets(&shown), &action).await?;
    let outcomes: Vec<Option<SkipReason>> = done.outcomes.iter().map(|each| each.skipped).collect();
    assert_eq!(outcomes, [Some(SkipReason::Workflow), None]);
    assert_eq!(status_of(&world, &admin, blocked).await?, OPEN);
    assert_eq!(status_of(&world, &admin, free).await?, DONE);
    Ok(())
}

#[tokio::test]
async fn wf_02_a_workflow_is_used_by_another_place_and_taken_away_again() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    crate::projects::create_project(
        &world.ctx(),
        crate::projects::CreateProject {
            session_token: admin.clone(),
            parent: Some(acme),
            short_name: "KID".to_owned(),
            name: "A child".to_owned(),
        },
    )
    .await?;
    let editor = a_workflow(&world, "ACME").await?;
    let slot = editor.workflow.ok_or("no workflow")?;
    // A project below ACME uses the workflow of ACME by name.
    let used = use_workflow(
        &world.ctx(),
        UseWorkflow {
            session_token: admin.clone(),
            project: Some("KID".to_owned()),
            issue_type: None,
            workflow: Some(slot.id),
        },
    )
    .await?
    .into_value();
    assert_eq!(used.workflow.map(|each| each.id), Some(slot.id));
    // Taking the workflow away takes both uses with it; nothing is in a status it lacks.
    delete_workflow(
        &world.ctx(),
        DeleteWorkflow {
            session_token: admin.clone(),
            workflow: slot.id,
            version: slot.version,
        },
    )
    .await?;
    let back = workflow_editor(
        &world.ctx(),
        WorkflowEditorQuery {
            session_token: admin,
            project: Some("ACME".to_owned()),
            issue_type: None,
        },
    )
    .await?
    .into_value();
    // The project follows the organization again.
    assert!(back.workflow.is_none());
    assert!(
        back.inherited_from
            .is_some_and(|from| from.whole_organization)
    );
    Ok(())
}

#[tokio::test]
async fn wf_03_try_it_tells_what_a_person_would_get_and_why_not_the_rest() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    with_rules(
        &world,
        "ACME",
        &[(
            OPEN,
            IN_PROGRESS,
            StatusRules {
                who: WhoCan {
                    roles: vec![RoleId::from_parts(jiff::Timestamp::UNIX_EPOCH, [5; 10])?],
                    ..WhoCan::default()
                },
                ..StatusRules::default()
            },
        )],
    )
    .await?;
    let id = an_issue(&world, project).await?;
    let tried = try_workflow(
        &world.ctx(),
        TryWorkflowQuery {
            session_token: world.admin.session_token.clone(),
            issue: id.to_string(),
            person: team.person.id,
        },
    )
    .await?
    .into_value();
    let answers: Vec<(&str, bool)> = tried
        .changes
        .iter()
        .map(|each| (each.to.key.as_str(), each.verdict == TriedVerdict::Allowed))
        .collect();
    assert_eq!(answers, [("in-progress", false), ("done", true)]);
    // Somebody who may not manage workflows cannot try it.
    let refused = try_workflow(
        &world.ctx(),
        TryWorkflowQuery {
            session_token: team.session_token.clone(),
            issue: id.to_string(),
            person: UserId::from_random([4; 16]),
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::Forbidden)));
    Ok(())
}
