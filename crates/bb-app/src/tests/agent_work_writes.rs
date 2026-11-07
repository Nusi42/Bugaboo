//! The writes beside the change of an issue that an agent may do only when it was given them
//! (ADR-0029, decision 3): taking a relation away, deleting or restoring a file, saving a long
//! text, restoring an issue. Each is refused without the tick and works with it where an agent can
//! reach it at all.

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::id::LockToken;
use bb_core::own_fields::FieldType;
use bb_core::relation::Direction;

use super::agent_work::{Handed, handed_with, plain_hand_over};
use super::agents::refused;
use super::scenarios::token;
use super::scenarios_fields::a_field;
use super::scenarios_files::{A_FILE, an_attachment};
use super::scenarios_issues::an_issue;
use super::scenarios_relations::a_kind;
use super::world::{Outcome, World};
use crate::agent_work::{HandOver, hand_over};
use crate::fields::{SaveFieldText, save_field_text};
use crate::files::{DeleteAttachment, RestoreAttachment, delete_attachment, restore_attachment};
use crate::issues::{DeleteIssue, RestoreIssue, delete_issue, restore_issue};
use crate::relations::{AddRelation, RemoveRelation, add_relation, remove_relation};

/// A hand-over that ticks these members of the issue.
fn ticking(members: &[&str]) -> impl FnOnce(&mut HandOver) {
    let members: Vec<String> = members.iter().map(|each| (*each).to_owned()).collect();
    move |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = members;
    }
}

#[tokio::test]
async fn ai_31_an_agent_deletes_and_restores_a_file_only_when_it_was_given_attachments() -> Outcome
{
    for ticked in [false, true] {
        let world = World::new().await?;
        let given = handed_with(
            &world,
            ticking(if ticked { &["attachments"] } else { &["title"] }),
        )
        .await?;
        let file = an_attachment(&world, given.issue, "jam.txt", A_FILE).await?;
        let delete = delete_attachment(
            &world.ctx(),
            DeleteAttachment {
                session_token: given.key.clone(),
                attachment: file,
            },
        )
        .await;
        if !ticked {
            assert!(refused(&delete.unwrap_err(), ErrorCode::Forbidden));
            continue;
        }
        delete?;
        restore_attachment(
            &world.ctx(),
            RestoreAttachment {
                session_token: given.key.clone(),
                attachment: file,
            },
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_restores_no_file_without_attachments_given() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, ticking(&["title"])).await?;
    let file = an_attachment(&world, given.issue, "jam.txt", A_FILE).await?;
    delete_attachment(
        &world.ctx(),
        DeleteAttachment {
            session_token: token(&world),
            attachment: file,
        },
    )
    .await?;
    let restore = restore_attachment(
        &world.ctx(),
        RestoreAttachment {
            session_token: given.key.clone(),
            attachment: file,
        },
    )
    .await;
    assert!(restore.is_err(), "the file stays in the trash");
    Ok(())
}

/// A hand-over that gives the agent two issues, the first with these members ticked.
async fn two_handed(
    world: &World,
    members: &[&str],
) -> Result<(Handed, bb_core::id::IssueId), String> {
    let first = handed_with(world, ticking(members))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let second = an_issue(world, first.project).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&first.team.session_token, first.agent, &[second]),
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    Ok((first, second))
}

#[tokio::test]
async fn ai_31_an_agent_takes_a_relation_away_only_when_it_was_given_relations() -> Outcome {
    for ticked in [false, true] {
        let world = World::new().await?;
        let (given, second) =
            two_handed(&world, if ticked { &["relations"] } else { &["title"] }).await?;
        let relation = add_relation(
            &world.ctx(),
            AddRelation {
                session_token: token(&world),
                issue: given.issue.to_string(),
                other: second.to_string(),
                kind: a_kind(&world).await?,
                direction: Direction::Forward,
            },
        )
        .await?
        .into_value();
        let removal = remove_relation(
            &world.ctx(),
            RemoveRelation {
                session_token: given.key.clone(),
                relation: relation.id,
            },
        )
        .await;
        if ticked {
            removal?;
        } else {
            assert!(refused(&removal.unwrap_err(), ErrorCode::Forbidden));
        }
    }
    Ok(())
}

#[tokio::test]
async fn ai_31_an_agent_never_saves_a_long_text_or_restores_an_issue() -> Outcome {
    let world = World::new().await?;
    let long = a_field(&world, "Notes", FieldType::LongText, &[]).await?;
    let given = handed_with(&world, ticking(&["title"])).await?;
    // A lock belongs to a person in a browser tab, never to a key.
    let saved = save_field_text(
        &world.ctx(),
        SaveFieldText {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            field: long.id,
            token: LockToken::from_random([7; 16]),
            version: 1,
            text: "Written by the agent".to_owned(),
        },
    )
    .await;
    assert!(saved.is_err());
    // An issue in the trash is not the agent's to bring back.
    delete_issue(
        &world.ctx(),
        DeleteIssue {
            session_token: token(&world),
            issue: given.issue.to_string(),
        },
    )
    .await?;
    let restored = restore_issue(
        &world.ctx(),
        RestoreIssue {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
        },
    )
    .await;
    assert!(restored.is_err());
    Ok(())
}
