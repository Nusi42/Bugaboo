//! The stale saves of agent work and standing rules: the rows of the table of `conflict_http` for
//! these routes. The version of the work or the rule is looked at before anything else about it.

use bb_testing::Failure;
use serde_json::json;

use super::{Browser, Save, made};
use crate::sign_in_tests::issues_site::{ISSUES, Site, ok, text_in};

pub(super) async fn prepare(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    let agent = made(
        ada,
        "/api/v1/agents",
        json!({"name": format!("Worker {id}")}),
    )
    .await?;
    let agent = text_in(&agent, "id")?;
    if id == "changeStandingRule" {
        let search = made(
            ada,
            "/api/v1/saved-searches",
            json!({"name": "Open work", "search_text": ""}),
        )
        .await?;
        let rule = made(
            ada,
            "/api/v1/standing-rules",
            json!({
                "agent": agent,
                "search": text_in(&search, "id")?,
                "project": site.acme,
            }),
        )
        .await?;
        return Save::new(
            "PATCH",
            format!("/api/v1/standing-rules/{}", text_in(&rule, "id")?),
            json!({"action": "stop"}),
            &rule["version"],
        );
    }
    let issue = ok(&site.create(ada, "Hand this over").await?)?.clone();
    let key = text_in(&issue, "readable_id")?;
    made(
        ada,
        "/api/v1/hand-overs",
        json!({"agent": agent, "issues": [key], "note": "Look into it"}),
    )
    .await?;
    let all = ok(&ada
        .send("GET", &format!("{ISSUES}/{key}/agent-work"), None)
        .await?)?
    .clone();
    let work = &all["work"][0];
    let address = format!("/api/v1/agent-work/{}", text_in(work, "id")?);
    match id {
        "answerAgentQuestion" => Save::new(
            "POST",
            format!("{address}/answer"),
            json!({"question": text_in(work, "id")?, "answer": "Yes"}),
            &work["version"],
        ),
        "approveAgentPlan" => Save::new(
            "POST",
            format!("{address}/approve-plan"),
            json!({}),
            &work["version"],
        ),
        _ => Save::new(
            "POST",
            format!("{address}/review"),
            json!({"answer": "accept"}),
            &work["version"],
        ),
    }
}
