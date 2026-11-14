//! What an AI app reads back from a tool over `/mcp`, on both databases (AI-21, ADR-0042): a new
//! issue is named so that it can be found again before the search knows it.

use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::ai_mcp_rig::{Answer, Mcp, ai_on, sign_in_app};
use super::issues_site::Site;

/// The text a tool answered with.
fn text_of(answer: &Answer) -> String {
    answer.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

/// The value of a line `name: value` of an answer, at any depth.
fn line_value<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(name)?.strip_prefix(": "))
}

#[tokio::test]
async fn ai_21_an_app_that_creates_an_issue_is_told_its_id_its_readable_id_and_its_version()
-> TestResult {
    each_database("ai_21_created_names_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        let signed = sign_in_app(
            &mcp,
            &mut ada,
            "Claude Code",
            "see-and-change",
            "see-and-change",
        )
        .await?;
        let made = mcp
            .call_tool(
                &signed,
                "create_issue",
                json!({"project": "ACME", "issue_type": site.kind, "title": "Brakes squeak"}),
            )
            .await?;
        assert_eq!(
            made.json["result"]["isError"],
            json!(false),
            "{}",
            made.json
        );
        let text = text_of(&made);
        let key = line_value(&text, "key").ok_or(text.clone())?;
        let id = line_value(&text, "id").ok_or(text.clone())?;
        assert!(key.starts_with("ACME-"), "{text}");
        assert_eq!(line_value(&text, "version"), Some("1"), "{text}");
        // Both names reach the new issue at once, before the search has caught up with it.
        for name in [key, id] {
            let read = mcp
                .call_tool(&signed, "get_issue", json!({"issue": name}))
                .await?;
            let read = text_of(&read);
            assert!(read.contains("Brakes squeak"), "{name}: {read}");
            assert_eq!(line_value(&read, "key"), Some(key), "{read}");
        }
        Ok(())
    })
    .await
}
