//! The statements behind *My agents* on both databases (AI-30, AI-37, AI-32): what waits for a person
//! is read first, work accepted by itself stays with the person it was given for, the rules of all
//! agents come in one answer, an offer is cut at the most agents, can be taken back, and a stopped
//! agent is offered to nobody.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::agent_work::{
    AskQuestion, AttachResult, EndAttempt, ReportAttempt, ask_question, attach_result, end_attempt,
    report_attempt,
};
use bb_core::agent::MOST_AGENTS;
use bb_core::agent_records::ResultKind;
use bb_core::agent_work::CheckResult;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, ok};
use super::rig::{PASSWORD, Rig};

fn wrong(error: impl std::fmt::Debug) -> Failure {
    format!("{error:?}").into()
}

async fn work_of(who: &mut Browser) -> Result<Value, Failure> {
    let answer = who.send("GET", "/api/v1/me/agents/work", None).await?;
    Ok(ok(&answer)?.clone())
}

async fn new_agent(who: &mut Browser, name: &str) -> Result<(String, Value), Failure> {
    let made = who
        .send("POST", "/api/v1/agents", Some(json!({"name": name})))
        .await?;
    let made = ok(&made)?.clone();
    let id = made["id"].as_str().ok_or("no agent ID")?.to_owned();
    Ok((id, made["version"].clone()))
}

/// The agent asks a question on the first issue, and finishes the third with a passed check, which the
/// project accepts by itself.
async fn agent_works(rig: &Rig, key: &str, first: &str, third: &str) -> Result<(), Failure> {
    let ctx = rig.services.ctx(Rig::info());
    for issue in [first, third] {
        report_attempt(
            &ctx,
            ReportAttempt {
                session_token: key.to_owned(),
                issue: (*issue).to_owned(),
                plan: None,
                minutes: 1,
                cost: 0,
            },
        )
        .await
        .map_err(wrong)?;
    }
    ask_question(
        &ctx,
        AskQuestion {
            session_token: key.to_owned(),
            issue: first.to_owned(),
            text: "Which browser?".to_owned(),
        },
    )
    .await
    .map_err(wrong)?;
    attach_result(
        &ctx,
        AttachResult {
            session_token: key.to_owned(),
            issue: third.to_owned(),
            kind: ResultKind::Check,
            label: "Tests".to_owned(),
            link: None,
            outcome: Some(CheckResult::Passed),
        },
    )
    .await
    .map_err(wrong)?;
    end_attempt(
        &ctx,
        EndAttempt {
            session_token: key.to_owned(),
            issue: third.to_owned(),
            gave_up: None,
        },
    )
    .await
    .map_err(wrong)?;
    Ok(())
}

