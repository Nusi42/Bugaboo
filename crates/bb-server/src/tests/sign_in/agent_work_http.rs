//! An agent with work as an actor of the routes (AI-31, ADR-0029, M7-F38): its person may do more
//! than it does, and it reaches only the issue it was handed, and only in the ways the hand-over
//! ticked. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::rig::PASSWORD;

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
        // It changes the title, which was ticked, and not the priority, which was not.
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
