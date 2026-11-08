//! The tools of agents over `/mcp`, on both databases (AI-36, AI-37, ADR-0029, decision 9): an agent
//! signs in with its key, takes work all or nothing, and is paused when it reaches a limit.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::ai_mcp_rig::{Answer, Mcp, ai_on, an_agent_with_a_key};
use super::http::Browser;
use super::issues_site::{Site, key_of, ok};

/// A team member, two agents of theirs with a key each, and an issue handed over to both.
struct Team {
    person: Browser,
    agents: Vec<(String, String)>,
    issue: String,
}

async fn team(site: &Site<'_>, ada: &mut Browser, names: &[&str]) -> Result<Team, Failure> {
    let (mut person, _) = site.person(StartingRole::TeamMember, "tess").await?;
    let issue = key_of(&site.create(ada, "Fix the login button").await?)?;
    let mut agents = Vec::new();
    for name in names {
        agents.push(an_agent_with_a_key(&mut person, name, Some(&issue)).await?);
    }
    Ok(Team {
        person,
        agents,
        issue,
    })
}

async fn call(mcp: &Mcp<'_>, key: &str, tool: &str, arguments: Value) -> Result<Answer, Failure> {
    mcp.post(
        Some(key),
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }),
    )
    .await
}

fn failed(answer: &Answer) -> bool {
    answer.json["result"]["isError"] == json!(true)
}

#[tokio::test]
async fn ai_36_two_agents_take_the_same_issue_over_mcp_and_only_one_gets_it() -> TestResult {
    each_database("ai_36_take_over_mcp", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let team = team(&site, &mut ada, &["Fixer", "Helper"]).await?;
        let mcp = Mcp::new(&app);
        // The agent is offered its tools, and a person's key is offered none of them.
        let listed = mcp
            .post(
                Some(&team.agents[0].1),
                json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            )
            .await?;
        let names: Vec<&str> = listed.json["result"]["tools"]
            .as_array()
            .ok_or("no tools")?
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        for tool in [
            "take",
            "report",
            "ask",
            "attach",
            "submit",
            "give_up",
            "changes_since",
        ] {
            assert!(names.contains(&tool), "{tool} is not offered: {names:?}");
        }
        assert!(!names.contains(&"log_time"), "{names:?}");
        let input = json!({"issue": team.issue});
        let (first, second) = tokio::join!(
            call(&mcp, &team.agents[0].1, "take", input.clone()),
            call(&mcp, &team.agents[1].1, "take", input.clone()),
        );
        let (first, second) = (first?, second?);
        assert_eq!(
            usize::from(!failed(&first)) + usize::from(!failed(&second)),
            1,
            "{} / {}",
            first.json,
            second.json
        );
        // The one that got it reports; the other one is told the work is not waiting for it.
        let (winner, loser) = if failed(&first) { (1, 0) } else { (0, 1) };
        let reported = call(
            &mcp,
            &team.agents[winner].1,
            "report",
            json!({"issue": team.issue, "minutes": 3, "cost_cents": 0}),
        )
        .await?;
        assert!(!failed(&reported), "{}", reported.json);
        let again = call(&mcp, &team.agents[loser].1, "take", input).await?;
        assert!(failed(&again));
        let _ = &team.person;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_37_an_agent_over_its_limit_is_paused_over_mcp_and_resumes() -> TestResult {
    each_database("ai_37_limit_over_mcp", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        // The most any agent can be given is two changes a minute, and a new agent starts there.
        ok(&ada
            .send(
                "PUT",
                "/api/v1/settings/agents.maximum-changes-a-minute",
                Some(json!({"value": 2})),
            )
            .await?)?;
        let mut team = team(&site, &mut ada, &["Fixer"]).await?;
        let (agent, key) = team.agents.remove(0);
        let mcp = Mcp::new(&app);
        let report =
            |minutes: u32| json!({"issue": team.issue, "minutes": minutes, "cost_cents": 0});
        // Two changes are within the limit. The third does not start, and the agent is paused.
        assert!(!failed(
            &call(&mcp, &key, "take", json!({"issue": team.issue})).await?
        ));
        assert!(!failed(&call(&mcp, &key, "report", report(1)).await?));
        let third = call(&mcp, &key, "report", report(2)).await?;
        assert!(failed(&third), "{}", third.json);
        let shown = team
            .person
            .send("GET", &format!("/api/v1/agents/{agent}"), None)
            .await?;
        assert_eq!(ok(&shown)?["state"], "paused");
        assert_eq!(ok(&shown)?["paused_for"], "changes-a-minute");
        // A paused agent is refused like one that is stopped.
        let refused = mcp
            .post(
                Some(&key),
                json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            )
            .await?;
        assert_eq!(refused.status, axum::http::StatusCode::UNAUTHORIZED);
        // Its person lets it work again.
        let version = ok(&shown)?["version"].as_i64().ok_or("no version")?;
        let resumed = team
            .person
            .send(
                "PATCH",
                &format!("/api/v1/agents/{agent}"),
                Some(json!({"action": "resume", "version": version})),
            )
            .await?;
        assert_eq!(ok(&resumed)?["state"], "active");
        assert!(!failed(&call(&mcp, &key, "report", report(3)).await?));
        Ok(())
    })
    .await
}
