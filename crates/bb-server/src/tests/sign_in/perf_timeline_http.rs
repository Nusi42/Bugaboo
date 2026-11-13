//! No N+1 queries in the timeline (PERF-05, PERF-N1): a page of forty rows over three projects,
//! with their bars, the people on them and their work times, the dependencies, the warnings and
//! who accepted them, runs as many statements as a page of five, for a reader who sees people by
//! name, grouped by person, and for one who sees them by their role, on SQLite and PostgreSQL.

use std::collections::HashSet;

use bb_app::access::StartingRole;
use bb_store::Database;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, key_of, ok};
use super::perf_queries::{assert_as_many_statements, five_and_forty_of};
use super::plan_site::{MONDAY, TIMELINE, blocks, issue, work_times};

/// How many issues the timeline of the test holds: more than the longer page in ACME alone, where
/// every fifth issue is in one of the two other projects. The five newest are in all three, so a
/// page of five meets every project a page of forty meets: what is read for each project (the
/// work times there, whose work times the reader may know) is read as often for both.
const ISSUES: usize = 56;

/// The most statements a page of the timeline of one project may run for a reader who sees people
/// by name (91 measured; 94 before pages met several projects).
const ONE_PROJECT_AT_MOST: usize = 100;

/// The most statements a page of the timeline may run: the search of the rows and the count of
/// *Not scheduled yet* (each with the fields and names of the reader), the page of issues, the
/// dependencies and sub-issues around it, the time factors, the work times of the people on the bars
/// in each project, the accepted warnings, who accepted them and the role labels of those the
/// reader sees by their role, each read once for the page (for the reader who sees people by their
/// role: 98 measured over three projects, 99 in one; 90 over three projects by name, 95 grouped by
/// person, where each person's row reads their days once for the page).
const TIMELINE_AT_MOST: usize = 100;

/// The statements of a page of five and a page of forty of the timeline `body`, as `browser` reads
/// them, and the page of forty, which holds the delays of its rows, planned by effort.
async fn five_and_forty(
    browser: &mut Browser,
    database: &Database,
    body: &Value,
) -> Result<(usize, usize, Value), Failure> {
    let (on_five, on_forty, forty) = five_and_forty_of(browser, database, "rows", |limit| {
        let mut asked = body.clone();
        if let Some(asked) = asked.as_object_mut() {
            asked.insert("limit".to_owned(), json!(limit));
        }
        ("POST", TIMELINE.to_owned(), Some(asked))
    })
    .await?;
    // Every row but the oldest waits for the issue before it: the page holds their delays, planned
    // by effort.
    assert!(
        forty["warnings"]
            .as_array()
            .is_some_and(|warnings| warnings.len() >= 39),
        "{forty}"
    );
    assert_eq!(forty["planned_by_effort"], true);
    Ok((on_five, on_forty, forty))
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

/// The scene of the test: Lea and Ben on the bars with their work times, ACME and two more
/// projects, and a chain of issues through them whose delays Max and Mia accepted in turn.
async fn scene(site: &Site<'_>, ada: &mut Browser) -> Result<(), Failure> {
    let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
    let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
    // Two people on no bar accept the delays, in turn: whatever rows a page holds, it names
    // people who accepted its warnings and are named nowhere else on it.
    let (max, _) = site.person(StartingRole::TeamMember, "max").await?;
    let (mia, _) = site.person(StartingRole::TeamMember, "mia").await?;
    let mut accepters = [max, mia];
    work_times(site, ada, lea, 360, 50).await?;
    work_times(site, ada, ben, 480, 100).await?;
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
        // A chain of dependencies through the rows, each one a delay that Max or Mia accepted.
        if let Some(earlier) = &before {
            blocks(ada, earlier, &key).await?;
            accept_delay(&mut accepters[number % 2], &key).await?;
        }
        before = Some(key);
    }
    Ok(())
}

#[tokio::test]
async fn perf_05_a_timeline_of_forty_rows_runs_as_many_statements_as_one_of_five() -> TestResult {
    each_database("perf_05_timeline", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        scene(&site, &mut ada).await?;
        let body = json!({
            "search_text": "",
            "timeline": {"zoom": "week", "plan_by_effort": true},
            "first": MONDAY,
            "last": "2027-05-31",
        });
        let (on_five, on_forty, forty) = five_and_forty(&mut ada, &rig.database, &body).await?;
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "a page by name");
        // The page of forty shows warnings that two people accepted.
        let accepted: HashSet<String> = forty["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|warning| warning["accepted"]["by"]["id"].as_str())
            .map(str::to_owned)
            .collect();
        assert_eq!(accepted.len(), 2, "{forty}");
        // Grouped by person, by name: each person's row hatches their days, read once for the page.
        let mut by_person = body.clone();
        if let Some(asked) = by_person.as_object_mut() {
            asked.insert("group_by".to_owned(), json!("assigned to"));
        }
        let (on_five, on_forty, grouped) =
            five_and_forty(&mut ada, &rig.database, &by_person).await?;
        assert!(
            grouped["people_days"]
                .as_array()
                .is_some_and(|rows| rows.len() == 2),
            "{grouped}"
        );
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "a page by person");
        // A reader who sees the people on the bars and the people who accepted warnings by their
        // role, over all three projects.
        for kind in ["assigned-to", "authors"] {
            site.show_people(&mut ada, StartingRole::Contributor, kind, "role-only")
                .await?;
        }
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        let (on_five, on_forty, _) = five_and_forty(&mut contributor, &rig.database, &body).await?;
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "a page by role");
        // Of one project: the role labels of the people on the page are read once for the page.
        let mut one_project = body.clone();
        if let Some(asked) = one_project.as_object_mut() {
            asked.insert("project".to_owned(), json!(site.acme));
        }
        let (on_five, on_forty, _) =
            five_and_forty(&mut contributor, &rig.database, &one_project).await?;
        assert_as_many_statements(on_five, on_forty, TIMELINE_AT_MOST, "one project by role");
        // The page of one project by name stays within what it ran before pages met several.
        let (on_five, on_forty, _) = five_and_forty(&mut ada, &rig.database, &one_project).await?;
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
