//! The scenarios of the issue use cases (ADR-0007, M1-F18): the preparation, the call, and how many
//! events the call added. `events.rs` names each use case once and runs them.

use bb_core::edit::EditWhat;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{EditorId, EntityId, IssueId, IssueTypeId, ProjectId};
use bb_core::issue::{IssueIcon, Status};
use bb_core::technical::TechnicalName;

use super::fake::FakeStore;
use super::scenarios::{Added, a_project, measured, text, token};
use super::world::World;
use crate::editing::{LockOutcome, TakeLockQuery, take_lock};
use crate::issues::{
    ChangeIssue, ChangeIssueType, DeleteIssue, IssueTypesQuery, MakeIssue, MakeIssueType,
    MakeStartingIssueTypes, RestoreIssue, SaveDescription, change_issue, change_issue_type,
    delete_issue, issue_types, make_issue, make_issue_type, make_starting_issue_types,
    restore_issue, save_description,
};

/// The first issue type of the first start (*Bug*).
async fn a_type(world: &World) -> Result<IssueTypeId, String> {
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
        .first()
        .map(|found| found.id)
        .ok_or_else(|| "no issue type on a started server".to_owned())
}

/// An issue of the project, made by Ada.
pub(crate) async fn an_issue(world: &World, project: ProjectId) -> Result<IssueId, String> {
    Ok(make_issue(
        &world.ctx(),
        MakeIssue {
            session_token: token(world),
            project: project.to_string(),
            issue_type: a_type(world).await?,
            title: "The login button does nothing".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: None,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

pub(crate) async fn make_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    measured(
        &world.store,
        make_issue(
            &world.ctx(),
            MakeIssue {
                session_token: token(world),
                project: project.to_string(),
                issue_type: a_type(world).await?,
                title: "The login button does nothing".to_owned(),
                description: Some("Press it and wait.".to_owned()),
                status: Some(Status::InProgress),
                priority: None,
                assigned_to: Some(vec![world.admin.person.id]),
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
                status: Some(Status::Done),
                priority: None,
                assigned_to: None,
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

pub(crate) async fn make_issue_type_scenario(world: &World) -> Added {
    measured(
        &world.store,
        make_issue_type(
            &world.ctx(),
            MakeIssueType {
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
pub(crate) async fn make_starting_issue_types_scenario(_world: &World) -> Added {
    let bare = World::new_without_issue_types().await?;
    let store: &FakeStore = &bare.store;
    measured(
        store,
        make_starting_issue_types(&bare.ctx(), MakeStartingIssueTypes),
    )
    .await
}
