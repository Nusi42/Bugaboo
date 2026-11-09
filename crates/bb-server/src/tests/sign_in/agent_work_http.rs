//! An agent with work as an actor of the routes (AI-31, ADR-0029, M7-F38): its person may do more
//! than it does, and it reaches only the issue it was handed, and only in the ways the hand-over
//! ticked. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::agent_work::{EndAttempt, TakeWork, end_attempt, take_work};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::rig::{PASSWORD, Rig};

/// A team member with an agent, the key of the agent, an issue handed over to it with the title
/// ticked, and another issue of the same project that was not.
struct Given {
    person: Browser,
    key: String,
    handed: String,
    other: String,
}

async fn given(site: &Site<'_>, ada: &mut Browser) -> Result<Given, Failure> {
    let (mut person, _) = site.person(StartingRole::TeamMember, "tess").await?;
    let agent = person
        .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
        .await?;
    let agent = ok(&agent)?["id"].as_str().ok_or("no agent ID")?.to_owned();
    let made = person
        .send(
            "POST",
            &format!("/api/v1/agents/{agent}/keys"),
            Some(json!({"password": PASSWORD, "name": "Key of Fixer"})),
        )
        .await?;
    let key = ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned();
    let handed = key_of(&site.create(ada, "Hand this over").await?)?;
    let other = key_of(&site.create(ada, "Not this one").await?)?;
    let hand_over = person
        .send(
            "POST",
            "/api/v1/hand-overs",
            Some(json!({
                "agent": agent,
                "issues": [handed],
                "note": "Fix the title",
                "powers": ["change-fields"],
                "fields": ["title"],
            })),
        )
        .await?;
    ok(&hand_over)?;
    Ok(Given {
        person,
        key,
        handed,
        other,
    })
}

