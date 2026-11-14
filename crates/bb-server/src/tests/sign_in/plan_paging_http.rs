//! The pages of the timeline and the checks of what a view and a way out may ask (PLAN-01,
//! PLAN-13, ADR-0034, ADR-0043): a page holds at most what a page of any list holds, the pages of
//! the rows, of *Group by person* and of *Not scheduled yet* lose and repeat no row, and a view or a
//! way out that names what cannot be is no request. Every flow runs on SQLite and PostgreSQL.

use std::collections::HashSet;

use bb_app::access::StartingRole;
use bb_core::paging::PAGE_SIZE_MAX;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::create_field;
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::plan_site::{MONDAY, TIMELINE, ask, issue, page, planned};

/// Every row of the timeline `body` asks for, page by page with `limit` issues a page: each as its
/// readable ID and its group.
async fn all_rows(browser: &mut Browser, body: &Value, limit: u32) -> Result<Vec<String>, Failure> {
    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..PAGES_MOST {
        let mut asked = body.clone();
        if let Some(asked) = asked.as_object_mut() {
            asked.insert("limit".to_owned(), json!(limit));
            if let Some(cursor) = &cursor {
                asked.insert("cursor".to_owned(), json!(cursor));
            }
        }
        let found = page(browser, asked).await?;
        for row in found["rows"].as_array().into_iter().flatten() {
            rows.push(format!(
                "{} {}",
                row["issue"]["readable_id"].as_str().unwrap_or_default(),
                row["group"]
            ));
        }
        match found["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => return Ok(rows),
        }
    }
    Err(format!("more than {PAGES_MOST} pages: {}", rows.len()).into())
}

/// More pages than any walk of the test has.
const PAGES_MOST: usize = 20;

#[tokio::test]
async fn plan_01_the_pages_of_the_timeline_lose_and_repeat_no_row() -> TestResult {
    each_database("plan_01_timeline_pages", async |db| {
        // More requests than a person makes in a minute set up the pages.
        let rig = super::rig::Rig::start_with(db, super::rig::Tries::Default).await?;
        let limits = bb_api::LimitsConfig {
            requests_per_minute: 10_000,
            searches_per_minute: 10_000,
            ..bb_api::LimitsConfig::default()
        };
        let app = super::status::app_with_probe_and_limits(&rig, None, limits);
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
        let dated = PAGE_SIZE_MAX as usize + 1;
        for number in 0..dated {
            planned(
                &site,
                &mut ada,
                &format!("Dated {number}"),
                json!({"start_date": MONDAY, "assigned_to": [lea.to_string(), ben.to_string()]}),
            )
            .await?;
        }
        for number in 0..45 {
            planned(&site, &mut ada, &format!("Waiting {number}"), json!({})).await?;
        }
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        // A page holds at most what a page of any list holds (asking for more is refused, like in
        // every list).
        let mut most = body.clone();
        if let Some(most) = most.as_object_mut() {
            most.insert("limit".to_owned(), json!(PAGE_SIZE_MAX));
        }
        let first = page(&mut ada, most).await?;
        assert_eq!(
            first["rows"].as_array().map(Vec::len),
            Some(PAGE_SIZE_MAX as usize)
        );
        assert!(first["next_cursor"].is_string());
        // The rows, the rows by person and the issues waiting, page by page.
        let cases = [
            (body.clone(), 40, dated),
            (
                ask(
                    &site,
                    MONDAY,
                    "2027-03-31",
                    json!({"group_by": "assigned to"}),
                ),
                30,
                dated * 2,
            ),
            (
                ask(
                    &site,
                    MONDAY,
                    "2027-03-31",
                    json!({"part": "not-scheduled"}),
                ),
                20,
                45,
            ),
        ];
        for (asked, limit, expected) in cases {
            let rows = all_rows(&mut ada, &asked, limit).await?;
            let distinct: HashSet<&String> = rows.iter().collect();
            assert_eq!(rows.len(), expected, "{asked}");
            assert_eq!(distinct.len(), expected, "{asked}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_view_or_a_way_out_that_names_what_cannot_be_is_no_request() -> TestResult {
    each_database("plan_13_no_request", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = planned(
            &site,
            &mut ada,
            "Calm",
            json!({"start_date": MONDAY, "due_date": "2027-03-05"}),
        )
        .await?;
        let version = issue(&mut ada, &key).await?["version"].clone();
        let accept = |kind: &str, timeline: Value| {
            json!({"issue": key, "version": version, "kind": kind, "timeline": timeline})
        };
        // A warning the issue does not have, and a view of own date fields, are not kept.
        for body in [
            accept("delay", json!({"zoom": "week"})),
            accept(
                "delay",
                json!({"zoom": "week", "start_field": "Kickoff", "end_field": "Launch"}),
            ),
        ] {
            let refused = ada
                .send("POST", &format!("{TIMELINE}/accepted-warnings"), Some(body))
                .await?;
            assert_eq!(refused.code(), "request.invalid", "{}", refused.body);
        }
        // One date field without the other is no view, for a page and for a saved search.
        let half = json!({"zoom": "week", "start_field": "start date"});
        let refused = ada
            .send(
                "POST",
                TIMELINE,
                Some(ask(&site, MONDAY, "2027-03-31", json!({"timeline": half}))),
            )
            .await?;
        assert_eq!(refused.code(), "request.invalid", "{}", refused.body);
        // Neither is a saved search that takes its days from fields a bar cannot take its days
        // from: fields that hold no day, a date field the timeline does not plan with (*Created*),
        // and an own field with a day and a time.
        create_field(
            &mut ada,
            json!({"name": "Kickoff call", "kind": "date-time", "applies_to": "issues"}),
        )
        .await?;
        for timeline in [
            half,
            json!({"zoom": "week", "start_field": "title", "end_field": "status"}),
            json!({"zoom": "week", "start_field": "created", "end_field": "due date"}),
            json!({"zoom": "week", "start_field": "Kickoff call", "end_field": "due date"}),
        ] {
            let saved = ada
                .send(
                    "POST",
                    "/api/v1/saved-searches",
                    Some(json!({
                        "name": "Plan",
                        "search_text": "",
                        "presentation": {
                            "view": "timeline",
                            "columns": [],
                            "group_by": null,
                            "timeline": timeline,
                        },
                    })),
                )
                .await?;
            assert_eq!(saved.code(), "request.invalid", "{}", saved.body);
        }
        ok(&ada
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({
                    "name": "Plan",
                    "search_text": "",
                    "presentation": {
                        "view": "timeline",
                        "columns": [],
                        "group_by": null,
                        "timeline": {"zoom": "month", "start_field": "start date", "end_field": "due date"},
                    },
                })),
            )
            .await?)?;
        Ok(())
    })
    .await
}