#[tokio::test]
async fn ai_37_waiting_work_comes_first_and_accepted_work_stays_with_its_person() -> TestResult {
    each_database("ai_37_my_agents_statements", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut olga, olga_id) = site.person(StartingRole::TeamMember, "olga").await?;
        let allowed = ada
            .send(
                "PUT",
                &format!("/api/v1/projects/{}/settings/agents.accept-by-itself", site.acme),
                Some(json!({"value": true})),
            )
            .await?;
        ok(&allowed)?;
        let (agent, version) = new_agent(&mut tess, "Fixer").await?;
        let made = tess
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/keys"),
                Some(json!({"password": PASSWORD, "name": "Key of Fixer"})),
            )
            .await?;
        let key = ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned();
        // The first issue gets a question; the second only waits for the agent; the third is accepted by itself.
        let first = key_of(&site.create(&mut ada, "First").await?)?;
        let second = key_of(&site.create(&mut ada, "Second").await?)?;
        let third = key_of(&site.create(&mut ada, "Third").await?)?;
        let handed = tess
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": agent, "issues": [first, second, third], "accept_by_itself": true})),
            )
            .await?;
        ok(&handed)?;
        agent_works(&rig, &key, &first, &third).await?;
        // What waits for the person comes first, though the second piece is newer than the first.
        let mine = work_of(&mut tess).await?;
        let pieces = mine["pieces"].as_array().ok_or("no pieces")?;
        assert_eq!(pieces.len(), 3, "{mine}");
        assert_eq!(pieces[0]["work"]["state"], json!("waiting-for-you"), "{mine}");
        assert_eq!(pieces[0]["waits_for"], json!("question"));
        // Work accepted by itself is listed, and waits for nobody.
        let accepted = pieces
            .iter()
            .find(|piece| piece["work"]["accepted_by_itself"] == json!(true))
            .ok_or("no accepted piece")?;
        assert!(accepted["waits_for"].is_null(), "{mine}");
        // The agent goes to Olga. Tess still sees what was accepted for her; Olga does not inherit it.
        let handed = tess
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/hand-over"),
                Some(json!({"to": olga_id.to_string(), "version": version})),
            )
            .await?;
        assert_eq!(handed.status, StatusCode::NO_CONTENT, "{}", handed.body);
        let taken = olga
            .send("POST", &format!("/api/v1/agents/{agent}/accept"), None)
            .await?;
        assert_eq!(taken.status, StatusCode::NO_CONTENT, "{}", taken.body);
        let tess_after = work_of(&mut tess).await?;
        let kept: Vec<&Value> = tess_after["pieces"]
            .as_array()
            .ok_or("no pieces")?
            .iter()
            .filter(|piece| piece["work"]["accepted_by_itself"] == json!(true))
            .collect();
        assert_eq!(kept.len(), 1, "{tess_after}");
        let olga_after = work_of(&mut olga).await?;
        assert!(
            olga_after["pieces"]
                .as_array()
                .ok_or("no pieces")?
                .iter()
                .all(|piece| piece["work"]["accepted_by_itself"] != json!(true)),
            "{olga_after}"
        );
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_37_the_rules_of_all_my_agents_come_in_one_answer() -> TestResult {
    each_database("ai_37_my_agents_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut olga, _) = site.person(StartingRole::TeamMember, "olga").await?;
        let search = tess
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({"name": "New bugs", "search_text": ""})),
            )
            .await?;
        let search = ok(&search)?["id"].as_str().ok_or("no search")?.to_owned();
        let mut rules = Vec::new();
        let mut agents = Vec::new();
        for name in ["Fixer", "Sorter"] {
            let (agent, _) = new_agent(&mut tess, name).await?;
            agents.push(agent.clone());
            let made = tess
                .send(
                    "POST",
                    "/api/v1/standing-rules",
                    Some(json!({"agent": agent, "search": search, "project": site.acme})),
                )
                .await?;
            rules.push(ok(&made)?["id"].as_str().ok_or("no rule")?.to_owned());
        }
        let mine = work_of(&mut tess).await?;
        let found: Vec<&str> = mine["rules"]
            .as_array()
            .ok_or("no rules")?
            .iter()
            .filter_map(|rule| rule["id"].as_str())
            .collect();
        assert_eq!(found.len(), 2, "{mine}");
        assert!(rules.iter().all(|rule| found.contains(&rule.as_str())));
        assert_eq!(mine["rules_truncated"], json!(false));
        // Nobody else sees them.
        let theirs = work_of(&mut olga).await?;
        assert_eq!(
            theirs["rules"].as_array().map(Vec::len),
            Some(0),
            "{theirs}"
        );
        // The agent route reads the rules of one agent: its person and an administrator may, nobody else.
        let path = format!("/api/v1/agents/{}/standing-rules", agents[0]);
        for reader in [&mut tess, &mut ada] {
            let read = reader.send("GET", &path, None).await?;
            assert_eq!(ok(&read)?["items"].as_array().map(Vec::len), Some(1));
        }
        let stranger = olga.send("GET", &path, None).await?;
        assert_eq!(stranger.code(), "resource.not-found", "{}", stranger.body);
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_30_an_offer_is_cut_taken_back_and_ended_by_a_stop() -> TestResult {
    each_database("ai_30_my_agents_offers", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut ruth, _) = site.person(StartingRole::TeamMember, "ruth").await?;
        let (mut olga, olga_id) = site.person(StartingRole::TeamMember, "olga").await?;
        let (mut stranger, _) = site.person(StartingRole::TeamMember, "stan").await?;
        // More agents are offered to Olga than a list holds: two people hand over one more than the most.
        let mut first = None;
        for (number, owner) in std::iter::repeat_n([0_u8, 1], MOST_AGENTS / 2 + 1)
            .flatten()
            .take(MOST_AGENTS + 1)
            .enumerate()
        {
            let who = if owner == 0 { &mut tess } else { &mut ruth };
            let (agent, version) = new_agent(who, &format!("Agent {number}")).await?;
            let handed = who
                .send(
                    "POST",
                    &format!("/api/v1/agents/{agent}/hand-over"),
                    Some(json!({"to": olga_id.to_string(), "version": version})),
                )
                .await?;
            assert_eq!(handed.status, StatusCode::NO_CONTENT, "{}", handed.body);
            if first.is_none() {
                first = Some((agent, owner));
            }
        }
        let (agent, owner) = first.ok_or("no agent")?;
        let list = work_of(&mut olga).await?;
        assert_eq!(
            list["offered"].as_array().map(Vec::len),
            Some(MOST_AGENTS),
            "{list}"
        );
        assert_eq!(list["offered_truncated"], json!(true));
        let path = format!("/api/v1/agents/{agent}/hand-over");
        // Somebody the agent is not offered to, and who is not its person, finds nothing to take back.
        let by_stranger = stranger.send("DELETE", &path, None).await?;
        assert_eq!(
            by_stranger.code(),
            "resource.not-found",
            "{}",
            by_stranger.body
        );
        // Its person takes the offer back.
        let who = if owner == 0 { &mut tess } else { &mut ruth };
        let back = who.send("DELETE", &path, None).await?;
        assert_eq!(back.status, StatusCode::NO_CONTENT, "{}", back.body);
        let after = work_of(&mut olga).await?;
        assert_eq!(after["offered_truncated"], json!(false), "{after}");
        assert_eq!(after["offered"].as_array().map(Vec::len), Some(MOST_AGENTS));
        // A stopped agent is offered to nobody, and cannot be accepted.
        let offered = after["offered"][0]["id"]
            .as_str()
            .ok_or("no offer")?
            .to_owned();
        let owner_of = |name: &str| {
            name.trim_start_matches("Agent ")
                .parse::<usize>()
                .unwrap_or(0)
                % 2
        };
        let name = after["offered"][0]["name"]
            .as_str()
            .ok_or("no name")?
            .to_owned();
        let who = if owner_of(&name) == 0 {
            &mut tess
        } else {
            &mut ruth
        };
        let current = who
            .send("GET", &format!("/api/v1/agents/{offered}"), None)
            .await?;
        let version = ok(&current)?["version"].clone();
        let stopped = who
            .send(
                "PATCH",
                &format!("/api/v1/agents/{offered}"),
                Some(json!({"action": "stop", "version": version})),
            )
            .await?;
        assert_eq!(stopped.status, StatusCode::OK, "{}", stopped.body);
        // The stop counts the version once, the offer that ended with it does not count again.
        assert_eq!(
            ok(&stopped)?["version"].as_i64(),
            version.as_i64().map(|before| before + 1),
            "{}",
            stopped.body
        );
        let remaining = work_of(&mut olga).await?;
        assert_eq!(
            remaining["offered"].as_array().map(Vec::len),
            Some(MOST_AGENTS - 1),
            "{remaining}"
        );
        let accepted = olga
            .send("POST", &format!("/api/v1/agents/{offered}/accept"), None)
            .await?;
        assert_eq!(accepted.code(), "resource.not-found", "{}", accepted.body);
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_32_the_rules_of_an_agent_come_open_first_then_ended_in_pages() -> TestResult {
    each_database("ai_32_agent_rules_pages", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let search = tess
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({"name": "New bugs", "search_text": ""})),
            )
            .await?;
        let search = ok(&search)?["id"].as_str().ok_or("no search")?.to_owned();
        let (agent, _) = new_agent(&mut tess, "Fixer").await?;
        let mut rules = Vec::new();
        for _ in 0..3 {
            let made = tess
                .send(
                    "POST",
                    "/api/v1/standing-rules",
                    Some(json!({"agent": agent, "search": search, "project": site.acme})),
                )
                .await?;
            let made = ok(&made)?.clone();
            rules.push(made);
            // Rules are told apart by when they were made.
            rig.clock.advance(60);
        }
        // The middle one is stopped; an administrator cannot change a rule that is not theirs.
        let middle = rules[1]["id"].as_str().ok_or("no rule")?.to_owned();
        let path = format!("/api/v1/standing-rules/{middle}");
        let by_admin = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"action": "stop", "version": rules[1]["version"]})),
            )
            .await?;
        assert_eq!(by_admin.code(), "resource.not-found", "{}", by_admin.body);
        let stopped = tess
            .send(
                "PATCH",
                &path,
                Some(json!({"action": "stop", "version": rules[1]["version"]})),
            )
            .await?;
        ok(&stopped)?;
        // Two to a page: those that work, newest first, then the one that ended, with nothing cut.
        let list = format!("/api/v1/agents/{agent}/standing-rules");
        let first = tess.send("GET", &format!("{list}?limit=2"), None).await?;
        let first = ok(&first)?.clone();
        let ids = |page: &Value| -> Vec<String> {
            page["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item["id"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        let id_of = |at: usize| rules[at]["id"].as_str().unwrap_or_default().to_owned();
        assert_eq!(ids(&first), vec![id_of(2), id_of(0)], "{first}");
        let cursor = first["next_cursor"]
            .as_str()
            .ok_or("cut without a cursor")?
            .to_owned();
        let second = tess
            .send("GET", &format!("{list}?limit=2&cursor={cursor}"), None)
            .await?;
        let second = ok(&second)?.clone();
        assert_eq!(ids(&second), vec![id_of(1)], "{second}");
        assert_eq!(second["items"][0]["state"], json!("ended"));
        assert!(second["next_cursor"].is_null(), "{second}");
        Ok::<(), Failure>(())
    })
    .await
}
