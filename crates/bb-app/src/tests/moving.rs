//! Moving an issue into another project against the in-memory store (ISS-09, ADR-0021, M3-Q4): the
//! issue gets an ID of the new project and the old one keeps working, and it moves only where it
//! can live on without losing anything.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::own_fields::{FieldType, Scope};
use serde_json::json;

use super::scenarios::{a_project, token};
use super::scenarios_fields::a_field;
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::fields::ChangeField;
use crate::issues::{ChangeIssue, IssueQuery, MoveIssue, change_issue, issue, move_issue};
use bb_core::own_fields::Edit;

fn moving(world: &World, issue: &str, version: i64, project: &str) -> MoveIssue {
    MoveIssue {
        session_token: token(world),
        issue: issue.to_owned(),
        version,
        project: project.to_owned(),
    }
}

#[tokio::test]
async fn iss_09_a_moved_issue_has_an_id_of_the_new_project_and_the_old_one_keeps_working() -> Outcome
{
    let world = World::new().await?;
    let (acme, elsewhere) = (
        a_project(&world, "ACME").await?,
        a_project(&world, "ELSE").await?,
    );
    let id = an_issue(&world, acme).await?;
    let before = issue(
        &world.ctx(),
        IssueQuery {
            session_token: token(&world),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue;
    assert!(before.key.starts_with("ACME-"), "{}", before.key);
    let moved = move_issue(&world.ctx(), moving(&world, &before.key, 1, "ELSE"))
        .await?
        .into_value();
    assert!(moved.key.starts_with("ELSE-"), "{}", moved.key);
    assert_eq!(moved.project.id, elsewhere);
    // The old ID finds the issue, and the answer holds the new one.
    let by_old = issue(
        &world.ctx(),
        IssueQuery {
            session_token: token(&world),
            issue: before.key,
        },
    )
    .await?
    .into_value()
    .issue;
    assert_eq!((by_old.id, by_old.key), (id, moved.key));
    Ok(())
}

#[tokio::test]
async fn iss_09_moving_to_the_project_it_is_in_changes_nothing() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let id = an_issue(&world, acme).await?;
    let events = world.store.event_count();
    let same = move_issue(&world.ctx(), moving(&world, &id.to_string(), 1, "ACME"))
        .await?
        .into_value();
    assert_eq!(same.project.id, acme);
    assert_eq!(world.store.event_count(), events);
    Ok(())
}

#[tokio::test]
async fn iss_09_a_move_on_an_old_version_is_stopped() -> Outcome {
    let world = World::new().await?;
    let (acme, _) = (
        a_project(&world, "ACME").await?,
        a_project(&world, "ELSE").await?,
    );
    let id = an_issue(&world, acme).await?;
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: token(&world),
            issue: id.to_string(),
            version: 1,
            issue_type: None,
            title: Some("Changed".to_owned()),
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?;
    let stopped = move_issue(&world.ctx(), moving(&world, &id.to_string(), 1, "ELSE"))
        .await
        .err();
    assert!(matches!(stopped, Some(AppError::Stale(_))), "{stopped:?}");
    Ok(())
}

#[tokio::test]
async fn iss_09_an_issue_with_a_value_the_other_project_has_no_field_for_stays() -> Outcome {
    let world = World::new().await?;
    let (acme, elsewhere) = (
        a_project(&world, "ACME").await?,
        a_project(&world, "ELSE").await?,
    );
    let field = a_field(&world, "Customer", FieldType::Text, &[]).await?;
    // The field is used in ACME only.
    crate::fields::change_field(
        &world.ctx(),
        ChangeField {
            session_token: token(&world),
            field: field.id,
            version: field.version,
            name: None,
            help: None,
            rules: None,
            default_value: Edit::Keep,
            look: None,
            never_to_assistant: None,
            scope: Some(Scope {
                projects: vec![acme],
                issue_types: Vec::new(),
            }),
            retired: None,
        },
    )
    .await?;
    let id = an_issue(&world, acme).await?;
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: token(&world),
            issue: id.to_string(),
            version: 1,
            issue_type: None,
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::from([(field.id.to_string(), json!("Ada"))]),
        },
    )
    .await?;
    let stays = move_issue(
        &world.ctx(),
        moving(&world, &id.to_string(), 2, &elsewhere.to_string()),
    )
    .await
    .err();
    assert_eq!(stays, Some(AppError::Refused(ErrorCode::FieldNotHere)));
    Ok(())
}

#[tokio::test]
async fn wf_03_a_move_that_starts_the_status_again_needs_the_status_level_of_the_new_place()
-> Outcome {
    use bb_core::access::FieldLevel;
    use bb_core::fields::{Field, FieldKey};
    use bb_core::issue::StatusGroup;
    use bb_core::workflow::StatusDraft;
    use bb_policy::StartingRole;

    use super::scenarios_workflows::{a_workflow, published_with};
    use super::workflows_rules::{assign, starting_role, status_of};
    use crate::roles::{CopyRole, SetFieldLevel, copy_role, set_level};

    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let base = a_project(&world, "BASE").await?;
    let admin = token(&world);
    // BASE has a workflow of its own: an issue that moves there starts again in *Intake*.
    let editor = a_workflow(&world, "BASE").await?;
    published_with(&world, &editor, |sent| {
        sent.statuses = vec![StatusDraft {
            key: None,
            name: "Intake".to_owned(),
            group: StatusGroup::NotStarted,
        }];
        sent.changes = Vec::new();
    })
    .await?;
    // A lead who may move issues.
    let reader = world.person("sees@example.test").await?;
    let lead = starting_role(&world, StartingRole::ProjectLead)
        .await
        .map_err(AppError::Internal)?;
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: admin.clone(),
            role: lead,
            name: "Status viewer".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin.clone(),
            role: copy.id,
            project: None,
            issue_type: None,
            field: FieldKey::BuiltIn(Field::Status),
            level: Some(FieldLevel::CanSee),
        },
    )
    .await?;
    // In ACME, where the issue is, the role may change the status; in BASE it only sees it.
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin.clone(),
            role: copy.id,
            project: Some(acme),
            issue_type: None,
            field: FieldKey::BuiltIn(Field::Status),
            level: Some(FieldLevel::CanChange),
        },
    )
    .await?;
    assign(&world, &reader, copy.id, acme).await?;
    assign(&world, &reader, copy.id, base).await?;
    let id = an_issue(&world, acme).await?;
    let refused = move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: reader.session_token.clone(),
            issue: id.to_string(),
            version: 1,
            project: "BASE".to_owned(),
        },
    )
    .await;
    assert_eq!(refused.err(), Some(AppError::Refused(ErrorCode::Forbidden)));
    // With the level in BASE as well the same person may move it, and the issue starts again.
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin.clone(),
            role: copy.id,
            project: Some(base),
            issue_type: None,
            field: FieldKey::BuiltIn(Field::Status),
            level: Some(FieldLevel::CanChange),
        },
    )
    .await?;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: reader.session_token.clone(),
            issue: id.to_string(),
            version: 1,
            project: "BASE".to_owned(),
        },
    )
    .await?;
    assert_eq!(status_of(&world, &admin, id).await?, "intake");
    Ok(())
}
