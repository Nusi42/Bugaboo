//! The saves of an agent on a version that is not the current one (AI-30): the rows of the table
//! of `conflict_http` for the routes of agents.

use bb_testing::Failure;
use serde_json::json;

use super::{Browser, Save, made};
use crate::sign_in_tests::issues_site::text_in;

/// The save of the route `id` of agents: an agent is made first, and the save is on its version.
pub(super) async fn prepare(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    let name = if id == "changeAgent" {
        "Fixer"
    } else {
        "Helper"
    };
    let agent = made(ada, "/api/v1/agents", json!({ "name": name })).await?;
    let address = format!("/api/v1/agents/{}", text_in(&agent, "id")?);
    match id {
        "changeAgent" => Save::new(
            "PATCH",
            address,
            json!({"name": "Finder"}),
            &agent["version"],
        ),
        // The version is looked at first, so the person it is offered to need not be there.
        _ => Save::new(
            "POST",
            format!("{address}/hand-over"),
            json!({"to": text_in(&agent, "id")?}),
            &agent["version"],
        ),
    }
}