#[tokio::test]
async fn sec_01_an_agent_with_work_reaches_only_its_issue_and_only_what_was_ticked() -> TestResult {
    each_database("sec_01_agent_with_work", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut given = given(&site, &mut ada).await?;
        let mut script = Browser::new(&app, &super::http::called());
        // It reads the issue it was handed, and not the other one of the project its person sees.
        let read = script
            .script(
                "GET",
                &format!("{ISSUES}/{}", given.handed),
                None,
                &given.key,
            )
            .await?;
        assert_eq!(read.status, StatusCode::OK, "{}", read.body);
        let hidden = script
            .script(
                "GET",
                &format!("{ISSUES}/{}", given.other),
                None,
                &given.key,
            )
            .await?;
        assert_eq!(hidden.code(), "resource.not-found", "{}", hidden.body);
        assert!(
            given
                .person
                .send("GET", &format!("{ISSUES}/{}", given.other), None)
                .await?
                .status
                == StatusCode::OK,
            "its person reads both"
        );
        // It changes the title, which was ticked, and not the priority, which was not: only while
        // an attempt runs, so it takes the issue first.
        take_work(
            &rig.services.ctx(Rig::info()),
            TakeWork {
                session_token: given.key.clone(),
                issue: given.handed.clone(),
                version: None,
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        let version = version_of(&mut given.person, &given.handed).await?;
        let title = script
            .script(
                "PATCH",
                &format!("{ISSUES}/{}", given.handed),
                Some(json!({"version": version, "title": "Fixed by the agent"})),
                &given.key,
            )
            .await?;
        assert_eq!(title.status, StatusCode::OK, "{}", title.body);
        let version = version_of(&mut given.person, &given.handed).await?;
        let priority = script
            .script(
                "PATCH",
                &format!("{ISSUES}/{}", given.handed),
                Some(json!({"version": version, "priority": "high"})),
                &given.key,
            )
            .await?;
        assert_eq!(priority.code(), "access.forbidden", "{}", priority.body);
        // It never hands work over, answers questions of its own work, or makes agents: those start
        // at a session.
        let hand_over = script
            .script(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": given.handed, "issues": [given.handed]})),
                &given.key,
            )
            .await?;
        assert_eq!(
            hand_over.code(),
            "auth.access-key-not-allowed",
            "{}",
            hand_over.body
        );
        let agents = script
            .script(
                "POST",
                "/api/v1/agents",
                Some(json!({"name": "Child"})),
                &given.key,
            )
            .await?;
        assert_eq!(
            agents.code(),
            "auth.access-key-not-allowed",
            "{}",
            agents.body
        );
        // It reads the agent work on its issue.
        let work = script
            .script(
                "GET",
                &format!("{ISSUES}/{}/agent-work", given.handed),
                None,
                &given.key,
            )
            .await?;
        assert_eq!(work.status, StatusCode::OK, "{}", work.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_01_a_hand_over_cannot_name_a_member_that_holds_people_or_is_none() -> TestResult {
    each_database("sec_01_hand_over_members", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut given = given(&site, &mut ada).await?;
        let agent = given.person.send("GET", "/api/v1/agents", None).await?.body["items"][0]["id"]
            .as_str()
            .ok_or("no agent")?
            .to_owned();
        let more = key_of(&site.create(&mut ada, "One more").await?)?;
        for member in ["assigned-to", "reported-by", "followers", "nope"] {
            let refused = given
                .person
                .send(
                    "POST",
                    "/api/v1/hand-overs",
                    Some(json!({
                        "agent": agent,
                        "issues": [more],
                        "powers": ["change-fields"],
                        "fields": ["title", member],
                    })),
                )
                .await?;
            assert_eq!(
                refused.code(),
                "request.invalid",
                "{member}: {}",
                refused.body
            );
        }
        // The title alone is fine.
        let fine = given
            .person
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({
                    "agent": agent,
                    "issues": [more],
                    "powers": ["change-fields"],
                    "fields": ["title"],
                })),
            )
            .await?;
        assert_eq!(fine.status, StatusCode::OK, "{}", fine.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_31_a_hand_over_with_one_piece_accepted_still_ends_and_stops_the_other() -> TestResult {
    each_database("ai_31_end_after_accept", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut given = given(&site, &mut ada).await?;
        let agent = given.person.send("GET", "/api/v1/agents", None).await?.body["items"][0]["id"]
            .as_str()
            .ok_or("no agent")?
            .to_owned();
        let third = key_of(&site.create(&mut ada, "The third one").await?)?;
        let both = given
            .person
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": agent, "issues": [given.other, third], "days": 1})),
            )
            .await?;
        ok(&both)?;
        // The agent finishes the work on one of the two, and the person accepts it.
        let ctx = rig.services.ctx(Rig::info());
        take_work(
            &ctx,
            TakeWork {
                session_token: given.key.clone(),
                issue: given.other.clone(),
                version: None,
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        end_attempt(
            &ctx,
            EndAttempt {
                session_token: given.key.clone(),
                issue: given.other.clone(),
                gave_up: None,
            },
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        let path = |issue: &str| format!("{ISSUES}/{issue}/agent-work");
        let shown = given.person.send("GET", &path(&given.other), None).await?;
        let work = ok(&shown)?["work"][0].clone();
        let accepted = given
            .person
            .send(
                "POST",
                &format!(
                    "/api/v1/agent-work/{}/review",
                    work["id"].as_str().ok_or("no work ID")?
                ),
                Some(json!({"answer": "accept", "version": work["version"]})),
            )
            .await?;
        ok(&accepted)?;
        // The hand-over runs out: the piece nobody finished stops, the accepted one stays.
        rig.clock.advance(24 * 3600 + 1);
        ctx.run_due_jobs()
            .await
            .map_err(|error| format!("{error:?}"))?;
        // The session ran out meanwhile: the person signs in again.
        let mut person = Browser::signed_in_as(&app, "tess@example.test").await?;
        let state_of = async |browser: &mut Browser, issue: &str| -> Result<Value, Failure> {
            let shown = browser.send("GET", &path(issue), None).await?;
            Ok(ok(&shown)?["work"][0].clone())
        };
        let left = state_of(&mut person, &third).await?;
        assert_eq!(left["state"], "stopped", "{left}");
        assert_eq!(left["open"], false);
        // The answer says why it stopped, on both databases: the hand-over ran out.
        assert_eq!(left["stopped_for"], "ran-out", "{left}");
        let done = state_of(&mut person, &given.other).await?;
        assert_eq!(done["state"], "ready-for-review", "{done}");
        assert!(done["stopped_for"].is_null(), "{done}");
        Ok(())
    })
    .await
}
