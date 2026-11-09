//! The tools of agents over `/mcp`, on both databases (AI-36, AI-37, ADR-0029, decision 9): an agent
//! signs in with its key, takes work all or nothing, and is paused when it reaches a limit.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::ai_mcp_rig::{Answer, Mcp, ai_on, an_agent_with_a_key};
use super::http::Browser;
use super::issues_site::{ISSUES, Site, key_of, ok};

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

pub(super) async fn call(
    mcp: &Mcp<'_>,
    key: &str,
    tool: &str,
    arguments: Value,
) -> Result<Answer, Failure> {
    mcp.post(
        Some(key),
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }),
    )
    .await
}

pub(super) fn failed(answer: &Answer) -> bool {
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

#[tokio::test]
async fn ai_37_two_takes_of_one_agent_at_the_same_moment_pass_a_limit_of_one_issue_once()
-> TestResult {
    each_database("ai_37_parallel_takes", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        ok(&ada
            .send(
                "PUT",
                "/api/v1/settings/agents.maximum-issues-at-once",
                Some(json!({"value": 1})),
            )
            .await?)?;
        let mut team = team(&site, &mut ada, &["Fixer"]).await?;
        let (agent, key) = team.agents.remove(0);
        let second = key_of(&site.create(&mut ada, "Another issue to fix").await?)?;
        ok(&team
            .person
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": agent, "issues": [second], "note": "And this one"})),
            )
            .await?)?;
        let mcp = Mcp::new(&app);
        let (one, other) = tokio::join!(
            call(&mcp, &key, "take", json!({"issue": team.issue})),
            call(&mcp, &key, "take", json!({"issue": second})),
        );
        let (one, other) = (one?, other?);
        assert_eq!(
            usize::from(!failed(&one)) + usize::from(!failed(&other)),
            1,
            "{} / {}",
            one.json,
            other.json
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_36_a_report_before_a_take_starts_nothing_over_mcp() -> TestResult {
    each_database("ai_36_report_before_take", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let team = team(&site, &mut ada, &["Fixer", "Helper"]).await?;
        let mcp = Mcp::new(&app);
        let report = json!({"issue": team.issue, "minutes": 1, "cost_cents": 0});
        assert!(failed(
            &call(&mcp, &team.agents[0].1, "report", report.clone()).await?
        ));
        assert!(!failed(
            &call(
                &mcp,
                &team.agents[0].1,
                "take",
                json!({"issue": team.issue})
            )
            .await?
        ));
        // The other agent reports on an issue the first one works on, and is turned away.
        assert!(failed(
            &call(&mcp, &team.agents[1].1, "report", report).await?
        ));
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ai_37_twenty_requests_at_once_against_a_limit_of_five_changes_let_five_in() -> TestResult {
    each_database("ai_37_parallel_changes", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        ok(&ada
            .send(
                "PUT",
                "/api/v1/settings/agents.maximum-changes-a-minute",
                Some(json!({"value": 5})),
            )
            .await?)?;
        let mut team = team(&site, &mut ada, &["Fixer"]).await?;
        let (agent, key) = team.agents.remove(0);
        // A comment needs an attempt that runs, so the agent takes the issue first.
        let taken = call(&Mcp::new(&app), &key, "take", json!({"issue": team.issue})).await?;
        assert!(!failed(&taken), "{}", taken.json);
        let handles: Vec<_> = (0..20)
            .map(|at| {
                let (app, key, issue) = (app.clone(), key.clone(), team.issue.clone());
                tokio::spawn(async move {
                    let mcp = Mcp::new(&app);
                    call(
                        &mcp,
                        &key,
                        "add_comment",
                        json!({"issue": issue, "text": format!("Comment {at}")}),
                    )
                    .await
                })
            })
            .collect();
        let mut made = 0;
        for handle in handles {
            let answer = handle.await.map_err(|error| error.to_string())??;
            if answer.status == axum::http::StatusCode::OK && !failed(&answer) {
                made += 1;
            }
        }
        assert!(
            (1..=5).contains(&made),
            "{made} changes got in against a limit of 5"
        );
        let shown = team
            .person
            .send("GET", &format!("/api/v1/agents/{agent}"), None)
            .await?;
        assert_eq!(ok(&shown)?["state"], "paused");
        // What got in is what was stored: one comment for each change that was let in.
        let stored = team
            .person
            .send(
                "GET",
                &format!("{ISSUES}/{}/history?tab=comments&limit=100", team.issue),
                None,
            )
            .await?
            .body["items"]
            .as_array()
            .map_or(0, Vec::len);
        assert_eq!(stored, made, "comments stored against changes let in");
        Ok(())
    })
    .await
}

/// An agent that may change the title of one issue it was handed, and the issue.
async fn titler(
    site: &Site<'_>,
    ada: &mut Browser,
    person: &mut Browser,
) -> Result<(String, String, String), Failure> {
    let issue = key_of(&site.create(ada, "Fix the login button").await?)?;
    let (agent, key) = an_agent_with_a_key(person, "Fixer", None).await?;
    ok(&person
        .send(
            "POST",
            "/api/v1/hand-overs",
            Some(
                json!({"agent": agent, "issues": [issue], "note": "Fix the title",
                        "powers": ["change-fields"], "fields": ["title"]}),
            ),
        )
        .await?)?;
    Ok((agent, issue, key))
}

/// The agent changes the title of its issue over MCP.
async fn retitle(
    mcp: &Mcp<'_>,
    person: &mut Browser,
    (issue, key): (&str, &str),
    title: &str,
) -> Result<Answer, Failure> {
    let shown = person
        .send("GET", &format!("{ISSUES}/{issue}"), None)
        .await?;
    let version = ok(&shown)?["issue"]["version"].clone();
    call(
        mcp,
        key,
        "update_issue",
        json!({"issue": issue, "version": version, "title": title}),
    )
    .await
}

#[tokio::test]
async fn m7_f120_a_change_over_mcp_needs_an_attempt_that_runs() -> TestResult {
    each_database("m7_f120_change_needs_attempt", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let (mut person, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (_, issue, key) = titler(&site, &mut ada, &mut person).await?;
        let mcp = Mcp::new(&app);
        let of = (issue.as_str(), key.as_str());
        // Waiting for the agent: the hand-over allows the title, but no attempt runs.
        assert!(failed(
            &retitle(&mcp, &mut person, of, "Before the take").await?
        ));
        let taken = call(&mcp, &key, "take", json!({"issue": issue})).await?;
        assert!(!failed(&taken), "{}", taken.json);
        let changed = retitle(&mcp, &mut person, of, "While it runs").await?;
        assert!(!failed(&changed), "{}", changed.json);
        let submitted = call(&mcp, &key, "submit", json!({"issue": issue})).await?;
        assert!(!failed(&submitted), "{}", submitted.json);
        // Ready for review: the attempt has ended and so has the right to change.
        assert!(failed(
            &retitle(&mcp, &mut person, of, "After the submit").await?
        ));
        let shown = person
            .send("GET", &format!("{ISSUES}/{issue}"), None)
            .await?;
        assert_eq!(ok(&shown)?["issue"]["title"], "While it runs");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_37_a_stale_change_over_the_api_gives_its_count_back() -> TestResult {
    each_database("ai_37_stale_gives_back", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        ok(&ada
            .send(
                "PUT",
                "/api/v1/settings/agents.maximum-changes-a-minute",
                Some(json!({"value": 3})),
            )
            .await?)?;
        let (mut person, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (agent, issue, key) = titler(&site, &mut ada, &mut person).await?;
        // The change is let through only while an attempt runs, so the agent takes the issue first.
        let taken = call(&Mcp::new(&app), &key, "take", json!({"issue": issue})).await?;
        assert!(!failed(&taken), "{}", taken.json);
        let mut script = Browser::new(&app, &super::http::called());
        // Ten changes from a version that is not the issue's: each is refused as stale, and none of
        // them counts against a limit of three.
        for _ in 0..10 {
            let stale = script
                .script(
                    "PATCH",
                    &format!("{ISSUES}/{issue}"),
                    Some(json!({"version": 999, "title": "Too late"})),
                    &key,
                )
                .await?;
            assert_eq!(
                stale.status,
                axum::http::StatusCode::CONFLICT,
                "{}",
                stale.body
            );
        }
        let shown = person
            .send("GET", &format!("/api/v1/agents/{agent}"), None)
            .await?;
        assert_eq!(ok(&shown)?["state"], "active");
        Ok(())
    })
    .await
}
