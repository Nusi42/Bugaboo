//! The exit review of M3 (WP-12), what a hidden issue, name or field must not show (SEC-06, PRIV-13,
//! ISS-03, ISS-06). Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::comments::{MAX_RESOLVED_ISSUE_IDS, MAX_RESOLVED_MENTIONS};
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
        let part = site.create(&mut gus, "Part of Gus").await?;
        let part = ok(&part)?["key"].as_str().ok_or("no key")?.to_owned();
        let goal = site.create(&mut ada, "Ada's goal").await?;
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
        let secret = site.create(&mut ada, "Ada's secret").await?;
        let secret = ok(&secret)?["key"].as_str().ok_or("no key")?.to_owned();
        let own = site.create(&mut gus, "Another problem of Gus").await?;
        let own = ok(&own)?["key"].as_str().ok_or("no key")?.to_owned();
        let made = gus
            .send(
                "POST",
                &format!("/api/v1/projects/{}/issues", site.acme),
                Some(json!({
                    "issue_type": site.kind,
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

#[tokio::test]
async fn sec_06_one_text_resolves_a_bounded_number_of_mentions_and_issue_ids() -> TestResult {
    each_database("sec_06_token_limits", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let home = site.create(&mut ada, "Where it is written").await?;
        let home = ok(&home)?["key"].as_str().ok_or("no key")?.to_owned();
        let mut handles = Vec::new();
        for number in 0..MAX_RESOLVED_MENTIONS + 5 {
            let name = format!("Team{number:03}");
            ok(&ada
                .send("POST", "/api/v1/groups", Some(json!({"name": name})))
                .await?)?;
            handles.push(format!("@team{number:03}"));
        }
        let mut keys = Vec::new();
        for number in 0..MAX_RESOLVED_ISSUE_IDS + 5 {
            let made = site.create(&mut ada, &format!("Issue {number}")).await?;
            keys.push(ok(&made)?["key"].as_str().ok_or("no key")?.to_owned());
        }
        let text = format!("{} and {}", handles.join(" "), keys.join(" "));
        let written = ada
            .send(
                "POST",
                &format!("{ISSUES}/{home}/comments"),
                Some(json!({"text": text})),
            )
            .await?;
        let html = ok(&written)?["html"].as_str().ok_or("no html")?.to_owned();
        assert_eq!(
            html.matches("data-kind=\"mention\"").count(),
            MAX_RESOLVED_MENTIONS,
            "the mentions past the limit stay text"
        );
        assert_eq!(
            html.matches("data-kind=\"issue-id\"").count(),
            MAX_RESOLVED_ISSUE_IDS,
            "the issue IDs past the limit stay text"
        );
        assert!(
            html.contains(&handles[handles.len() - 1]),
            "the last stays text"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_whoever_cannot_restore_a_comment_is_not_told_it_is_in_the_trash() -> TestResult {
    each_database("sec_06_comment_trash", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = site.create(&mut ada, "Printer jams").await?;
        let issue = ok(&issue)?["key"].as_str().ok_or("no key")?.to_owned();
        let written = ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/comments"),
                Some(json!({"text": "First."})),
            )
            .await?;
        let id = ok(&written)?["id"].as_str().ok_or("no id")?.to_owned();
        ok_no_content(
            &ada.send("DELETE", &format!("/api/v1/comments/{id}"), None)
                .await?,
        );
        // A Viewer sees the issue and may not change it: the comment in the trash and one that is
        // not there answer alike.
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let in_trash = vera
            .send("POST", &format!("/api/v1/comments/{id}/restore"), None)
            .await?;
        let missing = vera
            .send(
                "POST",
                "/api/v1/comments/01890a5d-ac96-774b-bcce-b302099a8057/restore",
                None,
            )
            .await?;
        assert_eq!(in_trash.code(), "resource.not_found", "{}", in_trash.body);
        assert_eq!(missing.code(), "resource.not_found", "{}", missing.body);
        Ok(())
    })
    .await
}

fn ok_no_content(reply: &super::http::Reply) {
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
}
