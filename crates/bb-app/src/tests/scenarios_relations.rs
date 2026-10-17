//! The scenarios of the relation, comment and follow use cases (ADR-0007, M1-F18): the preparation,
//! the call, and how many events the call added. `events.rs` names each use case once and runs
//! them.

use bb_core::comment::TEXT_PART;
use bb_core::edit::EditWhat;
use bb_core::event::RecordKind;
use bb_core::id::{CommentId, EditorId, IssueId, RelationKindId};
use bb_core::relation::Direction;
use bb_core::technical::TechnicalName;

use super::fake::FakeStore;
use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::world::World;
use crate::auth::entity;
use crate::comments::{
    AddComment, DeleteComment, MentionsQuery, ReadMentions, RestoreComment, SaveComment,
    add_comment, delete_comment, mentions, read_mentions, restore_comment, save_comment,
};
use crate::editing::{LockOutcome, TakeLockQuery, take_lock};
use crate::follow::{FollowIssue, UnfollowIssue, follow_issue, unfollow_issue};
use crate::relations::{
    AddRelation, ChangeRelationKind, CreateRelationKind, CreateStartingRelationKinds,
    RelationKindsQuery, RemoveRelation, SetParent, add_relation, change_relation_kind,
    create_relation_kind, create_starting_relation_kinds, relation_kinds, remove_relation,
    set_parent,
};

/// The first kind of relation of the first start (*blocks*).
pub(crate) async fn a_kind(world: &World) -> Result<RelationKindId, String> {
    let page = relation_kinds(
        &world.ctx(),
        RelationKindsQuery {
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
        .ok_or_else(|| "no kind of relation on a started server".to_owned())
}

async fn two_issues(world: &World) -> Result<(IssueId, IssueId), String> {
    let project = a_project(world, "ACME").await?;
    Ok((
        an_issue(world, project).await?,
        an_issue(world, project).await?,
    ))
}

pub(crate) async fn create_relation_kind_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_relation_kind(
            &world.ctx(),
            CreateRelationKind {
                session_token: token(world),
                forward: "precedes".to_owned(),
                backward: "follows".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn change_relation_kind_scenario(world: &World) -> Added {
    let kind = a_kind(world).await?;
    measured(
        &world.store,
        change_relation_kind(
            &world.ctx(),
            ChangeRelationKind {
                session_token: token(world),
                kind,
                version: 1,
                forward: Some("stops".to_owned()),
                backward: None,
                retired: None,
            },
        ),
    )
    .await
}

/// A server that has not made its kinds of relation: the first start.
pub(crate) async fn create_starting_relation_kinds_scenario(_world: &World) -> Added {
    let bare = World::new_without_issue_types().await?;
    let store: &FakeStore = &bare.store;
    measured(
        store,
        create_starting_relation_kinds(&bare.ctx(), CreateStartingRelationKinds),
    )
    .await
}

pub(crate) async fn add_relation_scenario(world: &World) -> Added {
    let (first, second) = two_issues(world).await?;
    let kind = a_kind(world).await?;
    measured(
        &world.store,
        add_relation(
            &world.ctx(),
            AddRelation {
                session_token: token(world),
                issue: first.to_string(),
                other: second.to_string(),
                kind,
                direction: Direction::Forward,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_relation_scenario(world: &World) -> Added {
    let (first, second) = two_issues(world).await?;
    let kind = a_kind(world).await?;
    let relation = add_relation(
        &world.ctx(),
        AddRelation {
            session_token: token(world),
            issue: first.to_string(),
            other: second.to_string(),
            kind,
            direction: Direction::Backward,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    measured(
        &world.store,
        remove_relation(
            &world.ctx(),
            RemoveRelation {
                session_token: token(world),
                relation,
            },
        ),
    )
    .await
}

pub(crate) async fn set_parent_scenario(world: &World) -> Added {
    let (child, parent) = two_issues(world).await?;
    measured(
        &world.store,
        set_parent(
            &world.ctx(),
            SetParent {
                session_token: token(world),
                issue: child.to_string(),
                version: 1,
                parent: Some(parent.to_string()),
            },
        ),
    )
    .await
}

pub(crate) async fn a_comment(world: &World, issue: IssueId) -> Result<CommentId, String> {
    Ok(add_comment(
        &world.ctx(),
        AddComment {
            session_token: token(world),
            issue: issue.to_string(),
            text: "It happens every time.".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

pub(crate) async fn add_comment_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        add_comment(
            &world.ctx(),
            AddComment {
                session_token: token(world),
                issue: issue.to_string(),
                text: "It happens every time.".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn save_comment_scenario(_world: &World) -> Added {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let comment = a_comment(&world, issue).await?;
    let what = EditWhat {
        record: entity(RecordKind::Comment, comment),
        part: TechnicalName::new(TEXT_PART).map_err(text)?,
    };
    let LockOutcome::Taken(lock) = take_lock(
        &world.ctx(),
        TakeLockQuery {
            session_token: token(&world),
            what,
            editor: EditorId::from_random([5; 16]),
        },
    )
    .await
    .map_err(text)?
    .into_value() else {
        return Err("nobody else is editing the comment".to_owned());
    };
    measured(
        &world.store,
        save_comment(
            &world.ctx(),
            SaveComment {
                session_token: token(&world),
                comment,
                version: 1,
                lock_token: lock.token,
                text: "It happens every other time.".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_comment_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let comment = a_comment(world, issue).await?;
    measured(
        &world.store,
        delete_comment(
            &world.ctx(),
            DeleteComment {
                session_token: token(world),
                comment,
            },
        ),
    )
    .await
}

pub(crate) async fn restore_comment_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let comment = a_comment(world, issue).await?;
    delete_comment(
        &world.ctx(),
        DeleteComment {
            session_token: token(world),
            comment,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        restore_comment(
            &world.ctx(),
            RestoreComment {
                session_token: token(world),
                comment,
            },
        ),
    )
    .await
}

pub(crate) async fn read_mentions_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let comment = a_comment(world, issue).await?;
    // The page of mentions is a read; reading it is not the command.
    mentions(
        &world.ctx(),
        MentionsQuery {
            session_token: token(world),
            limit: 10,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        read_mentions(
            &world.ctx(),
            ReadMentions {
                session_token: token(world),
                comments: vec![comment],
            },
        ),
    )
    .await
}

pub(crate) async fn unfollow_issue_scenario(world: &World) -> Added {
    // Ada follows what she reported.
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        unfollow_issue(
            &world.ctx(),
            UnfollowIssue {
                session_token: token(world),
                issue: issue.to_string(),
            },
        ),
    )
    .await
}

pub(crate) async fn follow_issue_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    unfollow_issue(
        &world.ctx(),
        UnfollowIssue {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        follow_issue(
            &world.ctx(),
            FollowIssue {
                session_token: token(world),
                issue: issue.to_string(),
            },
        ),
    )
    .await
}
