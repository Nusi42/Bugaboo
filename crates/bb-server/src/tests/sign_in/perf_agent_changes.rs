//! The look of an agent at what changed (*changes since*, AI-36) starts as many transactions for many
//! matches as for few: each transaction is a round trip to start and one to end, and a request that
//! reads one issue or one match at a time was over the budget of a page on a million issues (M7-F127).
//! Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::ai_mcp_agents::{call, failed};
use super::ai_mcp_rig::{Mcp, ai_on};
use super::issues_site::{Site, key_of};
use super::rules_hand_over_http::{agent_with_limit, look_again, rule_over};

/// The most transactions one look may start, however many matches it lists (17 measured: the key of
/// the agent, its rights and the reads of the catalogs come first, and the rest is one read each of
/// the matches, the issues, the people, the projects and the rights).
const AT_MOST: u64 = 17;

/// The text of an answer of the MCP server.
fn text_of(answer: &super::ai_mcp_rig::Answer) -> &str {
    answer.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

#[tokio::test]
async fn m7_f127_a_look_at_the_changes_starts_as_many_transactions_for_many_matches_as_for_few()
-> TestResult {
    each_database("m7_f127_changes_since", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (agent, key) = agent_with_limit(&mut tess, 5).await?;
        rule_over(&site, &mut tess, &agent, ("", "Everything"), false).await?;
        let mcp = Mcp::new(&app);
        let mut transactions = Vec::new();
        let mut statements = Vec::new();
        let mut matches = Vec::new();
        let mut issues = 0_usize;
        for more in [2_usize, 8] {
            while issues < more {
                issues += 1;
                key_of(&site.create(&mut ada, &format!("Issue {issues}")).await?)?;
            }
            look_again(&rig).await?;
            // The place of now in the log with the moment of the matches put back to the oldest:
            // nothing in the log is after it, so every match is an issue the request has not looked
            // at yet, the case that costs the most reads.
            let now = call(&mcp, &key, "changes_since", json!({})).await?;
            let place = text_of(&now)
                .lines()
                .find_map(|line| line.trim().strip_prefix("mark: "))
                .and_then(|mark| mark.trim_matches('"').split('.').next())
                .ok_or("no mark in the first answer")?
                .to_owned();
            let ask = || {
                call(
                    &mcp,
                    &key,
                    "changes_since",
                    json!({"since": format!("{place}.0")}),
                )
            };
            // The first look warms what is kept across requests and is not counted.
            assert!(!failed(&ask().await?));
            let (before, before_statements) = (
                rig.database.transactions_run(),
                rig.database.statements_run(),
            );
            let looked = ask().await?;
            assert!(!failed(&looked), "{}", looked.json);
            transactions.push(rig.database.transactions_run() - before);
            statements.push(rig.database.statements_run() - before_statements);
            matches.push(text_of(&looked).matches("rule:").count());
        }
        assert!(matches[1] > matches[0], "{matches:?} matches");
        assert!(transactions[0] > 0, "the counter sees nothing");
        assert_eq!(
            transactions[0], transactions[1],
            "{matches:?} matches start {transactions:?} transactions"
        );
        assert_eq!(
            statements[0], statements[1],
            "{matches:?} matches run {statements:?} statements"
        );
        assert!(
            transactions[1] <= AT_MOST,
            "{} transactions",
            transactions[1]
        );
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn m7_f127_a_key_that_was_taken_away_after_a_message_is_refused_at_the_next() -> TestResult {
    each_database("m7_f127_key_taken_away", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let made = tess
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let agent = made.body["id"].as_str().ok_or("no agent ID")?.to_owned();
        let made = tess
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/keys"),
                Some(json!({"password": super::rig::PASSWORD, "name": "Key"})),
            )
            .await?;
        let key = made.body["secret"].as_str().ok_or("no secret")?.to_owned();
        let id = made.body["key"]["id"]
            .as_str()
            .ok_or("no key ID")?
            .to_owned();
        let mcp = Mcp::new(&app);
        // A message that runs no tool leaves what its entry looked up, and no later request takes it.
        let ping = mcp
            .post(
                Some(&key),
                json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
            )
            .await?;
        assert!(ping.status.is_success(), "{}", ping.json);
        let gone = tess
            .send("DELETE", &format!("/api/v1/agents/{agent}/keys/{id}"), None)
            .await?;
        assert!(gone.status.is_success(), "{}", gone.body);
        let mut script = super::http::Browser::new(&app, &super::http::called());
        let rest = script
            .script("GET", &format!("/api/v1/agents/{agent}"), None, &key)
            .await?;
        assert!(!rest.status.is_success(), "{}", rest.body);
        let after = call(&mcp, &key, "changes_since", json!({})).await?;
        assert!(!after.status.is_success(), "{}", after.json);
        Ok::<(), Failure>(())
    })
    .await
}
