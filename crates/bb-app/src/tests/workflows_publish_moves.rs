//! Where the issues of a dropped status go, and what *Publish* tells before it asks (WF-02, M4-F42,
//! M4-F43, ADR-0031 amendment 11): a move may go to any status of the new workflow, judged with
//! the rights of the person who publishes and with no rule of a status change; the preview names
//! the issues the publisher sees and only counts the others.

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IssueId, LockToken, ProjectId, RoleId};
use bb_core::issue::StatusGroup;
use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_core::workflow::starter::{DONE, IN_PROGRESS, OPEN};
use bb_core::workflow::{StatusDraft, StatusRules};
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, publish_of, published_by, published_with};
use super::workflows::change;
use super::workflows_rules::{issue_in_progress, key, starting_role, status_of, with_rules};
use super::world::{Outcome, World};
use crate::auth::{AppError, Started};
use crate::issues::{IssueQuery, change_issue, issue};
use crate::projects::{CreateProject, create_project};
use crate::roles::{
    AssignRole, CopyRole, Given, SetFieldLevel, Subject, assign_role, copy_role, set_level,
};
use crate::workflows::{
    PublishPreview, PublishPreviewQuery, PublishWorkflow, StatusMoveInput, WorkflowEditor,
    WorkflowEditorQuery, publish_preview, workflow_editor,
};

/// What drops *In progress*, with the way to and from it.
fn without_progress(sent: &mut PublishWorkflow) {
    sent.statuses
        .retain(|each| each.key.as_ref() != Some(&key(IN_PROGRESS)));
    sent.changes
        .retain(|each| each.from != IN_PROGRESS && each.to != IN_PROGRESS);
}

/// The preview of what `sent` publishes, as `token` asks for it.
fn preview_of(token: &str, sent: PublishWorkflow) -> PublishPreviewQuery {
    PublishPreviewQuery {
        session_token: token.to_owned(),
        workflow: sent.workflow,
        statuses: sent.statuses,
        changes: sent.changes,
    }
}

/// The preview of dropping *In progress* from the workflow of the project, as `token` asks for it.
async fn preview_dropping_progress(
    world: &World,
    project: &str,
    token: &str,
    drop_it: bool,
) -> Result<PublishPreview, AppError> {
    let editor = a_workflow(world, project)
        .await
        .map_err(AppError::Internal)?;
    let mut sent =
        publish_of(world, &editor, LockToken::from_random([1; 16])).map_err(AppError::Internal)?;
    if drop_it {
        without_progress(&mut sent);
    }
    Ok(publish_preview(&world.ctx(), preview_of(token, sent))
        .await?
        .into_value())
}

#[tokio::test]
async fn wf_02_publishing_moves_the_issues_of_a_dropped_status_to_any_status_of_the_new_workflow()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let id = issue_in_progress(&world, project).await?;
    let editor = a_workflow(&world, "ACME").await?;
    // A status the new workflow lacks is no place to go to.
    let refused = published_with(&world, &editor, |sent| {
        without_progress(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: "nowhere".to_owned(),
        }];
    })
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::InvalidRequest)));
    assert_eq!(status_of(&world, &admin, id).await?, IN_PROGRESS);
    // Done is a place to go to, although no status change leads there from In progress any more.
    published_with(&world, &editor, |sent| {
        without_progress(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: DONE.to_owned(),
        }];
    })
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, DONE);
    Ok(())
}

/// A person with a copy of the role *Project lead* that sees *Status* but may not change it.
async fn lead_who_cannot_change_status(
    world: &World,
    project: ProjectId,
) -> Result<Started, AppError> {
    let person = world.person("lead@example.test").await?;
    let lead = starting_role(world, StartingRole::ProjectLead)
        .await
        .map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: admin.clone(),
            role: lead,
            name: "Watching lead".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin,
            role: copy.id,
            project: None,
            issue_type: None,
            field: FieldKey::BuiltIn(Field::Status),
            level: Some(FieldLevel::CanSee),
        },
    )
    .await?;
    given_in_one_project(world, &person, copy.id, project).await?;
    Ok(person)
}

