//! What a store may hold (M3-F78): at most `RELATIONS_MAX` relations and `SUB_ISSUES_MAX` sub-issues
//! for an issue, `FIELDS_MAX` own fields and `CHOICES_MAX` choices of a field; counted in the change
//! itself, with every row, also rows the person may not see. A full store answers `limit.reached`.

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, Assignment, Group, GroupMember};
use bb_core::comment::MENTION_RECIPIENTS_MAX;
use bb_core::id::{AssignmentId, GroupId, IssueId, RelationId, UserId};
use bb_core::relation::{Direction, IssueParent, RELATIONS_MAX, Relation, SUB_ISSUES_MAX};
use jiff::Timestamp;

use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::comments::{AddComment, add_comment};
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

/// A group of more people than one comment reaches, who all see the issue.
fn a_big_group(world: &World) -> usize {
    let ctx = world.ctx();
    let mut ids = ctx.ids();
    let mut shared = world.store.lock();
    let role = shared
        .state
        .roles
        .iter()
        .find(|record| record.role.starting.is_some())
        .map(|record| record.role.id)
        .expect("a starting role");
    let group = GroupId::generate(&mut ids).expect("an ID");
    shared.state.groups.push(Group {
        id: group,
        name: "Everybody".to_owned(),
        version: 1,
    });
    shared.state.assignments.push(Assignment {
        id: AssignmentId::generate(&mut ids).expect("an ID"),
        person: None,
        group: Some(group),
        role: Some(role),
        project: None,
        applies_to: AppliesTo::Organization,
        given_at: Timestamp::UNIX_EPOCH,
    });
    for _ in 0..=MENTION_RECIPIENTS_MAX {
        shared.state.members.push(GroupMember {
            group,
            person: UserId::generate(&mut ids),
            added_at: Timestamp::UNIX_EPOCH,
        });
    }
    MENTION_RECIPIENTS_MAX + 1
}

#[tokio::test]
async fn iss_05_a_comment_reaches_no_more_people_than_the_limit_and_is_kept() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await.map_err(text)?;
    let issue = an_issue(&world, project).await.map_err(text)?;
    let members = a_big_group(&world);
    let written = add_comment(
        &world.ctx(),
        AddComment {
            session_token: token(&world),
            issue: issue.to_string(),
            text: "Please look, @everybody.".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    // The comment is written whole; only the people it tells are cut off at the limit, and each
    // of them once.
    let shared = world.store.lock();
    assert_eq!(shared.state.comments.len(), 1);
    let rows: Vec<_> = shared
        .state
        .mentions
        .iter()
        .filter(|row| row.comment == written.id)
        .collect();
    assert!(members > MENTION_RECIPIENTS_MAX);
    assert_eq!(rows.len(), MENTION_RECIPIENTS_MAX);
    let distinct: std::collections::BTreeSet<_> = rows.iter().map(|row| row.recipient).collect();
    assert_eq!(distinct.len(), MENTION_RECIPIENTS_MAX);
    Ok(())
}

#[test]
fn iss_05_a_longest_text_of_four_byte_characters_is_shown_whole() {
    // The routes count characters and the renderer bytes: a text the routes keep must not be cut.
    let longest = "😀".repeat(bb_core::text::LONG_TEXT_MAX);
    assert!(crate::input::clean_long_text(&longest).is_ok());
    let rendered = bb_markdown::render(&longest, &bb_markdown::Options::default());
    assert!(!rendered.truncated, "a text the routes keep was cut");
}

#[tokio::test]
async fn cf_01_a_full_store_of_fields_and_choices_answers_limit_reached() -> Outcome {
    use bb_core::own_fields::{CHOICES_MAX, FIELDS_MAX, FieldType};

    use super::scenarios_fields::{a_field, new_field};
    use crate::fields::{AddChoice, add_choice, create_field};

    let world = World::new().await?;
    // A field with as many choices as it may hold takes no more.
    let names: Vec<String> = (0..CHOICES_MAX)
        .map(|number| format!("Level {number}"))
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let crowded = a_field(&world, "Crowded", FieldType::PickOne, &names)
        .await
        .map_err(text)?;
    let refused = add_choice(
        &world.ctx(),
        AddChoice {
            session_token: token(&world),
            field: crowded.id,
            version: crowded.version,
            name: "One more".to_owned(),
            color: None,
            icon: None,
        },
    )
    .await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::LimitReached))
    );
    // The organization holds as many fields as it may, and no more.
    for number in 1..FIELDS_MAX {
        a_field(&world, &format!("Field {number}"), FieldType::Text, &[])
            .await
            .map_err(text)?;
    }
    let refused = create_field(
        &world.ctx(),
        new_field(&world, "One field too many", FieldType::Text, &[]),
    )
    .await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::LimitReached))
    );
    Ok(())
}
