//! The title of the issue on the time entries of a week (TIME-04, PRIV-09): a list names
//! `issue_title` in `fields`, and the title is there only while the reader sees the issue and their
//! role shows the field. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::time_http::{MY_ENTRIES, book};

#[tokio::test]
async fn time_04_the_entries_of_a_week_carry_the_title_of_their_issue_when_asked() -> TestResult {
    each_database("time_04_issue_title", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        book(&mut ada, &issue, 30).await?;
        let range = "from=2027-01-11&to=2027-01-17";
        let plain = ada
            .send("GET", &format!("{MY_ENTRIES}?{range}"), None)
            .await?;
        assert!(ok(&plain)?["items"][0].get("issue_title").is_none());
        let asked = ada
            .send(
                "GET",
                &format!("{MY_ENTRIES}?{range}&fields=id,issue,issue_title"),
                None,
            )
            .await?;
        assert_eq!(
            ok(&asked)?["items"][0]["issue_title"],
            "Printer jams",
            "{}",
            asked.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_04_a_role_that_hides_the_title_gets_no_title_and_a_lost_issue_gets_no_name()
-> TestResult {
    each_database("time_04_issue_title_rights", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut tom, _) = site.person_in_acme(StartingRole::TeamMember, "tom").await?;
        ok(&book(&mut tom, &issue, 30).await?)?;
        let range = "from=2027-01-11&to=2027-01-17&fields=id,issue,readable_id,issue_title";
        let list = format!("{MY_ENTRIES}?{range}");
        let seen = tom.send("GET", &list, None).await?;
        assert_eq!(ok(&seen)?["items"][0]["issue_title"], "Printer jams");
        // Hidden *Title* for his role in ACME: the entry is still listed, with no title.
        let role = site.role_id(StartingRole::TeamMember);
        let hide = ada
            .send(
                "PUT",
                &format!(
                    "/api/v1/field-permissions/{role}/title?project={}",
                    site.acme
                ),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert_eq!(hide.status, StatusCode::NO_CONTENT, "{}", hide.body);
        let masked = tom.send("GET", &list, None).await?;
        let item = &ok(&masked)?["items"][0];
        assert!(item.get("issue_title").is_none(), "{item}");
        assert_eq!(item["readable_id"], issue.as_str());
        // The issue moves to a project he is not in: neither its ID nor its title is his to read.
        let beta = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?;
        let version = version_of(&mut ada, &issue).await?;
        let moved = ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/move"),
                Some(json!({"project": ok(&beta)?["id"], "version": version})),
            )
            .await?;
        ok(&moved)?;
        let gone = tom.send("GET", &list, None).await?;
        let item = &ok(&gone)?["items"][0];
        assert!(item.get("issue_title").is_none(), "{item}");
        assert!(item.get("readable_id").is_none(), "{item}");
        Ok(())
    })
    .await
}
