//! Whether the assistant is there for a person (AI-02, PRIV-08, COMP-08): the answer the button, the
//! panel and quick search ask. It follows the switch of the organization and of the project, says
//! where the answers come from, and sends nothing to the provider.

use bb_core::ErrorCode;
use bb_core::ai::Destination;

use super::ai_tools::mark;
use super::assistant::{ai_on, outside_provider};
use super::scenarios::{a_project, token};
use super::scenarios_ai::{a_local_model, setup};
use super::world::{Outcome, World};
use crate::ai::assistant::{AssistantStatus, AssistantStatusQuery, assistant_status};
use crate::ai::{
    ChangeAiSetup, ChangeProjectAi, NeverSentTarget, change_ai_setup, change_project_ai,
};
use crate::auth::AppError;

async fn status(world: &World, project: Option<&str>) -> Result<AssistantStatus, AppError> {
    Ok(assistant_status(
        &world.ctx(),
        AssistantStatusQuery {
            session_token: token(world),
            project: project.map(str::to_owned),
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn ai_02_the_assistant_is_not_there_while_ai_is_off() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    a_local_model(&world).await?;
    let found = status(&world, None).await?;
    assert!(!found.available);
    assert_eq!(found.destination, None);
    assert!(world.ai.calls().is_empty(), "nothing was sent");
    Ok(())
}

#[tokio::test]
async fn ai_02_a_project_that_switched_ai_off_has_no_assistant() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    a_project(&world, "OPS").await?;
    ai_on(&world).await?;
    change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "WEB".to_owned(),
            enabled: Some(false),
        },
    )
    .await?;
    assert!(status(&world, None).await?.available);
    assert!(status(&world, Some("OPS")).await?.available);
    assert!(!status(&world, Some("WEB")).await?.available);
    Ok(())
}

#[tokio::test]
async fn comp_08_the_status_says_where_the_answers_come_from() -> Outcome {
    // A model on the organization's own network: names go with it, nothing leaves.
    let own = World::new().await?;
    a_project(&own, "WEB").await?;
    ai_on(&own).await?;
    let local = status(&own, None).await?;
    assert_eq!(local.destination, Some(Destination::LocalModel));
    assert!(local.names_sent);
    // An outside provider: people go as their role until an administrator switches names on.
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    outside_provider(&world).await?;
    let outside = status(&world, None).await?;
    assert_eq!(outside.destination, Some(Destination::Anthropic));
    assert!(!outside.names_sent);
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            send_names: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    assert!(status(&world, None).await?.names_sent);
    Ok(())
}

#[tokio::test]
async fn ai_03_a_project_kept_from_ai_answers_not_found() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    ai_on(&world).await?;
    mark(&world, NeverSentTarget::Project("WEB".to_owned())).await?;
    let refused = status(&world, Some("WEB")).await.unwrap_err();
    assert_eq!(refused, AppError::Refused(ErrorCode::NotFound));
    Ok(())
}
