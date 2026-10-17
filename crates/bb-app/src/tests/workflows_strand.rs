//! A change to workflows never leaves an issue in a status its workflow does not have, a place
//! only uses workflows that sit at it or above it, and *Publish* and *Try it* name no field or
//! group the person may not know (WF-02, ADR-0031 amendments 3 to 6).

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IssueId, ProjectId};
use bb_core::issue::StatusGroup;
use bb_core::workflow::starter::{IN_PROGRESS, OPEN};
use bb_core::workflow::{ChangeDraft, Condition, StatusDraft, StatusRules, Test};
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, published_with};
use super::workflows::change;
use super::workflows_rules::{assign, key, member, starting_role, status_of};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::history::{HistoryQuery, HistoryWhat, history};
use crate::issues::{DeleteIssue, RestoreIssue, change_issue, delete_issue, restore_issue};
use crate::projects::{MovePreviewQuery, MoveProject, move_project, preview_move};
use crate::roles::{CopyRole, SetFieldLevel, copy_role, set_level};
use crate::workflows::{
    CreateWorkflow, DeleteWorkflow, TriedVerdict, TryWorkflowQuery, UseWorkflow, WorkflowsQuery,
    create_workflow, delete_workflow, try_workflow, use_workflow, workflows,
};

const REVIEW: &str = "review";

/// A person with a copy of the role *Project lead* that does not see *Assigned to*, in the project.
async fn blind_lead(world: &World, project: ProjectId) -> Result<crate::auth::Started, AppError> {
    let person = world.person("blind@example.test").await?;
    let lead = starting_role(world, StartingRole::ProjectLead)
        .await
        .map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: admin.clone(),
            role: lead,
            name: "Blind lead".to_owned(),
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
            field: FieldKey::BuiltIn(Field::AssignedTo),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    assign(world, &person, copy.id, project).await?;
    Ok(person)
}

/// ACME has a workflow of its own with a status *Review* that *Simple* lacks, and an issue in it.
async fn issue_in_review(world: &World) -> Result<(ProjectId, IssueId), AppError> {
    let acme = a_project(world, "ACME").await.map_err(AppError::Internal)?;
    let editor = a_workflow(world, "ACME")
        .await
        .map_err(AppError::Internal)?;
    published_with(world, &editor, |sent| {
        sent.statuses.push(StatusDraft {
            key: None,
            name: "Review".to_owned(),
            group: StatusGroup::InProgress,
        });
        sent.changes.push(ChangeDraft {
            from: OPEN.to_owned(),
            to: "Review".to_owned(),
            button: "Review".to_owned(),
            rules: StatusRules::default(),
        });
        sent.changes.push(ChangeDraft {
            from: "Review".to_owned(),
            to: OPEN.to_owned(),
            button: "Back".to_owned(),
            rules: StatusRules::default(),
        });
    })
    .await?;
    let id = an_issue(world, acme).await.map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    change_issue(&world.ctx(), change(&admin, &id.to_string(), 1, REVIEW)?).await?;
    Ok((acme, id))
}

#[tokio::test]
async fn wf_02_taking_a_workflow_away_never_strands_an_issue() -> Outcome {
    let world = World::new().await?;
    let (_, id) = issue_in_review(&world).await?;
    let admin = world.admin.session_token.clone();
    let editor = a_workflow(&world, "ACME").await?;
    let slot = editor.workflow.ok_or("no workflow")?;
    let away = |slot: crate::workflows::WorkflowSlot| DeleteWorkflow {
        session_token: admin.clone(),
        workflow: slot.id,
        version: slot.version,
    };
    // Simple, which counts afterwards, has no Review: the issue would be stranded.
    assert_eq!(
        delete_workflow(&world.ctx(), away(slot)).await.err(),
        Some(AppError::Refused(ErrorCode::WorkflowStatusInUse))
    );
    assert_eq!(status_of(&world, &admin, id).await?, REVIEW);
    // Following the parent again would strand it just the same.
    let follow = use_workflow(
        &world.ctx(),
        UseWorkflow {
            session_token: admin.clone(),
            project: Some("ACME".to_owned()),
            issue_type: None,
            workflow: None,
        },
    )
    .await;
    assert_eq!(
        follow.err(),
        Some(AppError::Refused(ErrorCode::WorkflowStatusInUse))
    );
    // Once the issue is out of Review, the workflow can go.
    change_issue(&world.ctx(), change(&admin, &id.to_string(), 2, OPEN)?).await?;
    delete_workflow(&world.ctx(), away(slot)).await?;
    Ok(())
}

