//! Standing rules that hand over what they find, through the real wiring, on both databases (AI-32,
//! ADR-0029, amendment "Rules that hand over", WP-14): a rule writes the match, the hand-over, its
//! work and the job of its end in one change; the limit of the agent caps the open work, also when
//! two looks happen at the same moment; the rest are handed over when room is made by the end of a
//! hand-over, by a review, or by the run-out of a hand-over.

use bb_app::access::StartingRole;
use bb_app::agent_work::{
    AgentWorkRecords as _, AttachResult, EndAttempt, GrantStore as _, TakeWork, attach_result,
};
use bb_app::agent_work::{end_attempt, take_work};
use bb_app::context::Clock as _;
use bb_core::agent_records::ResultKind;
use bb_core::agent_work::CheckResult;
use bb_core::id::{AgentId, IssueId, StandingRuleId};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, id_of, key_of, ok, version_of};
use super::rig::{PASSWORD, Rig};

/// A team member with an agent and its key, a rule that hands over, and the issues of ACME.
struct Crew {
    tess: Browser,
    agent: String,
    key: String,
    rule: String,
}

fn id_text<T: std::str::FromStr>(text: &str) -> Result<T, Failure> {
    text.parse().map_err(|_| format!("no ID in {text}").into())
}

/// The agent of the person, with its key and the limit of issues at once.
async fn agent_with_limit(tess: &mut Browser, limit: i64) -> Result<(String, String), Failure> {
    let made = tess
        .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
        .await?;
    let made = ok(&made)?.clone();
    let agent = made["id"].as_str().ok_or("no agent ID")?.to_owned();
    let limits = tess
        .send(
            "PATCH",
            &format!("/api/v1/agents/{agent}"),
            Some(json!({
                "limits": {
                    "changes_a_minute": 60,
                    "issues_at_once": limit,
                    "attempts_on_one_issue": 5,
                },
                "version": made["version"],
            })),
        )
        .await?;
    ok(&limits)?;
    let key = tess
        .send(
            "POST",
            &format!("/api/v1/agents/{agent}/keys"),
            Some(json!({"password": PASSWORD, "name": "Key of Fixer"})),
        )
        .await?;
    let key = ok(&key)?["secret"].as_str().ok_or("no secret")?.to_owned();
    Ok((agent, key))
}

/// A saved search of the person and a rule of the agent over it that hands over what it finds.
async fn rule_over(
    site: &Site<'_>,
    tess: &mut Browser,
    agent: &str,
    (text, name): (&str, &str),
    accept_by_itself: bool,
) -> Result<String, Failure> {
    let search = tess
        .send(
            "POST",
            "/api/v1/saved-searches",
            Some(json!({"name": name, "search_text": text})),
        )
        .await?;
    let search = ok(&search)?["id"]
        .as_str()
        .ok_or("no search ID")?
        .to_owned();
    let rule = tess
        .send(
            "POST",
            "/api/v1/standing-rules",
            Some(json!({
                "agent": agent,
                "search": search,
                "project": site.acme,
                "way": "hand-over",
                "powers": [],
                "fields": [],
                "accept_by_itself": accept_by_itself,
            })),
        )
        .await?;
    Ok(ok(&rule)?["id"].as_str().ok_or("no rule ID")?.to_owned())
}

/// A team member with an agent whose limit is `limit`, and a rule over every issue of ACME.
async fn crew(site: &Site<'_>, limit: i64) -> Result<Crew, Failure> {
    crew_accepting(site, limit, false).await
}

/// The same, whose hand-overs may be accepted by themselves when `accept_by_itself`.
async fn crew_accepting(
    site: &Site<'_>,
    limit: i64,
    accept_by_itself: bool,
) -> Result<Crew, Failure> {
    let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
    let (agent, key) = agent_with_limit(&mut tess, limit).await?;
    let rule = rule_over(
        site,
        &mut tess,
        &agent,
        ("", "Everything"),
        accept_by_itself,
    )
    .await?;
    Ok(Crew {
        tess,
        agent,
        key,
        rule,
    })
}

