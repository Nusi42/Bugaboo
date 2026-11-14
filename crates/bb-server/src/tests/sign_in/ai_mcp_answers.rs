//! What an AI app reads back from a tool over `/mcp`, on both databases (AI-21, ADR-0042): a new
//! issue is named so that it can be found again before the search knows it, and a call that saved
//! says what changed and the version it saved, where its preview says what would change.

use bb_app::access::StartingRole;
use bb_core::ErrorCode;
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

#[tokio::test]
async fn ai_21_a_saved_change_over_mcp_says_changed_and_its_version_where_the_preview_would()
-> TestResult {
    each_database("ai_21_saved_answer_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        let made = site.create(&mut ada, "Old title").await?;
        let key = made.body["readable_id"].as_str().ok_or("no key")?.to_owned();
        let version = super::issues_site::version_of(&mut ada, &key).await?;
        let signed =
            sign_in_app(&mcp, &mut ada, "Claude Code", "see-and-change", "see-and-change").await?;
        let input = |preview: bool| {
            json!({"issue": key, "version": version, "title": "New title", "preview": preview})
        };
        let preview = text_of(&mcp.call_tool(&signed, "update_issue", input(true)).await?);
        let saved = text_of(&mcp.call_tool(&signed, "update_issue", input(false)).await?);
        assert_ne!(preview, saved);
        assert!(preview.contains("\nwould_change:"), "{preview}");
        assert!(!preview.contains("Saved"), "{preview}");
        assert!(saved.contains("\nchanged:"), "{saved}");
        assert!(!saved.contains("would_change"), "{saved}");
        let now = super::issues_site::version_of(&mut ada, &key).await?;
        assert_eq!(
            saved.lines().last(),
            Some(format!("Saved: {key} is now at version {now}.").as_str()),
            "{saved}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_21_a_limit_of_zero_or_less_is_refused_over_mcp_with_the_range() -> TestResult {
    each_database("ai_21_limit_range_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        for title in ["Brakes squeak", "Lights flicker"] {
            site.create(&mut ada, title).await?;
        }
        let signed = sign_in_app(&mcp, &mut ada, "Claude Code", "see-only", "see-only").await?;
        let range = ErrorCode::LimitOutOfRange.text().render();
        for (tool, input) in [
            ("list_projects", json!({})),
            ("get_my_work", json!({})),
            ("search_issues", json!({"query": ""})),
        ] {
            for limit in [0, -1, 101] {
                let mut asked = input.clone();
                asked["limit"] = json!(limit);
                let refused = mcp.call_tool(&signed, tool, asked).await?;
                assert_eq!(
                    refused.json["result"]["isError"],
                    json!(true),
                    "{tool} {limit}"
                );
                assert_eq!(text_of(&refused), range, "{tool} {limit}");
            }
        }
        // A limit of one answers one issue.
        let one = mcp
            .call_tool(&signed, "search_issues", json!({"query": "", "limit": 1}))
            .await?;
        assert_eq!(one.json["result"]["isError"], json!(false), "{}", one.json);
        assert_eq!(
            text_of(&one).matches("\nissue:").count(),
            1,
            "{}",
            text_of(&one)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_04_tools_list_offers_only_the_tools_a_person_may_use_somewhere() -> TestResult {
    each_database("ai_04_tools_by_role_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        for (role, name, offered) in [
            (StartingRole::Hr, "hana", 0),
            (StartingRole::Viewer, "vic", 4),
            (StartingRole::Guest, "gus", 8),
            (StartingRole::TeamMember, "tom", 11),
        ] {
            let (mut person, _) = site.person(role, name).await?;
            let signed = sign_in_app(
                &mcp,
                &mut person,
                "Claude Code",
                "see-and-change",
                "see-and-change",
            )
            .await?;
            let listed = mcp
                .post(
                    Some(&signed.access),
                    json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
                )
                .await?;
            let tools = listed.json["result"]["tools"]
                .as_array()
                .ok_or("no tools")?
                .clone();
            assert_eq!(tools.len(), offered, "{role:?}: {}", listed.json);
            // A tool left out is still refused when it is called by its name.
            if offered == 0 {
                let refused = mcp
                    .call_tool(&signed, "create_issue", json!({"project": "ACME"}))
                    .await?;
                assert_eq!(
                    refused.json["result"]["isError"],
                    json!(true),
                    "{}",
                    refused.json
                );
            }
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_21_search_issues_reads_plain_words_as_a_search_of_the_texts() -> TestResult {
    each_database("ai_21_plain_words_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        for title in ["Brakes squeak", "Lights flicker"] {
            site.create(&mut ada, title).await?;
        }
        // The text index follows a new issue in the next round of the jobs (ADR-0016).
        let ctx = rig.services.ctx(super::rig::Rig::info());
        while ctx
            .run_due_jobs()
            .await
            .map_err(|error| format!("{error:?}"))?
            > 0
        {}
        let signed = sign_in_app(&mcp, &mut ada, "Claude Code", "see-only", "see-only").await?;
        let search = async |query: &str| -> Result<String, bb_testing::Failure> {
            let found = mcp
                .call_tool(&signed, "search_issues", json!({ "query": query }))
                .await?;
            assert_eq!(
                found.json["result"]["isError"],
                json!(false),
                "{query}: {}",
                found.json
            );
            Ok(text_of(&found))
        };
        // Plain words find what the search language finds for them.
        let words = search("brakes").await?;
        let language = search("title contains \"brakes\"").await?;
        assert!(words.contains("Brakes squeak"), "{words}");
        assert!(!words.contains("Lights flicker"), "{words}");
        assert!(
            words.contains("contains \"brakes\""),
            "the answer says how it was read: {words}"
        );
        assert!(language.contains("Brakes squeak"), "{language}");
        // A search that cannot be read says how to write one.
        let refused = mcp
            .call_tool(&signed, "search_issues", json!({"query": "colour is red"}))
            .await?;
        assert_eq!(
            refused.json["result"]["isError"],
            json!(true),
            "{}",
            refused.json
        );
        assert!(
            text_of(&refused).contains("title contains \"brakes\""),
            "{}",
            refused.json
        );
        // The tool says so in its description.
        let listed = mcp
            .post(
                Some(&signed.access),
                json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            )
            .await?;
        let described = listed.json["result"]["tools"]
            .as_array()
            .and_then(|tools| tools.iter().find(|tool| tool["name"] == "search_issues"))
            .map(|tool| tool["description"].to_string())
            .unwrap_or_default();
        assert!(described.contains("title contains"), "{described}");
        Ok(())
    })
    .await
}
