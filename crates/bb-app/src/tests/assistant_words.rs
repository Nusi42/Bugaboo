//! A proposal in the words of the history and the way back of an applied change (AI-05): the card
//! shows a priority by its name and a status by its name, and an applied comment says which comment
//! *Undo* takes away.

use std::collections::BTreeMap;

use serde_json::json;

use super::ai_tools::issue_with;
use super::assistant::{ai_on, apply_of, ask, question, will_call, will_say, with_an_issue};
use super::scenarios::a_project;
use super::scenarios::token;
use super::search_fields::{bob_with_a_role, every_field};
use super::world::{Outcome, World};
use crate::ai::assistant::apply_proposal;
use crate::history::ChangeValue;
use crate::issues::{ChangeIssue, change_issue};

#[tokio::test]
async fn ai_05_a_proposal_comes_with_the_words_of_the_history() -> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "update_issue",
        json!({"issue": key, "version": 1, "priority": "high"}),
    );
    will_say(&world, "Proposed.");
    let answer = ask(&world, question(&token(&world), "Make it high")).await?;
    let proposal = answer.proposals.first().ok_or("no proposal")?;
    let words = proposal
        .words
        .iter()
        .find(|each| each.field == "priority")
        .ok_or("no words for the priority")?;
    assert_eq!(
        words.after,
        Some(ChangeValue::Text {
            text: "high".to_owned()
        })
    );
    Ok(())
}

#[tokio::test]
async fn ai_05_an_applied_comment_says_which_comment_undo_takes_away() -> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "add_comment",
        json!({"issue": key, "text": "Looks fine."}),
    );
    will_say(&world, "Proposed.");
    let answer = ask(
        &world,
        question(&token(&world), "Comment that it looks fine"),
    )
    .await?;
    let proposal = answer.proposals.first().ok_or("no proposal")?;
    let applied = apply_proposal(&world.ctx(), apply_of(&world, proposal))
        .await?
        .into_value();
    let undo = applied.undo.ok_or("no way back")?;
    assert_eq!(undo.kind, crate::ai::tools::UndoKind::Comment);
    assert!(!undo.id.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_05_a_status_change_is_worded_by_the_name_of_the_status() -> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "change_status",
        json!({"issue": key, "version": 1, "status": "done"}),
    );
    will_say(&world, "Proposed.");
    let answer = ask(&world, question(&token(&world), "Close WEB-1")).await?;
    let words = answer
        .proposals
        .first()
        .and_then(|proposal| proposal.words.iter().find(|each| each.field == "status"))
        .ok_or("no words for the status")?;
    let named = |value: &Option<ChangeValue>| match value {
        Some(ChangeValue::Text { text }) => !text.is_empty(),
        Some(ChangeValue::Names { names }) => !names.is_empty(),
        _ => false,
    };
    assert!(named(&words.before), "{:?}", words.before);
    assert!(named(&words.after), "{:?}", words.after);
    Ok(())
}

#[tokio::test]
async fn ai_05_people_in_a_proposal_are_worded_as_the_reader_may_see_them() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    let key = issue_with(&world, "WEB", "Login", "plain", BTreeMap::new()).await?;
    bob_with_a_role(&world, &every_field()).await?;
    let bob = world
        .store
        .lock()
        .state
        .people
        .iter()
        .find(|(person, _)| person.email == "bob@example.test")
        .map(|(person, _)| person.id.to_string())
        .ok_or("bob exists")?;
    will_call(
        &world,
        "update_issue",
        json!({"issue": key, "version": 1, "assigned_to": [bob]}),
    );
    will_say(&world, "Done.");
    let answer = ask(
        &world,
        question(&token(&world), "Assign WEB-1 to Bob Builder"),
    )
    .await?;
    let words = answer
        .proposals
        .first()
        .and_then(|proposal| {
            proposal
                .words
                .iter()
                .find(|each| each.field == "assigned-to")
        })
        .ok_or("no words for the people")?;
    match &words.after {
        Some(ChangeValue::People { people }) => assert_eq!(people.len(), 1),
        other => return Err(format!("not people: {other:?}").into()),
    }
    Ok(())
}

#[tokio::test]
async fn ai_05_the_version_after_apply_is_the_one_the_write_made_and_undo_on_it_is_refused_later()
-> Outcome {
    let (world, key) = with_an_issue().await?;
    will_call(
        &world,
        "update_issue",
        json!({"issue": key, "version": 1, "priority": "high"}),
    );
    will_say(&world, "Proposed.");
    let answer = ask(&world, question(&token(&world), "Make it high")).await?;
    let proposal = answer.proposals.first().ok_or("no proposal")?;
    let applied = apply_proposal(&world.ctx(), apply_of(&world, proposal))
        .await?
        .into_value();
    // The write counted the issue from 1 to 2, and says so itself.
    assert_eq!(applied.version, Some(2));
    // Somebody else changes the issue right after: the version of the write is not read again.
    change_issue(
        &world.ctx(),
        ChangeIssue {
            title: Some("Theirs".to_owned()),
            ..ChangeIssue::of(&token(&world), key.clone(), 2)
        },
    )
    .await?;
    let undo = change_issue(
        &world.ctx(),
        ChangeIssue {
            priority: Some(bb_core::issue::Priority::Normal),
            ..ChangeIssue::of(&token(&world), key, applied.version.ok_or("no version")?)
        },
    )
    .await;
    assert!(
        matches!(undo, Err(crate::auth::AppError::Stale(_))),
        "{undo:?}"
    );
    Ok(())
}
