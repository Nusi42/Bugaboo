//! No N+1 queries in the timeline (PERF-05, PERF-N1): a page of forty rows over three projects,
//! with their bars, the people on them and their work times, the dependencies, the warnings and
//! who accepted them, runs as many statements as a page of five, for a reader who sees people by
//! name and for one who sees them by their role, on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, key_of, ok};
use super::perf_queries::{Counter, assert_as_many_statements, assert_same_statements};
use super::plan_site::{MONDAY, TIMELINE, blocks, issue, work_times};

/// How many issues the timeline of the test holds: more than the longer page in ACME alone, where
/// every fifth issue is in one of the two other projects. The five newest are in all three, so a
/// page of five meets every project a page of forty meets: what is read for each project (the
/// work times there, whose work times the reader may know) is read as often for both.
const ISSUES: usize = 56;

/// The most statements a page of the timeline of one project may run (90 measured; 94 before
/// pages met several projects).
const ONE_PROJECT_AT_MOST: usize = 100;

/// The most statements a page of the timeline may run: the search of the rows and the count of
/// *Not scheduled yet* (each with the fields and names of the reader), the page of issues, the
/// dependencies and sub-issues around it, the time factors, the work times of the people on the bars
/// in each project, the accepted warnings and who accepted them, each read once for the page (94
/// measured over three projects for the reader who sees people by their role).
const TIMELINE_AT_MOST: usize = 100;

/// The statements of a page of five and a page of forty of the timeline `body`, as `browser` reads
/// them, after a read that warms the caches (not counted).
async fn five_and_forty(
    browser: &mut Browser,
    counter: &mut Counter<'_>,
    body: &Value,
) -> Result<(usize, usize), Failure> {
    let limited = |limit: u32| {
        let mut asked = body.clone();
        if let Some(asked) = asked.as_object_mut() {
            asked.insert("limit".to_owned(), json!(limit));
        }
        asked
    };
    ok(&browser.send("POST", TIMELINE, Some(limited(5))).await?)?;
    counter.take();
    let five = browser.send("POST", TIMELINE, Some(limited(5))).await?;
    assert_eq!(ok(&five)?["rows"].as_array().map(Vec::len), Some(5));
    let (on_five, five_log) = counter.take_logged();
    let forty = browser.send("POST", TIMELINE, Some(limited(40))).await?;
    assert_eq!(ok(&forty)?["rows"].as_array().map(Vec::len), Some(40));
    // Every row but the oldest waits for the issue before it: the page holds their delays, planned
    // by effort.
    assert!(
        forty.body["warnings"]
            .as_array()
            .is_some_and(|warnings| warnings.len() >= 39),
        "{}",
        forty.body
    );
    assert_eq!(forty.body["planned_by_effort"], true);
    let (on_forty, forty_log) = counter.take_logged();
    assert_same_statements(&five_log, &forty_log);
    Ok((on_five, on_forty))
}

/// Accepts the delay of an issue, as `browser`.
async fn accept_delay(browser: &mut Browser, key: &str) -> Result<(), Failure> {
    let version = issue(browser, key).await?["version"].clone();
    ok(&browser
        .send(
            "POST",
            &format!("{TIMELINE}/accepted-warnings"),
            Some(json!({
                "issue": key,
                "version": version,
                "kind": "delay",
                "timeline": {"zoom": "week", "plan_by_effort": true},
            })),
        )
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn perf_05_a_timeline_of_forty_rows_runs_as_many_statements_as_one_of_five() -> TestResult {
    each_database("perf_05_timeline", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
        work_times(&site, &mut ada, lea, 360, 50).await?;
        work_times(&site, &mut ada, ben, 480, 100).await?;
        let mut projects = vec![site.acme.clone()];
        for short_name in ["BETA", "GAMMA"] {
            let made = ada
                .send(
                    "POST",
                    PROJECTS,
                    Some(json!({"short_name": short_name, "name": short_name})),
                )
                .await?;
            projects.push(ok(&made)?["id"].as_str().ok_or("no ID")?.to_owned());
        }
        let mut before: Option<String> = None;
        for number in 0..ISSUES {
            let first = if number % 2 == 0 { lea } else { ben };
            let project = if number + 5 >= ISSUES {
                &projects[number % 3]
            } else if number % 5 == 4 {
                &projects[1 + (number / 5) % 2]
            } else {
                &projects[0]
            };
            let key = key_of(
                &ada.send(
                    "POST",
                    &format!("{PROJECTS}/{project}/issues"),
                    Some(json!({
                        "issue_type": site.kind,
                        "title": format!("Step {number}"),
                        "start_date": MONDAY,
                        "estimated_time": 240,
                        "assigned_to": [first.to_string()],
                    })),
                )
                .await?,
            )?;
            // A chain of dependencies through the rows, each one a delay; every third is accepted.
            if let Some(earlier) = &before {
                blocks(&mut ada, earlier, &key).await?;
                if number % 3 == 0 {
                    accept_delay(&mut ada, &key).await?;
                }
            }
            before = Some(key);
        }
        let body = json!({
            "search_text": "",
            "timeline": {"zoom": "week", "plan_by_effort": true},
            "first": MONDAY,
            "last": "2027-05-31",
        });
        let mut counter = Counter::on(&rig.database);
        let (on_five, on_forty) = five_and_forty(&mut ada, &mut counter, &body).await?;
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "a page by name");
        // A reader who sees the people on the bars by their role.
        site.show_people(
            &mut ada,
            StartingRole::Contributor,
            "assigned-to",
            "role-only",
        )
        .await?;
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        // Of one project: the role labels of the people on the page are read once for the page.
        let mut one_project = body.clone();
        if let Some(asked) = one_project.as_object_mut() {
            asked.insert("project".to_owned(), json!(site.acme));
        }
        let (on_five, on_forty) =
            five_and_forty(&mut contributor, &mut counter, &one_project).await?;
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "a page by role");
        // The page of one project by name stays within what it ran before pages met several.
        let (on_five, on_forty) = five_and_forty(&mut ada, &mut counter, &one_project).await?;
        assert_as_many_statements(
            on_five,
            on_forty,
            ONE_PROJECT_AT_MOST,
            "one project by name",
        );
        Ok(())
    })
    .await
}
