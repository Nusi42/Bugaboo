//! Where the issues of a dropped status go, and what *Publish* tells before it asks (WF-02, M4-F42,
//! M4-F43, ADR-0031 amendment 11): a move may go to any status of the new workflow, judged with
//! the rights of the person who publishes and with no rule of a status change; the preview names
//! the issues the publisher sees and only counts the others.

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{LockToken, ProjectId, RoleId};
use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_core::workflow::starter::{DONE, IN_PROGRESS};
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, publish_of, published_with};
use super::workflows::change;
use super::workflows_rules::{key, starting_role, status_of};
use super::world::{Outcome, World};
use crate::auth::{AppError, Started};
use crate::issues::change_issue;
use crate::projects::{CreateProject, create_project};
use crate::roles::{
    AssignRole, CopyRole, Given, SetFieldLevel, Subject, assign_role, copy_role, set_level,
};
use crate::workflows::{
    PublishPreview, PublishPreviewQuery, PublishWorkflow, StatusMoveInput, publish_preview,
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
    let id = an_issue(&world, project).await?;
    change_issue(
        &world.ctx(),
        change(&admin, &id.to_string(), 1, IN_PROGRESS)?,
    )
    .await?;
    let editor = a_workflow(&world, "ACME").await?;
    // A status the new workflow lacks is no place to go to.
    let refused = published_with(&world, &editor, |sent| {
        without_progress(sent);
        sent.moves = vec![StatusMoveInput {
            from: key(IN_PROGRESS),
            to: key("nowhere"),
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
            to: key(DONE),
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
