//! What an AI app reads back from a tool over `/mcp`, on both databases (AI-21, ADR-0042): a new
//! issue is named so that it can be found again before the search knows it, and a call that saved
//! says what changed and the version it saved, where its preview says what would change.

use bb_app::access::StartingRole;
use bb_core::ErrorCode;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::ai_mcp_rig::{Answer, Mcp, ai_on, an_agent_with_a_key, sign_in_app};
use super::issues_site::{Site, ok};

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
            for limit in [json!(0), json!(-1), json!(101), json!(2.5)] {
                let mut asked = input.clone();
                asked["limit"] = limit.clone();
                let refused = mcp.call_tool(&signed, tool, asked).await?;
                assert_eq!(
                    refused.json["result"]["isError"],
                    json!(true),
                    "{tool} {limit}"
                );
                assert_eq!(text_of(&refused), range, "{tool} {limit}");
            }
        }
        // The tool of agents that takes a limit is judged the same way.
        let (_, agent) = an_agent_with_a_key(&mut ada, "Fixer", None).await?;
        for limit in [json!(0), json!(101), json!(2.5)] {
            let refused = super::ai_mcp_agents::call(
                &mcp,
                &agent,
                "changes_since",
                json!({ "limit": limit }),
            )
            .await?;
            assert_eq!(refused.json["result"]["isError"], json!(true), "{limit}");
            assert_eq!(text_of(&refused), range, "changes_since {limit}");
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

/// Words in a project the reader may not see, in a comment, and in a project kept from AI: BETA
/// has the issue "Wheels" with a comment about brakes, KEPT the issue "Brakes kept from AI".
async fn words_out_of_sight(
    site: &Site<'_>,
    ada: &mut super::http::Browser,
) -> Result<(), bb_testing::Failure> {
    for (short_name, title, comment) in [
        ("BETA", "Wheels", "The brakes are hidden here"),
        ("KEPT", "Brakes kept from AI", "brakes"),
    ] {
        let project = ok(&ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": short_name, "name": short_name})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let made = ok(&ada
            .send(
                "POST",
                &format!("/api/v1/projects/{project}/issues"),
                Some(json!({"issue_type": site.kind, "title": title})),
            )
            .await?)?["readable_id"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        ok(&ada
            .send(
                "POST",
                &format!("/api/v1/issues/{made}/comments"),
                Some(json!({ "text": comment })),
            )
            .await?)?;
    }
    ok(&ada
        .send(
            "PUT",
            "/api/v1/never-sent-to-ai/projects/KEPT",
            Some(json!({"marked": true, "version": 0})),
        )
        .await?)?;
    Ok(())
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
        words_out_of_sight(&site, &mut ada).await?;
        // The text index follows a new issue in the next round of the jobs (ADR-0016).
        let ctx = rig.services.ctx(super::rig::Rig::info());
        while ctx
            .run_due_jobs()
            .await
            .map_err(|error| format!("{error:?}"))?
            > 0
        {}
        let signed = sign_in_app(&mcp, &mut ada, "Claude Code", "see-only", "see-only").await?;
        let (mut tom, _) = site.person_in_acme(StartingRole::TeamMember, "tom").await?;
        let toms = sign_in_app(&mcp, &mut tom, "Claude Code", "see-only", "see-only").await?;
        let found = mcp
            .call_tool(&toms, "search_issues", json!({"query": "brakes"}))
            .await?;
        let found = text_of(&found);
        assert!(found.contains("Brakes squeak"), "{found}");
        assert!(
            !found.contains("Wheels"),
            "a comment in a project Tom cannot see: {found}"
        );
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
        assert!(words.contains("Wheels"), "Ada reads the comment: {words}");
        assert!(!words.contains("Brakes kept"), "kept from AI: {words}");
        // A mistyped search is no words: it is told its mistake.
        for typo in ["title includes x", "status open"] {
            let refused = mcp
                .call_tool(&signed, "search_issues", json!({ "query": typo }))
                .await?;
            assert_eq!(refused.json["result"]["isError"], json!(true), "{typo}");
            let said = text_of(&refused);
            assert!(!said.starts_with("Write"), "{typo}: {said}");
            assert!(said.contains("title contains \"brakes\""), "{typo}: {said}");
        }
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

/// The project KEPT with a workflow of its own whose status *Blockade* no other project has.
async fn a_status_only_in_kept(ada: &mut super::http::Browser) -> Result<(), bb_testing::Failure> {
    let kept = ok(&ada
        .send(
            "POST",
            "/api/v1/projects",
            Some(json!({"short_name": "KEPT", "name": "Kept"})),
        )
        .await?)?["id"]
        .as_str()
        .ok_or("no project ID")?
        .to_owned();
    let editor = ok(&ada
        .send(
            "POST",
            "/api/v1/workflows",
            Some(json!({ "project": kept })),
        )
        .await?)?
    .clone();
    let workflow = editor["workflow"]["id"]
        .as_str()
        .ok_or("no workflow ID")?
        .to_owned();
    let lock = super::workflows_http::lock_workflow(ada, &workflow).await?;
    ok(&ada
        .send(
            "PUT",
            &format!("/api/v1/workflows/{workflow}"),
            Some(json!({
                "version": editor["workflow"]["version"],
                "lock_token": lock,
                "name": "With a blockade",
                "statuses": [
                    {"key": "open", "name": "Open", "status_group": "not-started"},
                    {"key": "blockade", "name": "Blockade", "status_group": "in-progress"},
                    {"key": "done", "name": "Done", "status_group": "done"},
                ],
                "changes": [
                    {"from": "open", "to": "blockade", "button": "Block"},
                    {"from": "blockade", "to": "done", "button": "Finish"},
                ],
                "moves": [],
            })),
        )
        .await?)?;
    Ok(())
}

/// What only the project KEPT knows besides its status: an issue type, a label in a group made
/// there, and a tag used only on an issue there.
async fn names_only_in_kept(
    ada: &mut super::http::Browser,
    kind: &str,
) -> Result<(), bb_testing::Failure> {
    ok(&ada
        .send(
            "POST",
            "/api/v1/issue-types",
            Some(json!({"name": "Blockwork", "icon": "star", "project": "KEPT"})),
        )
        .await?)?;
    let group = ok(&ada
        .send(
            "POST",
            "/api/v1/label-groups",
            Some(json!({"name": "Weather", "pick_one": false, "project": "KEPT"})),
        )
        .await?)?["id"]
        .clone();
    ok(&ada
        .send(
            "POST",
            "/api/v1/labels",
            Some(json!({"name": "zephyr", "label_group": group})),
        )
        .await?)?;
    let issue = ok(&ada
        .send(
            "POST",
            "/api/v1/projects/KEPT/issues",
            Some(json!({"issue_type": kind, "title": "Tagged"})),
        )
        .await?)?["readable_id"]
        .as_str()
        .ok_or("no key")?
        .to_owned();
    ok(&ada
        .send(
            "POST",
            &format!("/api/v1/issues/{issue}/tags"),
            Some(json!({"name": "zephyrtag"})),
        )
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn ai_03_a_search_never_tells_a_name_known_only_from_a_project_kept_from_ai() -> TestResult {
    each_database("ai_03_suggestion_kept_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        a_status_only_in_kept(&mut ada).await?;
        names_only_in_kept(&mut ada, &site.kind).await?;
        let signed = sign_in_app(&mcp, &mut ada, "Claude Code", "see-only", "see-only").await?;
        let search = async |query: &str| -> Result<(bool, String), bb_testing::Failure> {
            let answer = mcp
                .call_tool(&signed, "search_issues", json!({ "query": query }))
                .await?;
            Ok((
                answer.json["result"]["isError"] == json!(true),
                text_of(&answer),
            ))
        };
        let misspelt = [
            ("status is Blockad", "Blockade"),
            ("type is Blockwor", "Blockwork"),
        ];
        let named = ["labels is zephyr", "tags is zephyrtag"];
        // While the project may be read by AI, its names are fair suggestions and fair names.
        for (query, name) in misspelt {
            let (refused, said) = search(query).await?;
            assert!(refused && said.contains(name), "{query}: {said}");
        }
        for query in named {
            let (refused, said) = search(query).await?;
            assert!(!refused, "{query}: {said}");
        }
        ok(&ada
            .send(
                "PUT",
                "/api/v1/never-sent-to-ai/projects/KEPT",
                Some(json!({"marked": true, "version": 0})),
            )
            .await?)?;
        for (query, name) in misspelt {
            let (refused, said) = search(query).await?;
            assert!(refused && !said.contains(name), "{query}: {said}");
        }
        // A name only the kept project knows is refused like a name nobody has.
        for (query, unknown) in [
            ("labels is zephyr", "labels is nonesuch"),
            ("tags is zephyrtag", "tags is nonesuchtag"),
        ] {
            let (refused, said) = search(query).await?;
            let (_, nobody) = search(unknown).await?;
            assert!(refused, "{query}: {said}");
            let word = |text: &str| text.rsplit(' ').next().unwrap_or_default().to_owned();
            assert_eq!(
                said.replace(&word(query), &word(unknown)),
                nobody,
                "{query}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_04_an_app_of_an_administrator_is_offered_the_tools_before_any_project_exists()
-> TestResult {
    each_database("ai_04_tools_no_project_http", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let _org = super::policy_rig::Org::start(&rig).await?;
        let mut ada = super::http::Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        let names =
            async |signed: &super::ai_mcp_rig::Signed| -> Result<String, bb_testing::Failure> {
                let listed = mcp
                    .post(
                        Some(&signed.access),
                        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
                    )
                    .await?;
                Ok(listed.json["result"]["tools"].to_string())
            };
        // A role for the whole organization allows the tools wherever a project will be.
        let looking = sign_in_app(
            &mcp,
            &mut ada,
            "Claude Desktop",
            "see-and-change",
            "see-only",
        )
        .await?;
        let seen = names(&looking).await?;
        assert!(seen.contains("search_issues"), "{seen}");
        assert!(!seen.contains("create_issue"), "{seen}");
        let changing = sign_in_app(
            &mcp,
            &mut ada,
            "Claude Code",
            "see-and-change",
            "see-and-change",
        )
        .await?;
        let all = names(&changing).await?;
        assert!(all.contains("create_issue"), "{all}");
        Ok(())
    })
    .await
}
