//! A proposal in the words of the history and the way back of an applied change (AI-05): the card
//! shows a priority by its name and a status by its name, and an applied comment says which comment
//! *Undo* takes away.

use serde_json::json;

use super::assistant::{apply_of, ask, question, will_call, will_say, with_an_issue};
use super::scenarios::token;
use super::world::Outcome;
use crate::ai::assistant::apply_proposal;
use crate::history::ChangeValue;

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
    assert_eq!(undo.kind, "comment");
    assert!(!undo.id.is_empty());
    Ok(())
}
