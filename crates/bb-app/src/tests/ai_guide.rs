//! The guide to AI as a page of the app (COMP-11, M7-F90): the text of `userDocs/ai/literacy.md`,
//! rendered, for whoever is signed in.

use super::scenarios::token;
use super::world::{Outcome, World};
use crate::ai::{AiGuideQuery, ai_guide};

#[tokio::test]
async fn comp_11_a_signed_in_person_reads_the_guide_without_a_dead_link() -> Outcome {
    let world = World::new().await?;
    let guide = ai_guide(
        &world.ctx(),
        AiGuideQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(guide.html.contains("<h1>The AI guide"), "{}", guide.html);
    assert!(guide.html.contains("Never sent to AI"));
    assert!(
        !guide.html.contains("href=\"numbers.md\"") && !guide.html.contains(".md\""),
        "a page of the docs that the app does not have is not a link"
    );
    Ok(())
}

#[tokio::test]
async fn comp_11_nobody_reads_the_guide_without_signing_in() -> Outcome {
    let world = World::new().await?;
    let refused = ai_guide(
        &world.ctx(),
        AiGuideQuery {
            session_token: "no-session".to_owned(),
        },
    )
    .await;
    assert!(refused.is_err());
    Ok(())
}