#[tokio::test]
async fn wf_03_an_issue_back_from_the_trash_starts_again_in_the_starting_status() -> Outcome {
    let world = World::new().await?;
    let (_, id) = issue_in_review(&world).await?;
    let admin = world.admin.session_token.clone();
    let address = id.to_string();
    delete_issue(
        &world.ctx(),
        DeleteIssue {
            session_token: admin.clone(),
            issue: address.clone(),
        },
    )
    .await?;
    // In the trash it keeps its status, so taking the workflow away is allowed.
    let editor = a_workflow(&world, "ACME").await?;
    let slot = editor.workflow.ok_or("no workflow")?;
    delete_workflow(
        &world.ctx(),
        DeleteWorkflow {
            session_token: admin.clone(),
            workflow: slot.id,
            version: slot.version,
        },
    )
    .await?;
    restore_issue(
        &world.ctx(),
        RestoreIssue {
            session_token: admin.clone(),
            issue: address.clone(),
        },
    )
    .await?;
    // Back from the trash, the issue is outside its workflow, so it starts again in the starting
    // status, and that has its history. It can go no other way without the rules.
    assert_eq!(status_of(&world, &admin, id).await?, OPEN);
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: admin.clone(),
            issue: address.clone(),
            tab: bb_core::history::HistoryTab::Changes,
            cursor: None,
            limit: 20,
        },
    )
    .await?
    .into_value();
    let restarted = page.items.iter().any(|item| match &item.what {
        HistoryWhat::Changes { fields } => fields.iter().any(|each| {
            each.field == "status"
                && matches!(
                    &each.after,
                    Some(crate::history::ChangeValue::Text { text }) if text == OPEN
                )
        }),
        _ => false,
    });
    assert!(restarted, "the start again is in the history");
    Ok(())
}

#[tokio::test]
async fn wf_03_a_moved_project_sends_the_issues_its_workflow_lacks_to_the_starting_status()
-> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let base = a_project(&world, "BASE").await?;
    let admin = world.admin.session_token.clone();
    // BASE has a workflow of its own whose statuses Simple lacks.
    let editor = a_workflow(&world, "BASE").await?;
    published_with(&world, &editor, |sent| {
        sent.statuses = vec![
            StatusDraft {
                key: None,
                name: "Intake".to_owned(),
                group: StatusGroup::NotStarted,
            },
            StatusDraft {
                key: None,
                name: "Closed".to_owned(),
                group: StatusGroup::Done,
            },
        ];
        sent.changes = vec![ChangeDraft {
            from: "Intake".to_owned(),
            to: "Closed".to_owned(),
            button: "Close".to_owned(),
            rules: StatusRules::default(),
        }];
    })
    .await?;
    let id = an_issue(&world, acme).await?;
    change_issue(
        &world.ctx(),
        change(&admin, &id.to_string(), 1, IN_PROGRESS)?,
    )
    .await?;
    let preview = preview_move(
        &world.ctx(),
        MovePreviewQuery {
            session_token: admin.clone(),
            project: acme,
            to: Some(base),
        },
    )
    .await?
    .into_value();
    move_project(
        &world.ctx(),
        MoveProject {
            session_token: admin.clone(),
            project: acme,
            to: Some(base),
            preview: preview.preview,
        },
    )
    .await?;
    // The projects below BASE use its workflow: the issue starts again, not in a status of Simple.
    assert_eq!(status_of(&world, &admin, id).await?, "intake");
    Ok(())
}

