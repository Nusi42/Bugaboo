//! The history and the notifications name an issue type or a label only where the reader may know
//! it (LM1 of the M4 exit review, SEC-03, SEC-06): one rule, `offers/known.rs`. An issue that was
//! of a type made in ACME and moves to OTHER does not tell OTHER the name of that type or of its
//! labels.

use std::collections::BTreeMap;

use bb_core::history::HistoryTab;
use bb_core::id::{IssueId, IssueTypeId};
use bb_core::message::{self, Message};

use super::offers::{Groups, groups, label_on, lead_of};
use super::scenarios_issues::{a_type, an_issue};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::history::{HistoryQuery, history};
use crate::issues::{ChangeIssue, IssueQuery, MoveIssue, change_issue, issue, move_issue};
use crate::tags::{RemoveLabel, remove_label};

async fn version_of(world: &World, who: &str, issue_id: IssueId) -> Result<i64, AppError> {
    Ok(issue(
        &world.ctx(),
        IssueQuery {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .version)
}

async fn type_to(
    world: &World,
    who: &str,
    issue_id: IssueId,
    issue_type: IssueTypeId,
) -> Result<(), AppError> {
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
            version: version_of(world, who, issue_id).await?,
            issue_type: Some(issue_type),
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?;
    Ok(())
}

/// What the history of the issue says to the reader, as one text (the debug form of its rows).
async fn read(world: &World, who: &str, issue_id: IssueId) -> Result<String, AppError> {
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
            tab: HistoryTab::Changes,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    Ok(format!("{:?}", page.items))
}

#[tokio::test]
async fn sec_03_the_history_names_a_type_and_a_label_only_where_the_reader_may_know_them() -> Outcome
{
    let Groups {
        world,
        admin,
        lead,
        acme,
        other,
        login,
        ..
    } = groups().await?;
    let question = super::offers_secrecy::new_type(&world, &lead, Some("ACME"), "Question").await?;
    let task = a_type(&world).await?;
    let issue_id = an_issue(&world, acme).await?;
    // The issue was a Question with the label login, then became a Task, and moved to OTHER.
    type_to(&world, &admin, issue_id, question.id).await?;
    label_on(&world, &admin, issue_id, login).await?;
    type_to(&world, &admin, issue_id, task).await?;
    remove_label(
        &world.ctx(),
        RemoveLabel {
            session_token: admin.clone(),
            issue: issue_id.to_string(),
            label: login,
        },
    )
    .await?;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&world, &admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let outsider = lead_of(&world, "sam@example.test", other)
        .await?
        .session_token;

    // The administrator knows everything and reads both names.
    let inside = read(&world, &admin, issue_id).await?;
    assert!(inside.contains("Question"), "{inside}");
    assert!(inside.contains("names: [\"login\"]"), "{inside}");
    // Who works in OTHER reads neither, and a neutral word where the type was.
    let outside = read(&world, &outsider, issue_id).await?;
    assert!(!outside.contains("Question"), "{outside}");
    assert!(!outside.contains("names: [\"login\"]"), "{outside}");
    let neutral = Message::from(message::HISTORY_TYPE_UNKNOWN).render();
    assert!(outside.contains(&neutral), "{outside}");
    Ok(())
}