async fn issues(
    site: &Site<'_>,
    ada: &mut Browser,
    titles: &[&str],
) -> Result<Vec<(String, IssueId)>, Failure> {
    let mut made = Vec::new();
    for title in titles {
        let reply = site.create(ada, title).await?;
        made.push((key_of(&reply)?, id_text(&id_of(&reply)?)?));
    }
    Ok(made)
}

/// How many pieces of work the issue has, as its person reads them.
async fn works_on(tess: &mut Browser, issue: &str) -> Result<usize, Failure> {
    let shown = tess
        .send("GET", &format!("{ISSUES}/{issue}/agent-work"), None)
        .await?;
    Ok(ok(&shown)?["work"].as_array().map_or(0, Vec::len))
}

/// The jobs that look at the rules: what the notification round queues after the changes, then
/// the jobs run, as the server's workers do.
async fn look_again(rig: &Rig) -> Result<(), Failure> {
    rig.services
        .ctx(Rig::info())
        .catch_up_notifications()
        .await
        .map_err(|error| format!("{error:?}"))?;
    rig.run_jobs().await?;
    Ok(())
}

async fn open_work(rig: &Rig, agent: &str) -> Result<u64, Failure> {
    Ok(rig
        .storage
        .open_work_count_of(id_text::<AgentId>(agent)?, rig.clock.now())
        .await?)
}

async fn found_by(rig: &Rig, rule: &str) -> Result<usize, Failure> {
    Ok(rig
        .storage
        .matches_of(id_text::<StandingRuleId>(rule)?, None, 100)
        .await?
        .len())
}

