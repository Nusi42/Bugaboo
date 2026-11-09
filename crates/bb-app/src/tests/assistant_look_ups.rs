//! M7 WP-15: how many tools the assistant may run for one question is a setting. An administrator
//! sets the most (also *No limit*; 8 at the start), a person may set a lower number for themselves
//! and never a higher one than the organization's.

use bb_core::ErrorCode;

use bb_core::settings::SettingValue;
use serde_json::json;

use super::assistant::{ask, question, with_an_issue};
use super::scenarios::token;
use super::world::{Outcome, World};
use crate::ai::{AiAnswer, AiToolCall};
use crate::auth::AppError;
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

const ORGANIZATION: &str = "assistant.look-ups-most";
const PERSON: &str = "assistant.look-ups";

async fn set(world: &World, scope: SettingsScope, key: &str, value: &str) -> Result<(), AppError> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope,
            key: key.to_owned(),
            value: SettingValue::Text(value.to_owned()),
        },
    )
    .await?;
    Ok(())
}

fn will_look_up(world: &World, times: usize) {
    for _ in 0..times {
        world.ai.will_answer(AiAnswer {
            tool_calls: vec![AiToolCall {
                id: "c".to_owned(),
                name: "list_projects".to_owned(),
                arguments: json!({}),
            }],
            ..AiAnswer::default()
        });
    }
}

/// A model that never stops asking: the tools it ran, and the calls to the provider.
async fn endless(world: &World) -> Result<(usize, usize), String> {
    will_look_up(world, 200);
    let error = ask(world, question(&token(world), "Loop"))
        .await
        .unwrap_err();
    assert_eq!(error, AppError::Refused(ErrorCode::AiUnavailable));
    Ok((world.store.lock().tool_calls.len(), world.ai.calls().len()))
}

#[tokio::test]
async fn m7_wp15_the_organization_sets_the_most_look_ups() -> Outcome {
    let (world, _) = with_an_issue().await?;
    set(&world, SettingsScope::Organization, ORGANIZATION, "2").await?;
    assert_eq!(endless(&world).await?, (2, 3));
    Ok(())
}

#[tokio::test]
async fn m7_wp15_the_most_look_ups_is_eight_at_the_start() -> Outcome {
    let (world, _) = with_an_issue().await?;
    assert_eq!(endless(&world).await?, (8, 9));
    Ok(())
}

#[tokio::test]
async fn m7_wp15_no_limit_lets_a_question_look_up_more_than_the_usual_most() -> Outcome {
    let (world, _) = with_an_issue().await?;
    set(
        &world,
        SettingsScope::Organization,
        ORGANIZATION,
        "no-limit",
    )
    .await?;
    will_look_up(&world, 12);
    super::assistant::will_say(&world, "Done.");
    let answer = ask(&world, question(&token(&world), "Look at everything")).await?;
    assert_eq!(answer.text, "Done.");
    assert_eq!(world.store.lock().tool_calls.len(), 12);
    Ok(())
}

#[tokio::test]
async fn m7_wp15_a_person_can_set_a_lower_number_for_themselves() -> Outcome {
    let (world, _) = with_an_issue().await?;
    set(&world, SettingsScope::Person, PERSON, "2").await?;
    assert_eq!(endless(&world).await?, (2, 3));
    Ok(())
}

#[tokio::test]
async fn m7_wp15_a_person_cannot_go_above_the_most_of_the_organization() -> Outcome {
    let (world, _) = with_an_issue().await?;
    set(&world, SettingsScope::Organization, ORGANIZATION, "4").await?;
    set(&world, SettingsScope::Person, PERSON, "20").await?;
    assert_eq!(endless(&world).await?, (4, 5));
    Ok(())
}

#[tokio::test]
async fn m7_wp15_a_person_who_chose_no_number_gets_the_one_of_the_organization() -> Outcome {
    let (world, _) = with_an_issue().await?;
    set(&world, SettingsScope::Organization, ORGANIZATION, "4").await?;
    set(&world, SettingsScope::Person, PERSON, "as-organization").await?;
    assert_eq!(endless(&world).await?, (4, 5));
    Ok(())
}

#[tokio::test]
async fn m7_wp15_only_administrators_set_the_most() -> Outcome {
    let (world, _) = with_an_issue().await?;
    let bob = world.person("bob@example.test").await?;
    let refused = change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: bob.session_token,
            scope: SettingsScope::Organization,
            key: ORGANIZATION.to_owned(),
            value: SettingValue::Text("50".to_owned()),
        },
    )
    .await;
    assert!(refused.is_err());
    Ok(())
}
