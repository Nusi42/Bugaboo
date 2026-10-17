//! The scenarios of the issue use cases (ADR-0007, M1-F18): the preparation, the call, and how many
//! events the call added. `events.rs` names each use case once and runs them.

use bb_core::edit::EditWhat;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{EditorId, EntityId, IssueId, IssueTypeId, ProjectId};
use bb_core::issue::IssueIcon;
use bb_core::message::{self, Message};
use bb_core::technical::TechnicalName;
use bb_core::workflow::starter;

use super::fake::FakeStore;
use super::scenarios::{Added, a_project, measured, text, token};
use super::world::World;
use crate::editing::{LockOutcome, TakeLockQuery, take_lock};
use crate::issues::{
    ChangeIssue, ChangeIssueType, CreateIssue, CreateIssueType, CreateStartingIssueTypes,
    DeleteIssue, IssueTypesQuery, MoveIssue, RestoreIssue, SaveDescription, change_issue,
    change_issue_type, create_issue, create_issue_type, create_starting_issue_types, delete_issue,
    issue_types, move_issue, restore_issue, save_description,
};
use crate::many_changes::{
    ApplyManyChanges, ManyChange, ManyChangeAction, ManyChangeTarget, apply_many_changes,
};

/// An issue type of the first start: *Task*, which uses the workflow *Simple*, where any status can
/// follow any other.
pub(crate) async fn a_type(world: &World) -> Result<IssueTypeId, String> {
    let page = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            session_token: token(world),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    page.items
        .iter()
        .find(|found| found.name == Message::from(message::ISSUE_TYPE_TASK).render())
        .map(|found| found.id)
        .ok_or_else(|| "no issue type Task on a started server".to_owned())
}

/// An issue of the project, made by Ada.
pub(crate) async fn an_issue(world: &World, project: ProjectId) -> Result<IssueId, String> {
    Ok(create_issue(
        &world.ctx(),
        CreateIssue {
            session_token: token(world),
            project: project.to_string(),
            issue_type: a_type(world).await?,
            title: "The login button does nothing".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

pub(crate) async fn create_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    measured(
        &world.store,
        create_issue(
            &world.ctx(),
            CreateIssue {
                session_token: token(world),
                project: project.to_string(),
                issue_type: a_type(world).await?,
                title: "The login button does nothing".to_owned(),
                description: Some("Press it and wait.".to_owned()),
                status: Some(starter::in_progress()),
                priority: None,
                assigned_to: Some(vec![world.admin.person.id]),
                fields: std::collections::BTreeMap::new(),
            },
        ),
    )
    .await
}

pub(crate) async fn change_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        change_issue(
            &world.ctx(),
            ChangeIssue {
                session_token: token(world),
                issue: issue.to_string(),
                version: 1,
                issue_type: None,
                title: Some("The login button does nothing at all".to_owned()),
                status: Some(starter::done()),
                priority: None,
                assigned_to: None,
                fields: std::collections::BTreeMap::new(),
            },
        ),
    )
    .await
}

pub(crate) async fn save_description_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let taken = take_lock(
        &world.ctx(),
        TakeLockQuery {
            session_token: token(world),
            what: EditWhat {
                record: EntityRef {
                    kind: RecordKind::Issue.into(),
                    id: EntityId::from(issue),
                },
                part: TechnicalName::new("description").map_err(text)?,
            },
            editor: EditorId::from_random([5; 16]),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let LockOutcome::Taken(lock) = taken else {
        return Err("the lock was not free".to_owned());
    };
    measured(
        &world.store,
        save_description(
            &world.ctx(),
            SaveDescription {
                session_token: token(world),
                issue: issue.to_string(),
                version: 1,
                lock_token: lock.token,
                text: "It fails on Monday.".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        delete_issue(
            &world.ctx(),
            DeleteIssue {
                session_token: token(world),
                issue: issue.to_string(),
            },
        ),
    )
    .await
}

pub(crate) async fn restore_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    delete_issue(
        &world.ctx(),
        DeleteIssue {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        restore_issue(
            &world.ctx(),
            RestoreIssue {
                session_token: token(world),
                issue: issue.to_string(),
            },
        ),
    )
    .await
}

pub(crate) async fn create_issue_type_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_issue_type(
            &world.ctx(),
            CreateIssueType {
                session_token: token(world),
                name: "Question".to_owned(),
                icon: IssueIcon::CircleHelp,
            },
        ),
    )
    .await
}

pub(crate) async fn change_issue_type_scenario(world: &World) -> Added {
    let issue_type = a_type(world).await?;
    measured(
        &world.store,
        change_issue_type(
            &world.ctx(),
            ChangeIssueType {
                session_token: token(world),
                issue_type,
                version: 1,
                name: Some("Defect".to_owned()),
                icon: None,
                retired: Some(true),
            },
        ),
    )
    .await
}

/// A server that has not made its issue types: the first start.
pub(crate) async fn create_starting_issue_types_scenario(_world: &World) -> Added {
    let bare = World::new_without_issue_types().await?;
    let store: &FakeStore = &bare.store;
    measured(
        store,
        create_starting_issue_types(&bare.ctx(), CreateStartingIssueTypes),
    )
    .await
}

pub(crate) async fn move_issue_scenario(world: &World) -> Added {
    let (acme, other) = (
        a_project(world, "ACME").await?,
        a_project(world, "ELSE").await?,
    );
    let issue = an_issue(world, acme).await?;
    measured(
        &world.store,
        move_issue(
            &world.ctx(),
            MoveIssue {
                session_token: token(world),
                issue: issue.to_string(),
                version: 1,
                project: other.to_string(),
            },
        ),
    )
    .await
}

pub(crate) async fn apply_many_changes_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        apply_many_changes(
            &world.ctx(),
            ApplyManyChanges {
                session_token: token(world),
                issues: vec![ManyChangeTarget {
                    issue: issue.to_string(),
                    version: 1,
                }],
                action: ManyChangeAction::Change {
                    change: ManyChange {
                        status: Some(starter::done()),
                        ..ManyChange::default()
                    },
                },
            },
        ),
    )
    .await
}
