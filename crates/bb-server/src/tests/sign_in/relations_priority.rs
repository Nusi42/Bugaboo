//! The priority of the issues in the lines of the issue page (ISS-03, ISS-04, M3-F173): a
//! sub-issue, the issue it is part of and a related issue carry their priority, so the row can
//! show its edge, and a reader whose role hides the field gets none. Both databases.

use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::fields_http::reader_of;
use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, key_of, ok};

#[tokio::test]
async fn iss_03_lines_of_sub_issues_and_related_issues_carry_their_priority() -> TestResult {
    each_database("iss_03_priority_in_lines", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = key_of(&site.create(&mut ada, "Goal").await?)?;
        let part = key_of(&site.create(&mut ada, "Part").await?)?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{part}"),
                Some(json!({"version": 1, "priority": "high"})),
            )
            .await?)?;
        ok(&ada
            .send(
                "PUT",
                &format!("{ISSUES}/{part}/parent"),
                Some(json!({"version": 2, "parent": goal})),
            )
            .await?)?;
        let subs = ada
            .send("GET", &format!("{ISSUES}/{goal}/sub-issues"), None)
            .await?;
        assert_eq!(ok(&subs)?["items"][0]["priority"], "high");
        let own = ada
            .send("GET", &format!("{ISSUES}/{part}/relations"), None)
            .await?;
        assert_eq!(ok(&own)?["part_of"]["priority"], "normal");
        // A role that does not see the priority gets no priority in the lines.
        let mut reader = reader_of(
            &site,
            &rig,
            "lines@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                ("priority", None, FieldLevel::Hidden),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let seen = reader
            .send("GET", &format!("{ISSUES}/{goal}/sub-issues"), None)
            .await?;
        assert!(
            ok(&seen)?["items"][0].get("priority").is_none(),
            "{}",
            seen.body
        );
        Ok(())
    })
    .await
}
