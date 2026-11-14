//! What a tool takes from an AI app and what it answers for it (AI-21): a limit is a number in the
//! range of a page, or the call is told the range.

use bb_core::ErrorCode;
use serde_json::json;

use super::ai_tools::{call, issue_with};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::ai::tools::call_tool;
use crate::auth::AppError;

#[tokio::test]
async fn ai_21_a_limit_outside_one_to_a_hundred_is_refused_with_the_range() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    issue_with(
        &world,
        "WEB",
        "Brakes",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    issue_with(
        &world,
        "WEB",
        "Lights",
        "d",
        std::collections::BTreeMap::new(),
    )
    .await?;
    let who = token(&world);
    let tools = [
        ("list_projects", json!({})),
        ("get_my_work", json!({})),
        ("search_issues", json!({"query": "title contains \"a\""})),
    ];
    for (tool, input) in tools {
        for limit in [0, -1, 101] {
            let mut asked = input.clone();
            asked["limit"] = json!(limit);
            let refused = call_tool(&world.ctx(), call(&who, tool, asked)).await;
            assert!(
                matches!(refused, Err(AppError::Refused(ErrorCode::LimitOutOfRange))),
                "{tool} {limit}: {refused:?}"
            );
        }
        for limit in [1, 100] {
            let mut asked = input.clone();
            asked["limit"] = json!(limit);
            call_tool(&world.ctx(), call(&who, tool, asked)).await?;
        }
    }
    // The refusal says the range.
    let said = ErrorCode::LimitOutOfRange.text().render();
    assert!(said.contains('1') && said.contains("100"), "{said}");
    Ok(())
}
