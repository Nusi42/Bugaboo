//! The routes of searching over HTTP (QRY-04, QRY-05, SEC-17, PERF-05): a mistake is answered with
//! the checker's key and range, the sixty-first search of a minute waits, a cursor opens only for
//! the person and the search it was made for, and a page of many issues over many projects runs as
//! many statements as a small one.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, called, rig_and_app};
use super::issues_site::{PROJECTS, Site, ok, text_in};
use super::perf_queries::{Counter, assert_as_many_statements, five_and_forty};

/// More than any search may run, however long the page or the list of projects (a ceiling that a
/// change has to notice, 60 measured; the exact numbers are compared apart, M5-F45).
const AT_MOST: usize = 60;

const WRONG: &str = "/api/v1/search/issues?q=nonsense%20is%20x";

#[tokio::test]
async fn qry_05_a_mistake_in_a_search_is_answered_with_its_key_and_its_range() -> TestResult {
    each_database("search_http_mistake", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let wrong = ada.send("GET", WRONG, None).await?;
        assert_eq!(wrong.status, StatusCode::BAD_REQUEST);
        assert_eq!(wrong.code(), "search.invalid");
        let first = &wrong.body["mistakes"][0];
        assert!(
            first["key"].as_str().is_some_and(|key| !key.is_empty()),
            "{first}"
        );
        assert_eq!(first["start"], 0);
        assert!(first["end"].as_u64().is_some_and(|end| end > 0), "{first}");
        // The count and the groups answer the same mistake the same way.
        for path in [
            "/api/v1/search/issues/count?q=nonsense%20is%20x",
            "/api/v1/search/issues/groups?by=priority&q=nonsense%20is%20x",
        ] {
            let again = ada.send("GET", path, None).await?;
            assert_eq!(again.code(), "search.invalid", "{path}");
        }
        // A person who is not signed in is refused as such, before the text is read.
        let mut nobody = Browser::new(&app, &called());
        let refused = nobody.send("GET", WRONG, None).await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_the_sixty_first_search_of_a_minute_waits() -> TestResult {
    each_database("search_http_limit", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::TeamMember, "gus").await?;
        for number in 0..bb_api::limits::SEARCHES_PER_MINUTE {
            let done = ada.send("GET", "/api/v1/search/issues/count", None).await?;
            assert_eq!(done.status, StatusCode::OK, "search {number}");
        }
        let over = ada.send("GET", "/api/v1/search/issues/count", None).await?;
        assert_eq!(over.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(over.headers.contains_key("retry-after"));
        // Another person has their own allowance.
        let other = gus.send("GET", "/api/v1/search/issues/count", None).await?;
        assert_eq!(other.status, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_a_cursor_opens_only_for_the_person_and_the_search_it_was_made_for() -> TestResult {
    each_database("search_http_cursor", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::TeamMember, "gus").await?;
        for number in 0..3 {
            ok(&site.create(&mut ada, &format!("Issue {number}")).await?)?;
        }
        let first = ada
            .send("GET", "/api/v1/search/issues?limit=2", None)
            .await?;
        let cursor = text_in(ok(&first)?, "next_cursor")?;
        let path =
            |search: &str| format!("/api/v1/search/issues?limit=2&q={search}&cursor={cursor}");
        // The same person with the same search carries on.
        let next = ada.send("GET", &path(""), None).await?;
        assert_eq!(ok(&next)?["items"].as_array().map(Vec::len), Some(1));
        // Another search or another person cannot use it.
        let changed = ada.send("GET", &path("priority%20is%20high"), None).await?;
        assert_eq!(changed.status, StatusCode::BAD_REQUEST);
        let stolen = gus.send("GET", &path(""), None).await?;
        assert_eq!(stolen.status, StatusCode::BAD_REQUEST);
        assert_eq!(stolen.code(), changed.code());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_found_issues_runs_as_many_statements_as_five() -> TestResult {
    each_database("search_http_perf_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for number in 0..40 {
            let key = text_in(
                ok(&site.create(&mut ada, &format!("Issue {number}")).await?)?,
                "readable_id",
            )?;
            ok(&ada
                .send(
                    "POST",
                    &format!("/api/v1/issues/{key}/tags"),
                    Some(json!({"name": format!("tag{}", number % 7)})),
                )
                .await?)?;
        }
        let path = |limit: u32| format!("/api/v1/search/issues?limit={limit}");
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, path).await?;
        assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page");
        Ok(())
    })
    .await
}

/// Makes an issue in a project, by its ID.
async fn issue_in(
    site: &Site<'_>,
    ada: &mut Browser,
    project: &str,
    number: u32,
) -> Result<(), Failure> {
    let made = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{project}/issues"),
            Some(site.new_issue(&format!("Issue {number}"))),
        )
        .await?;
    ok(&made)?;
    Ok(())
}

/// Makes a project and returns its ID.
async fn project_called(ada: &mut Browser, short: &str) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": short, "name": short})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

#[tokio::test]
async fn perf_05_a_page_over_many_projects_runs_as_many_statements_as_one_over_two() -> TestResult {
    each_database("search_http_perf_projects", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // Forty issues in two projects.
        let second = project_called(&mut ada, "TWO").await?;
        for number in 0..20 {
            issue_in(&site, &mut ada, &site.acme, number).await?;
            issue_in(&site, &mut ada, &second, number).await?;
        }
        let path = "/api/v1/search/issues?limit=40";
        let mut counter = Counter::on(&rig.database);
        ok(&ada.send("GET", path, None).await?)?;
        counter.take();
        let few = ada.send("GET", path, None).await?;
        assert_eq!(ok(&few)?["items"].as_array().map(Vec::len), Some(40));
        let on_two = counter.take();
        // Eight more projects, five issues each: the forty newest now stand in eight projects.
        for project in 0..8 {
            let made = project_called(&mut ada, &format!("P{project}")).await?;
            for number in 0..5 {
                issue_in(&site, &mut ada, &made, number).await?;
            }
        }
        counter.take();
        let many = ada.send("GET", path, None).await?;
        assert_eq!(ok(&many)?["items"].as_array().map(Vec::len), Some(40));
        let on_many = counter.take();
        assert_as_many_statements(on_two, on_many, AT_MOST, "a page over more projects");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_the_list_of_a_project_holds_the_projects_below_it() -> TestResult {
    each_database("search_http_list_below", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let below = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "BELOW", "name": "Below"})),
            )
            .await?;
        let below = text_in(ok(&below)?, "id")?;
        issue_in(&site, &mut ada, &site.acme, 1).await?;
        issue_in(&site, &mut ada, &below, 2).await?;
        issue_in(&site, &mut ada, &site.acme, 3).await?;
        let list = ada
            .send("GET", &format!("{PROJECTS}/{}/issues", site.acme), None)
            .await?;
        let titles: Vec<_> = ok(&list)?["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|each| each["title"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        // The test server's clock stands still, so issues made one after the other share their
        // moment: the order among them is the one of their IDs, and only the set is stable.
        let mut sorted = titles.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, ["Issue 1", "Issue 2", "Issue 3"]);
        // The list of the sub-project holds its own issue only.
        let inside = ada
            .send("GET", &format!("{PROJECTS}/{below}/issues"), None)
            .await?;
        assert_eq!(ok(&inside)?["items"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_an_access_key_searches_as_its_person_and_has_the_same_allowance() -> TestResult {
    use bb_core::access_key::KeyChoice;
    each_database("search_http_key", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ok(&site.create(&mut ada, "Found by a script").await?)?;
        let key = super::access_keys::key_of(&rig, &site.org.admin, KeyChoice::Everything).await?;
        let mut script = Browser::new(&app, &called());
        let found = script
            .script(
                "GET",
                "/api/v1/search/issues?q=priority%20is%20normal",
                None,
                &key,
            )
            .await?;
        assert_eq!(ok(&found)?["items"].as_array().map(Vec::len), Some(1));
        // A mistake is answered the same way as for a person with a session.
        let wrong = script.script("GET", WRONG, None, &key).await?;
        assert_eq!(wrong.code(), "search.invalid");
        // The allowance is the person's, whichever way they come: the searches of the key and of
        // the session of Ada add up (two are made already).
        for number in 2..bb_api::limits::SEARCHES_PER_MINUTE {
            let done = script
                .script("GET", "/api/v1/search/issues/count", None, &key)
                .await?;
            assert_eq!(done.status, StatusCode::OK, "search {number}");
        }
        let by_session = ada.send("GET", "/api/v1/search/issues/count", None).await?;
        assert_eq!(by_session.status, StatusCode::TOO_MANY_REQUESTS);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_the_groups_by_a_person_field_run_as_many_statements_for_ten_people_as_for_three()
-> TestResult {
    each_database("search_http_perf_groups", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let path = "/api/v1/search/issues/groups?by=reportedBy";
        let mut counter = Counter::on(&rig.database);
        let mut made = 0_u32;
        let mut measure = async |until: u32, ada: &mut Browser| -> Result<usize, Failure> {
            while made < until {
                let (mut person, _) = site
                    .person(StartingRole::TeamMember, &format!("reporter{made}"))
                    .await?;
                ok(&site.create(&mut person, &format!("By {made}")).await?)?;
                made += 1;
            }
            // The first read warms what is kept for a request; the second is the one counted.
            let first = ada.send("GET", path, None).await?;
            assert_eq!(
                ok(&first)?["groups"].as_array().map(Vec::len),
                Some(until as usize)
            );
            counter.take();
            ok(&ada.send("GET", path, None).await?)?;
            Ok(counter.take())
        };
        let few = measure(3, &mut ada).await?;
        let many = measure(10, &mut ada).await?;
        assert_as_many_statements(few, many, AT_MOST, "more people in the groups");
        Ok(())
    })
    .await
}
