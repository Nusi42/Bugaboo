//! What one issue may hold (M3-F68): at most `RELATIONS_MAX` relations and `SUB_ISSUES_MAX`
//! sub-issues, counted in the change itself and with every row, also rows the person may not see.

use bb_core::ErrorCode;
use bb_core::id::{IssueId, RelationId, UserId};
use bb_core::relation::{Direction, IssueParent, RELATIONS_MAX, Relation, SUB_ISSUES_MAX};
use jiff::Timestamp;

use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::relations::{
    AddRelation, RelationKindsQuery, SetParent, add_relation, relation_kinds, set_parent,
};

fn full_of_relations(world: &World, issue: IssueId, kind: bb_core::id::RelationKindId) {
    let ctx = world.ctx();
    let mut ids = ctx.ids();
    let mut shared = world.store.lock();
    for _ in 0..RELATIONS_MAX {
        let relation = Relation {
            id: RelationId::generate(&mut ids).expect("an ID"),
            kind,
            from: issue,
            to: IssueId::generate(&mut ids).expect("an ID"),
            added_by: UserId::generate(&mut ids),
            added_at: Timestamp::UNIX_EPOCH,
        };
        shared.state.relations.push(relation);
    }
}

#[tokio::test]
async fn iss_04_an_issue_holds_no_more_relations_than_the_limit() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await.map_err(text)?;
    let full = an_issue(&world, project).await.map_err(text)?;
    let other = an_issue(&world, project).await.map_err(text)?;
    let kinds = relation_kinds(
        &world.ctx(),
        RelationKindsQuery {
            session_token: token(&world),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    let kind = kinds.items.first().ok_or("no kind")?.id;
    full_of_relations(&world, full, kind);
    let refused = add_relation(
        &world.ctx(),
        AddRelation {
            session_token: token(&world),
            issue: full.to_string(),
            other: other.to_string(),
            kind,
            direction: Direction::Forward,
        },
    )
    .await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::LimitReached))
    );
    // Nothing was added.
    assert_eq!(world.store.lock().state.relations.len(), RELATIONS_MAX);
    Ok(())
}

#[tokio::test]
async fn iss_03_an_issue_holds_no_more_sub_issues_than_the_limit() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await.map_err(text)?;
    let parent = an_issue(&world, project).await.map_err(text)?;
    let child = an_issue(&world, project).await.map_err(text)?;
    {
        let ctx = world.ctx();
        let mut ids = ctx.ids();
        let mut shared = world.store.lock();
        for _ in 0..SUB_ISSUES_MAX {
            let row = IssueParent {
                child: IssueId::generate(&mut ids).expect("an ID"),
                parent,
                set_by: UserId::generate(&mut ids),
                set_at: Timestamp::UNIX_EPOCH,
            };
            shared.state.parents.push(row);
        }
    }
    let refused = set_parent(
        &world.ctx(),
        SetParent {
            session_token: token(&world),
            issue: child.to_string(),
            version: 1,
            parent: Some(parent.to_string()),
        },
    )
    .await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::LimitReached))
    );
    assert_eq!(world.store.lock().state.parents.len(), SUB_ISSUES_MAX);
    Ok(())
}
