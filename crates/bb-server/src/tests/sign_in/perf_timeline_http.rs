//! No N+1 queries in the timeline (PERF-05, PERF-N1): a page of forty rows with their bars, the
//! people on them and their work times, the dependencies and the warnings runs as many statements
//! as a page of five, on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{Site, ok};
use super::perf_queries::{Counter, assert_as_many_statements, assert_same_statements};
use super::plan_site::{MONDAY, TIMELINE, blocks, by_effort, planned, work_times};

/// How many issues the timeline of the test holds: more than the longer page.
const ISSUES: usize = 41;

/// The most statements a page of the timeline may run: the search of the rows and the count of
/// *Not scheduled yet* (each with the fields and names of the reader), the page of issues, the
/// dependencies and sub-issues around it, the time factors, the work times of the people on the bars
/// in their project, and the accepted warnings, each read once for the page (94 measured).
const TIMELINE_AT_MOST: usize = 100;

#[tokio::test]
async fn perf_05_a_timeline_of_forty_rows_runs_as_many_statements_as_one_of_five() -> TestResult {
    each_database("perf_05_timeline", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
        work_times(&site, &mut ada, lea, 360, 50).await?;
        work_times(&site, &mut ada, ben, 480, 100).await?;
        let mut before: Option<String> = None;
        for number in 0..ISSUES {
            let first = if number % 2 == 0 { lea } else { ben };
            let key = planned(
                &site,
                &mut ada,
                &format!("Step {number}"),
                json!({
                    "start_date": MONDAY,
                    "estimated_time": 240,
                    "assigned_to": [first.to_string()],
                }),
            )
            .await?;
            // A chain of dependencies through the rows, each one a delay.
            if let Some(earlier) = &before {
                blocks(&mut ada, earlier, &key).await?;
            }
            before = Some(key);
        }
        let body = |limit: u32| {
            let mut body = by_effort(&site, MONDAY, "2027-05-31");
            if let Some(body) = body.as_object_mut() {
                body.insert("limit".to_owned(), json!(limit));
            }
            body
        };
        let mut counter = Counter::on(&rig.database);
        ok(&ada.send("POST", TIMELINE, Some(body(5))).await?)?;
        counter.take();
        let five = ada.send("POST", TIMELINE, Some(body(5))).await?;
        assert_eq!(ok(&five)?["rows"].as_array().map(Vec::len), Some(5));
        let (on_five, five_log) = counter.take_logged();
        let forty = ada.send("POST", TIMELINE, Some(body(40))).await?;
        assert_eq!(ok(&forty)?["rows"].as_array().map(Vec::len), Some(40));
        assert!(
            forty.body["warnings"]
                .as_array()
                .is_some_and(|warnings| warnings.len() >= 39),
            "{}",
            forty.body
        );
        let (on_forty, forty_log) = counter.take_logged();
        assert_same_statements(&five_log, &forty_log);
        assert_as_many_statements(
            on_five,
            on_forty,
            TIMELINE_AT_MOST,
            "a page of the timeline",
        );
        Ok(())
    })
    .await
}
