//! The assistant cannot be made to call the provider without end (SEC-14): a person asks only so
//! many questions in a minute, and one question stops at a ceiling of rounds even under *No limit*.

use bb_core::ErrorCode;
use bb_core::ai::{QUESTIONS_A_MINUTE, ROUNDS_MOST};
use bb_core::settings::SettingValue;
use serde_json::json;

use super::assistant::{ask, question, will_say, with_an_issue};
use super::scenarios::token;
use super::world::Outcome;
use crate::ai::{AiAnswer, AiToolCall};
use crate::auth::AppError;
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

#[tokio::test]
async fn sec_14_a_person_asks_the_assistant_only_so_many_questions_a_minute() -> Outcome {
    let (world, _) = with_an_issue().await?;
    for _ in 0..QUESTIONS_A_MINUTE {
        will_say(&world, "Done.");
        ask(&world, question(&token(&world), "Again?")).await?;
    }
    let calls = world.ai.calls().len();
    let refused = ask(&world, question(&token(&world), "Again?")).await;
    assert!(
        matches!(refused, Err(AppError::Limited { .. })),
        "{refused:?}"
    );
    assert_eq!(
        world.ai.calls().len(),
        calls,
        "nothing went to the provider"
    );
    // The next minute takes questions again.
    world.clock.advance(60);
    will_say(&world, "Done.");
    ask(&world, question(&token(&world), "Again?")).await?;
    Ok(())
}

#[tokio::test]
async fn sec_14_no_limit_still_stops_a_question_at_the_ceiling_of_rounds() -> Outcome {
    let (world, _) = with_an_issue().await?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: "assistant.look-ups-most".to_owned(),
            value: SettingValue::Text("no-limit".to_owned()),
        },
    )
    .await?;
    for _ in 0..ROUNDS_MOST + 5 {
        world.ai.will_answer(AiAnswer {
            tool_calls: vec![AiToolCall {
                id: "c".to_owned(),
                name: "list_projects".to_owned(),
                arguments: json!({}),
            }],
            ..AiAnswer::default()
        });
    }
    let error = ask(&world, question(&token(&world), "Loop")).await;
    assert_eq!(
        error.err(),
        Some(AppError::Refused(ErrorCode::AiUnavailable))
    );
    assert_eq!(world.ai.calls().len(), ROUNDS_MOST);
    Ok(())
}