/// Gives the role in the project alone, not in its sub-projects.
async fn given_in_one_project(
    world: &World,
    person: &Started,
    role: RoleId,
    project: ProjectId,
) -> Result<(), AppError> {
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: world.admin.session_token.clone(),
            subject: Subject::Person(person.person.id),
            given: Given::Role(role),
            project: Some(project),
            applies_to: AppliesTo::Project,
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn wf_02_the_preview_of_a_publish_counts_the_issues_of_each_dropped_status() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    for _ in 0..2 {
        let id = an_issue(&world, project).await?;
        change_issue(
            &world.ctx(),
            change(&admin, &id.to_string(), 1, IN_PROGRESS)?,
        )
        .await?;
    }
    an_issue(&world, project).await?;
    let shown = preview_dropping_progress(&world, "ACME", &admin, true).await?;
    assert_eq!(shown.limit, MANY_CHANGES_ISSUES_MAX);
    let [dropped] = shown.dropped.as_slice() else {
        return Err("exactly one status is dropped".into());
    };
    assert_eq!(dropped.status.key, key(IN_PROGRESS));
    assert_eq!((dropped.issues, dropped.unseen, dropped.blocked), (2, 0, 0));
    assert_eq!(dropped.named.len(), 2);
    // Changing nothing drops nothing.
    let nothing = preview_dropping_progress(&world, "ACME", &admin, false).await?;
    assert!(nothing.dropped.is_empty());
    Ok(())
}

#[tokio::test]
async fn wf_02_the_preview_names_what_the_publisher_sees_and_only_counts_the_rest() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let kid = create_project(
        &world.ctx(),
        CreateProject {
            session_token: world.admin.session_token.clone(),
            parent: Some(acme),
            short_name: "KID".to_owned(),
            name: "A child".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let admin = world.admin.session_token.clone();
    for project in [acme, kid] {
        let id = an_issue(&world, project).await?;
        change_issue(
            &world.ctx(),
            change(&admin, &id.to_string(), 1, IN_PROGRESS)?,
        )
        .await?;
    }
    // The lead holds the role in ACME only: the issue of KID is not theirs to see, and the one of
    // ACME is theirs to see but not to change.
    let lead = lead_who_cannot_change_status(&world, acme).await?;
    let shown = preview_dropping_progress(&world, "ACME", &lead.session_token, true).await?;
    let [dropped] = shown.dropped.as_slice() else {
        return Err("exactly one status is dropped".into());
    };
    assert_eq!(dropped.issues, 2);
    assert_eq!(dropped.unseen, 1);
    assert_eq!(dropped.blocked, 2);
    assert_eq!(dropped.named.len(), 1);
    Ok(())
}

#[tokio::test]
async fn iss_09_a_project_lists_the_status_changes_of_its_workflows_once_by_their_words() -> Outcome
{
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let listed = crate::workflows::project_statuses(
        &world.ctx(),
        crate::workflows::ProjectStatusesQuery {
            session_token: world.admin.session_token.clone(),
            project: "ACME".to_owned(),
        },
    )
    .await?
    .into_value();
    let words = |to: &str| -> Vec<String> {
        listed
            .changes
            .iter()
            .filter(|each| each.to.key == key(to))
            .map(|each| each.button.clone())
            .collect()
    };
    // Bug and Simple both have "Mark as done"; it is listed once. Only Bug leads to Testing.
    assert_eq!(words("testing"), ["Send to testing"]);
    assert_eq!(
        words(DONE)
            .iter()
            .filter(|each| *each == "Mark as done")
            .count(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn wf_04_the_description_cannot_be_asked_for_in_a_small_form() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let editor = a_workflow(&world, "ACME").await?;
    // The description is long text with a lock of its own: a status change cannot carry it, so a
    // rule that asks for it could never be met.
    let asking = published_with(&world, &editor, |sent| {
        if let Some(first) = sent.changes.first_mut() {
            first.rules.ask_for = vec![FieldKey::BuiltIn(Field::Description)];
        }
    })
    .await
    .err();
    assert_eq!(asking, Some(AppError::Refused(ErrorCode::WorkflowInvalid)));
    // It can still be required to be filled in before a change.
    published_with(&world, &editor, |sent| {
        if let Some(first) = sent.changes.first_mut() {
            first.rules.must_fill = vec![FieldKey::BuiltIn(Field::Description)];
        }
    })
    .await?;
    Ok(())
}

/// The editor of the workflow ACME has of its own, as the administrator reads it.
async fn editor_of_acme(world: &World) -> Result<WorkflowEditor, AppError> {
    Ok(workflow_editor(
        &world.ctx(),
        WorkflowEditorQuery {
            session_token: world.admin.session_token.clone(),
            project: Some("ACME".to_owned()),
            issue_type: None,
        },
    )
    .await?
    .into_value())
}

/// The status and the version an issue has now.
async fn status_and_version(world: &World, id: IssueId) -> Result<(String, i64), AppError> {
    let shown = issue(
        &world.ctx(),
        IssueQuery {
            session_token: world.admin.session_token.clone(),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue;
    Ok((shown.status.key.to_string(), shown.version))
}

/// A publish by a lead who may not change *Status* drops *In progress* with an issue in it that
/// they do not see (`unseen`) or see and may not change; it is refused, and nothing changes.
async fn refused_for_an_issue(unseen: bool) -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let kid = create_project(
        &world.ctx(),
        CreateProject {
            session_token: world.admin.session_token.clone(),
            parent: Some(acme),
            short_name: "KID".to_owned(),
            name: "A child".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let id = issue_in_progress(&world, if unseen { kid } else { acme }).await?;
    let lead = lead_who_cannot_change_status(&world, acme).await?;
    let editor = a_workflow(&world, "ACME").await?;
    let before = status_and_version(&world, id).await?;
    let refused = published_by(&world, &lead, &editor, |sent| {
        without_progress(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: OPEN.to_owned(),
        }];
    })
    .await
    .err();
    assert_eq!(
        refused,
        Some(AppError::Refused(ErrorCode::WorkflowStatusInUse))
    );
    assert_eq!(status_and_version(&world, id).await?, before);
    let after = editor_of_acme(&world).await?;
    assert_eq!(
        after.workflow.map(|slot| slot.version),
        editor.workflow.map(|slot| slot.version)
    );
    assert!(
        after
            .statuses
            .iter()
            .any(|each| each.key == key(IN_PROGRESS))
    );
    Ok(())
}

#[tokio::test]
async fn wf_02_a_publish_with_an_issue_the_publisher_does_not_see_publishes_nothing() -> Outcome {
    refused_for_an_issue(true).await
}

#[tokio::test]
async fn wf_02_a_publish_with_an_issue_whose_status_the_publisher_may_not_change_publishes_nothing()
-> Outcome {
    refused_for_an_issue(false).await
}

#[tokio::test]
async fn wf_02_a_move_ignores_the_rules_of_the_old_change_and_closes_by_the_group_of_the_target()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let id = issue_in_progress(&world, project).await?;
    // The old change into Done needs the description, which this issue lacks.
    let needs = StatusRules {
        must_fill: vec![FieldKey::BuiltIn(Field::Description)],
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(IN_PROGRESS, DONE, needs)]).await?;
    assert!(
        change_issue(&world.ctx(), change(&admin, &id.to_string(), 2, DONE)?)
            .await
            .is_err()
    );
    let editor = editor_of_acme(&world).await?;
    published_with(&world, &editor, |sent| {
        without_progress(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: DONE.to_owned(),
        }];
    })
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, DONE);
    let shown = issue(
        &world.ctx(),
        IssueQuery {
            session_token: admin,
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue;
    assert!(shown.closed_at.is_some(), "Done closes the issue");
    Ok(())
}

#[tokio::test]
async fn wf_02_a_move_may_go_to_a_status_that_is_new_in_the_draft() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let id = issue_in_progress(&world, project).await?;
    let editor = a_workflow(&world, "ACME").await?;
    published_with(&world, &editor, |sent| {
        without_progress(sent);
        sent.statuses.push(StatusDraft {
            key: None,
            name: "Review".to_owned(),
            group: StatusGroup::InProgress,
        });
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: "Review".to_owned(),
        }];
    })
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, "review");
    Ok(())
}