#[tokio::test]
async fn wf_02_a_lead_uses_no_workflow_of_a_sibling_project() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    a_project(&world, "OTHER").await?;
    let lead = member(&world, "lena@example.test", StartingRole::ProjectLead, acme).await?;
    let others = a_workflow(&world, "OTHER").await?;
    let slot = others.workflow.ok_or("no workflow")?;
    // The lead of ACME does not see the workflow of OTHER in the choice, and cannot use it.
    let listed = workflows(
        &world.ctx(),
        WorkflowsQuery {
            session_token: lead.session_token.clone(),
            project: Some("ACME".to_owned()),
        },
    )
    .await?
    .into_value();
    assert!(listed.workflows.iter().all(|each| each.id != slot.id));
    let taken = use_workflow(
        &world.ctx(),
        UseWorkflow {
            session_token: lead.session_token.clone(),
            project: Some("ACME".to_owned()),
            issue_type: None,
            workflow: Some(slot.id),
        },
    )
    .await;
    assert_eq!(taken.err(), Some(AppError::Refused(ErrorCode::NotFound)));
    // The workflows of the organization, which sit above ACME, are the lead's to choose from.
    assert!(!listed.workflows.is_empty());
    let above = listed.workflows[0].id;
    use_workflow(
        &world.ctx(),
        UseWorkflow {
            session_token: lead.session_token,
            project: Some("ACME".to_owned()),
            issue_type: None,
            workflow: Some(above),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn sec_06_publishing_a_rule_names_no_field_or_group_the_person_may_not_know() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    // A lead who does not see *Assigned to*, and does not manage groups.
    let blind = blind_lead(&world, acme).await?;
    let editor = a_workflow(&world, "ACME").await?;
    let hidden = FieldKey::BuiltIn(Field::AssignedTo);
    let rule = StatusRules {
        only_if: vec![Condition {
            field: hidden,
            test: Test::Filled,
        }],
        ..StatusRules::default()
    };
    let mut sent = super::scenarios_workflows::publish_of(
        &world,
        &editor,
        super::scenarios_workflows::lock_the_workflow(&world, editor.workflow.ok_or("none")?),
    )?;
    "x".clone_into(&mut sent.changes[0].button);
    sent.changes[0].rules = rule;
    sent.session_token = blind.session_token.clone();
    let refused = crate::workflows::publish_workflow(&world.ctx(), sent)
        .await
        .err();
    assert_eq!(
        refused,
        Some(AppError::Refused(ErrorCode::WorkflowInvalid)),
        "a new rule that names a field the publisher cannot see"
    );
    // A lead who does not manage groups names no group they do not know.
    let lead = member(&world, "lena@example.test", StartingRole::ProjectLead, acme).await?;
    let editor = a_workflow(&world, "ACME").await?;
    let mut sent = super::scenarios_workflows::publish_of(
        &world,
        &editor,
        super::scenarios_workflows::lock_the_workflow(&world, editor.workflow.ok_or("none")?),
    )?;
    sent.changes[0].rules.who.groups = vec![bb_core::id::GroupId::from_parts(
        jiff::Timestamp::UNIX_EPOCH,
        [4; 10],
    )?];
    sent.session_token = lead.session_token;
    let refused = crate::workflows::publish_workflow(&world.ctx(), sent)
        .await
        .err();
    assert_eq!(
        refused,
        Some(AppError::Refused(ErrorCode::WorkflowInvalid)),
        "a new rule that names a group the publisher does not know"
    );
    Ok(())
}

#[tokio::test]
async fn sec_06_try_it_names_nothing_of_a_field_the_person_trying_cannot_see() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    let blind = blind_lead(&world, acme).await?;
    let hidden = FieldKey::BuiltIn(Field::AssignedTo);
    // The administrator writes a rule on the field the lead cannot see.
    super::workflows_rules::with_rules(
        &world,
        "ACME",
        &[(
            OPEN,
            IN_PROGRESS,
            StatusRules {
                ask_for: vec![hidden],
                ..StatusRules::default()
            },
        )],
    )
    .await?;
    let id = an_issue(&world, acme).await?;
    let tried = try_workflow(
        &world.ctx(),
        TryWorkflowQuery {
            session_token: blind.session_token.clone(),
            issue: id.to_string(),
            person: blind.person.id,
        },
    )
    .await?
    .into_value();
    let start = tried
        .changes
        .iter()
        .find(|each| each.to.key == key(IN_PROGRESS))
        .ok_or("no change")?;
    assert_eq!(
        start.verdict,
        TriedVerdict::NotAllowed {
            why: bb_core::workflow::judge::Why::CannotFill,
            field: None
        }
    );
    Ok(())
}

#[tokio::test]
async fn wf_01_the_status_changes_of_a_project_name_the_status_they_lead_to_once() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let admin = world.admin.session_token.clone();
    let listed = async |world: &World| -> Result<Vec<String>, AppError> {
        let found = crate::workflows::project_statuses(
            &world.ctx(),
            crate::workflows::ProjectStatusesQuery {
                session_token: world.admin.session_token.clone(),
                project: "ACME".to_owned(),
            },
        )
        .await?
        .into_value();
        let mut names: Vec<String> = Vec::new();
        for change in found
            .changes
            .into_iter()
            .filter(|each| each.to.key == key(OPEN))
        {
            if !names.contains(&change.to.name) {
                names.push(change.to.name);
            }
        }
        Ok(names)
    };
    // The starter workflows all call their first status Open: it is listed once.
    assert_eq!(listed(&world).await?, ["Open"]);
    // ACME calls it something else for tasks: the same key is listed for each name.
    let task_type = super::scenarios_issues::a_type(&world).await?;
    let editor = create_workflow(
        &world.ctx(),
        CreateWorkflow {
            session_token: admin,
            project: Some("ACME".to_owned()),
            issue_type: Some(task_type),
        },
    )
    .await?
    .into_value();
    published_with(&world, &editor, |sent| {
        "To do".clone_into(&mut sent.statuses[0].name);
    })
    .await?;
    assert_eq!(listed(&world).await?, ["Open", "To do"]);
    Ok(())
}
