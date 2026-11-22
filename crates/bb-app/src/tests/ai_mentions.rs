//! A mention names a person by their handle, so with an outside provider and *Send names of people*
//! off it goes as *someone*, in every text a person wrote (M7-Q5, ADR-0013).

use std::collections::BTreeMap;

use serde_json::json;

use super::ai_tools::{call, issue_with, without_nonces};
use super::scenarios::{a_project, token};
use super::search_fields::{bob_with_a_role, every_field};
use super::world::{Outcome, World};
use crate::ai::context::PeopleMode;
use crate::ai::tools::call_tool;
use crate::comments::{AddComment, add_comment};

#[tokio::test]
async fn ai_04_a_mention_reaches_an_outside_provider_as_someone() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let (bob, _) = bob_with_a_role(&world, &every_field()).await?;
    let description = "@ada.admin please look; the address x@ada.example stays";
    let key = issue_with(&world, "WEB", "A title", description, BTreeMap::new()).await?;
    add_comment(
        &world.ctx(),
        AddComment {
            session_token: token(&world),
            issue: key.clone(),
            text: "Ask @bob.builder first.".to_owned(),
        },
    )
    .await?;
    let read = async |people: PeopleMode| {
        let mut query = call(&bob, "get_issue", json!({"issue": key}));
        query.people = people;
        call_tool(&world.ctx(), query)
            .await
            .map(|answer| without_nonces(&answer.into_value().text))
    };
    let by_role = read(PeopleMode::RoleOnly).await?;
    for handle in ["ada.admin", "bob.builder"] {
        assert!(!by_role.contains(handle), "{handle}: {by_role}");
    }
    assert!(by_role.contains("someone please look"), "{by_role}");
    assert!(by_role.contains("Ask someone first."), "{by_role}");
    // An e-mail address is no mention: what the person typed as plain words cannot be told apart.
    assert!(by_role.contains("x@ada.example"), "{by_role}");
    // As the person sees them (a local model), the mention stays.
    let as_seen = read(PeopleMode::AsSeen).await?;
    assert!(as_seen.contains("@ada.admin please look"), "{as_seen}");
    Ok(())
}
