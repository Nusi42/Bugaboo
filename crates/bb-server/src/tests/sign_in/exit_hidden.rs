//! The exit review of M3 (WP-12), what a hidden issue, name or field must not show (SEC-06, PRIV-13,
//! ISS-03, ISS-06). Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, ok};

#[tokio::test]
async fn sec_06_taking_out_a_parent_the_person_cannot_see_changes_nothing_and_says_nothing()
-> TestResult {
    each_database("sec_06_hidden_parent", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A Guest sees only the issues they reported.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let part = site.make(&mut gus, "Part of Gus").await?;
        let part = ok(&part)?["key"].as_str().ok_or("no key")?.to_owned();
        let goal = site.make(&mut ada, "Ada's goal").await?;
        let goal = ok(&goal)?["key"].as_str().ok_or("no key")?.to_owned();
        let seen = ada.send("GET", &format!("{ISSUES}/{part}"), None).await?;
        let version = ok(&seen)?["issue"]["version"].clone();
        ok(&ada
            .send(
                "PUT",
                &format!("{ISSUES}/{part}/parent"),
                Some(json!({"version": version, "parent": goal})),
            )
            .await?)?;
        // Gus sees his issue under no parent, and takes "none" out: nothing happens.
        let seen = gus.send("GET", &format!("{ISSUES}/{part}"), None).await?;
        let version = ok(&seen)?["issue"]["version"].clone();
        let out = gus
            .send(
                "PUT",
                &format!("{ISSUES}/{part}/parent"),
                Some(json!({"version": version, "parent": null})),
            )
            .await?;
        assert_eq!(out.status, StatusCode::OK, "{}", out.body);
        let after = gus.send("GET", &format!("{ISSUES}/{part}"), None).await?;
        assert_eq!(
            ok(&after)?["issue"]["version"],
            version,
            "the version did not count up"
        );
        // Ada still sees the issue under the goal.
        let relations = ada
            .send("GET", &format!("{ISSUES}/{part}/relations"), None)
            .await?;
        assert_eq!(ok(&relations)?["part_of"]["key"], goal);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_the_description_makes_a_chip_only_of_what_the_reader_may_see() -> TestResult {
    each_database("sec_06_description_chips", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let secret = site.make(&mut ada, "Ada's secret").await?;
        let secret = ok(&secret)?["key"].as_str().ok_or("no key")?.to_owned();
        let own = site.make(&mut gus, "Another problem of Gus").await?;
        let own = ok(&own)?["key"].as_str().ok_or("no key")?.to_owned();
        let made = gus
            .send(
                "POST",
                &format!("/api/v1/projects/{}/issues", site.acme),
                Some(json!({
                    "issue_type": site.bug,
                    "title": "Gus's problem",
                    "description": format!("See {secret} and {own}."),
                })),
            )
            .await?;
        let theirs = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        let read = gus.send("GET", &format!("{ISSUES}/{theirs}"), None).await?;
        let html = ok(&read)?["description_html"]
            .as_str()
            .ok_or("no html")?
            .to_owned();
        assert_eq!(html.matches("data-kind=\"issue-id\"").count(), 1, "{html}");
        assert!(html.contains(&secret) && html.contains(&own), "{html}");
        // Ada sees both as chips.
        let read = ada.send("GET", &format!("{ISSUES}/{theirs}"), None).await?;
        let html = ok(&read)?["description_html"].as_str().ok_or("no html")?;
        assert_eq!(html.matches("data-kind=\"issue-id\"").count(), 2, "{html}");
        Ok(())
    })
    .await
}
