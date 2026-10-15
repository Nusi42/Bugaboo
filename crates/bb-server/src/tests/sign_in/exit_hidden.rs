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