#[tokio::test]
async fn m7_wp14_a_rule_writes_its_hand_over_in_one_change_and_stops_at_the_cap() -> TestResult {
    each_database("m7_wp14_rule_cap", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut crew = crew(&site, 2).await?;
        let made = issues(&site, &mut ada, &["One", "Two", "Three"]).await?;
        rig.run_jobs().await?;
        // Two places: two issues are handed over, with the match, the work and the job of the end
        // of each; the third is neither handed over nor written down.
        let mut handed = 0;
        for (key, _) in &made {
            handed += works_on(&mut crew.tess, key).await?;
        }
        assert_eq!(handed, 2);
        assert_eq!(found_by(&rig, &crew.rule).await?, 2);
        assert_eq!(open_work(&rig, &crew.agent).await?, 2);
        // (The job of the end of each hand-over is shown by the run-out test below.)
        // Looked at again, nothing more: the agent has no room.
        issues(&site, &mut ada, &["Four"]).await?;
        look_again(&rig).await?;
        assert_eq!(open_work(&rig, &crew.agent).await?, 2);
        assert_eq!(found_by(&rig, &crew.rule).await?, 2);
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn m7_f234_a_hand_over_that_runs_out_makes_room_for_the_issue_that_waited() -> TestResult {
    each_database("m7_f234_run_out", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let crew = crew(&site, 1).await?;
        let made = issues(&site, &mut ada, &["One", "Two"]).await?;
        rig.run_jobs().await?;
        assert_eq!(open_work(&rig, &crew.agent).await?, 1);
        assert_eq!(found_by(&rig, &crew.rule).await?, 1);
        // The hand-over runs out (14 days); the rule still works (30 days). The round that sees the
        // end queues the look, which hands the other issue over.
        rig.clock.advance(15 * 24 * 3600);
        rig.run_jobs().await?;
        look_again(&rig).await?;
        assert_eq!(
            open_work(&rig, &crew.agent).await?,
            1,
            "the other is handed"
        );
        assert_eq!(found_by(&rig, &crew.rule).await?, 2);
        let mut tess = Browser::signed_in_as(&app, "tess@example.test").await?;
        let (one, two) = (
            works_on(&mut tess, &made[0].0).await?,
            works_on(&mut tess, &made[1].0).await?,
        );
        assert_eq!(
            one + two,
            2,
            "the first keeps its stopped work, the other has new work"
        );
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn m7_f234_a_review_that_ends_work_makes_room_for_the_issue_that_waited() -> TestResult {
    each_database("m7_f234_review", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut crew = crew(&site, 1).await?;
        let made = issues(&site, &mut ada, &["One", "Two"]).await?;
        rig.run_jobs().await?;
        let (first, second) = if works_on(&mut crew.tess, &made[0].0).await? == 1 {
            (&made[0], &made[1])
        } else {
            (&made[1], &made[0])
        };
        // The agent works and says it is ready; the person throws the result away, which ends the
        // work and the hand-over: there is room again. The agent changed nothing on the issue, so
        // no change of an issue is what makes the look come.
        let version_before = version_of(&mut crew.tess, &first.0).await?;
        let ctx = rig.services.ctx(Rig::info());
        take_work(
            &ctx,
            TakeWork {
                session_token: crew.key.clone(),
                issue: first.0.clone(),
                version: None,
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        end_attempt(
            &ctx,
            EndAttempt {
                session_token: crew.key.clone(),
                issue: first.0.clone(),
                gave_up: None,
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        let shown = crew
            .tess
            .send("GET", &format!("{ISSUES}/{}/agent-work", first.0), None)
            .await?;
        let work = ok(&shown)?["work"][0].clone();
        let thrown = crew
            .tess
            .send(
                "POST",
                &format!(
                    "/api/v1/agent-work/{}/review",
                    work["id"].as_str().ok_or("no work ID")?
                ),
                Some(json!({"answer": "throw-away", "version": work["version"]})),
            )
            .await?;
        ok(&thrown)?;
        assert_eq!(open_work(&rig, &crew.agent).await?, 0);
        assert_eq!(
            version_of(&mut crew.tess, &first.0).await?,
            version_before,
            "nothing on the issue changed"
        );
        look_again(&rig).await?;
        assert_eq!(works_on(&mut crew.tess, &second.0).await?, 1);
        assert_eq!(open_work(&rig, &crew.agent).await?, 1);
        Ok::<(), Failure>(())
    })
    .await
}

/// The first of two issues is the one that was handed over; the agent takes it, and `finish` ends it.
async fn room_after(
    rig: &Rig,
    crew: &mut Crew,
    made: &[(String, IssueId)],
    attach_a_passed_check: bool,
    gave_up: Option<&str>,
) -> Result<String, Failure> {
    let (first, second) = if works_on(&mut crew.tess, &made[0].0).await? == 1 {
        (&made[0], &made[1])
    } else {
        (&made[1], &made[0])
    };
    let ctx = rig.services.ctx(Rig::info());
    take_work(
        &ctx,
        TakeWork {
            session_token: crew.key.clone(),
            issue: first.0.clone(),
            version: None,
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    if attach_a_passed_check {
        attach_result(
            &ctx,
            AttachResult {
                session_token: crew.key.clone(),
                issue: first.0.clone(),
                kind: ResultKind::Check,
                label: "Unit tests".to_owned(),
                link: None,
                outcome: Some(CheckResult::Passed),
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
    }
    end_attempt(
        &ctx,
        EndAttempt {
            session_token: crew.key.clone(),
            issue: first.0.clone(),
            gave_up: gave_up.map(str::to_owned),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    Ok(second.0.clone())
}

#[tokio::test]
async fn m7_f234_an_agent_that_gives_up_makes_room_for_the_issue_that_waited() -> TestResult {
    each_database("m7_f234_gave_up", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut crew = crew(&site, 1).await?;
        let made = issues(&site, &mut ada, &["One", "Two"]).await?;
        rig.run_jobs().await?;
        let second =
            room_after(&rig, &mut crew, &made, false, Some("no steps to reproduce")).await?;
        assert_eq!(works_on(&mut crew.tess, &second).await?, 0, "no room yet");
        look_again(&rig).await?;
        assert_eq!(works_on(&mut crew.tess, &second).await?, 1);
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn m7_f234_a_result_accepted_by_itself_makes_room_for_the_issue_that_waited() -> TestResult {
    each_database("m7_f234_accepted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let allowed = ada
            .send(
                "PUT",
                &format!(
                    "/api/v1/projects/{}/settings/agents.accept-by-itself",
                    site.acme
                ),
                Some(json!({"value": true})),
            )
            .await?;
        ok(&allowed)?;
        let mut crew = crew_accepting(&site, 1, true).await?;
        let made = issues(&site, &mut ada, &["One", "Two"]).await?;
        rig.run_jobs().await?;
        let second = room_after(&rig, &mut crew, &made, true, None).await?;
        look_again(&rig).await?;
        assert_eq!(works_on(&mut crew.tess, &second).await?, 1);
        Ok::<(), Failure>(())
    })
    .await
}

/// Four of five places are taken; two rules of the agent each find one issue and look at the same moment.
async fn two_rules(rig: &Rig, app: &axum::Router) -> Result<(), Failure> {
    let (site, mut ada) = Site::start(rig, app).await?;
    let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
    let (agent, _) = agent_with_limit(&mut tess, 5).await?;
    // Four of the five places are taken by hand-overs of the person.
    let fillers = issues(&site, &mut ada, &["f1", "f2", "f3", "f4"]).await?;
    let keys: Vec<&String> = fillers.iter().map(|(key, _)| key).collect();
    let handed = tess
        .send(
            "POST",
            "/api/v1/hand-overs",
            Some(json!({"agent": agent, "issues": keys, "note": "", "powers": [], "fields": []})),
        )
        .await?;
    ok(&handed)?;
    let alpha = issues(&site, &mut ada, &["alpha"]).await?;
    let beta = issues(&site, &mut ada, &["beta"]).await?;
    rule_over(
        &site,
        &mut tess,
        &agent,
        ("title contains alpha", "Alpha"),
        false,
    )
    .await?;
    rule_over(
        &site,
        &mut tess,
        &agent,
        ("title contains beta", "Beta"),
        false,
    )
    .await?;
    // Two looks are waiting (the creation of each rule queued one); they run at the same moment,
    // each with its own lease.
    let (first, second) = tokio::join!(rig.run_jobs(), rig.run_jobs());
    first?;
    second?;
    // One place was left: one of the two issues is handed over, never both.
    let open = open_work(rig, &agent).await?;
    assert_eq!(open, 5, "never above the limit");
    let on = works_on(&mut tess, &alpha[0].0).await? + works_on(&mut tess, &beta[0].0).await?;
    assert_eq!(on, 1);
    Ok(())
}

#[tokio::test]
async fn m7_wp14_two_rules_of_one_agent_looking_at_the_same_moment_pass_its_limit_once()
-> TestResult {
    each_database("m7_wp14_two_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Box::pin(two_rules(&rig, &app)).await
    })
    .await
}

#[tokio::test]
async fn m7_wp14_a_person_and_a_rule_handing_over_one_issue_at_once_give_it_one_open_work()
-> TestResult {
    each_database("m7_wp14_person_and_rule", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (agent, _) = agent_with_limit(&mut tess, 5).await?;
        let made = issues(&site, &mut ada, &["contested"]).await?;
        let rule = rule_over(&site, &mut tess, &agent, ("", "Everything"), false).await?;
        let by_hand =
            json!({"agent": agent, "issues": [made[0].0], "note": "", "powers": [], "fields": []});
        let (person, rules) = tokio::join!(
            tess.send("POST", "/api/v1/hand-overs", Some(by_hand)),
            rig.run_jobs()
        );
        let person = person?;
        rules?;
        // One of them handed it over; the other was told it was open, or only wrote it down.
        if person.status != axum::http::StatusCode::OK {
            assert_eq!(person.code(), "agent-work.already-open", "{}", person.body);
        }
        assert_eq!(open_work(&rig, &agent).await?, 1);
        assert_eq!(works_on(&mut tess, &made[0].0).await?, 1);
        assert_eq!(
            found_by(&rig, &rule).await?,
            1,
            "found once, whoever handed it over"
        );
        Ok::<(), Failure>(())
    })
    .await
}