#[tokio::test]
async fn wf_02_the_issues_of_a_dropped_status_are_read_only_as_far_as_the_limit() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    for _ in 0..3 {
        issue_in_progress(&world, project).await?;
    }
    let editor = a_workflow(&world, "ACME").await?;
    let workflow = editor.workflow.ok_or("no workflow of its own")?.id;
    let ctx = world.ctx();
    let directory = ctx.workflow_directory().await?;
    let keys = [key(IN_PROGRESS)];
    // Reading stops one past the limit; below it everything is read.
    assert_eq!(
        ctx.governed_issues(&directory, workflow, &keys, 2)
            .await?
            .len(),
        3
    );
    assert_eq!(
        ctx.governed_issues(&directory, workflow, &keys, 1)
            .await?
            .len(),
        2
    );
    assert_eq!(
        ctx.governed_issues(&directory, workflow, &keys, 5)
            .await?
            .len(),
        3
    );
    Ok(())
}

#[tokio::test]
async fn wf_04_a_rule_that_names_no_such_field_is_refused_like_one_that_names_a_hidden_field()
-> Outcome {
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let editor = a_workflow(&world, "ACME").await?;
    // The answer must not tell whether the field is not there or only hidden from the person.
    let refused = published_with(&world, &editor, |sent| {
        if let Some(first) = sent.changes.first_mut() {
            first.rules.must_fill = vec![FieldKey::Own(
                bb_core::id::FieldId::from_bytes([
                    0x01, 0x92, 0xb0, 0xf2, 0x6f, 0x6e, 0x7c, 0x11, 0x8a, 0x41, 0x2a, 0x8f, 0x0c,
                    0x3d, 0x7e, 0x51,
                ])
                .unwrap_or_else(|_| unreachable!()),
            )];
        }
    })
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::WorkflowInvalid)));
    Ok(())
}
