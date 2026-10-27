//! The names a suggestion offers (QRY-05, ADR-0005 amendment M5, decision 9): a person or a group
//! the searcher may not name is not in the list, exactly as one who does not exist. Every flow runs
//! on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::Site;

const NAMES: &str = "/api/v1/search/names";

/// The names an answer lists.
async fn offered(browser: &mut Browser, query: &str) -> Result<Vec<String>, Failure> {
    let reply = browser
        .send("GET", &format!("{NAMES}?{query}"), None)
        .await?;
    assert_eq!(reply.status, StatusCode::OK, "{query}");
    Ok(reply.body["names"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}

#[tokio::test]
async fn qry_05_a_person_who_is_shown_no_names_is_offered_no_one_and_a_missing_name_looks_the_same()
-> TestResult {
    each_database("qry_05_search_names_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A guest sees people by their role only, so may name nobody else.
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        site.person_called(StartingRole::TeamMember, "cleo", "Cleo Cook")
            .await?;
        // An administrator is offered the people whose names start with the words.
        assert_eq!(
            offered(&mut ada, "kind=people&starting_with=cl").await?,
            ["Cleo Cook"]
        );
        // The guest is offered nobody else; Cleo, who exists, and Zed, who does not, look the same.
        let there = offered(&mut dan, "kind=people&starting_with=cleo").await?;
        let nothing = offered(&mut dan, "kind=people&starting_with=zed").await?;
        assert!(there.is_empty());
        assert_eq!(there, nothing);
        // A person is offered their own name.
        assert_eq!(
            offered(&mut dan, "kind=people&starting_with=dan").await?,
            ["Dan Dyer"]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_a_group_is_offered_to_a_person_who_may_name_it_only() -> TestResult {
    each_database("qry_05_search_names_groups", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        let made = ada
            .send("POST", "/api/v1/groups", Some(json!({ "name": "Backend" })))
            .await?;
        assert!(made.status.is_success());
        assert_eq!(
            offered(&mut ada, "kind=groups&starting_with=back").await?,
            ["Backend"]
        );
        // `member of` needs being in the group or managing groups: Dan is offered nothing.
        let there = offered(&mut dan, "kind=groups&starting_with=back").await?;
        let nothing = offered(&mut dan, "kind=groups&starting_with=zzz").await?;
        assert_eq!(there, nothing);
        assert!(there.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_a_start_that_is_too_long_or_a_kind_that_is_unknown_is_refused() -> TestResult {
    each_database("qry_05_search_names_refused", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let long = "a".repeat(101);
        let reply = ada
            .send(
                "GET",
                &format!("{NAMES}?kind=people&starting_with={long}"),
                None,
            )
            .await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        let reply = ada
            .send("GET", &format!("{NAMES}?kind=issues"), None)
            .await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_names_are_those_that_start_so_cut_at_the_limit_and_offered_once() -> TestResult {
    each_database("qry_05_search_names_start", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for (user, name) in [
            ("sam1", "Sam Same"),
            ("sam2", "Sam Same"),
            ("sara", "Sara Stone"),
            ("sven", "Sven Stahl"),
            ("tom", "Tom Sam"),
            ("pct", "50% Percent"),
        ] {
            site.person_called(StartingRole::TeamMember, user, name)
                .await?;
        }
        // Only the start counts, not a word inside the name; capitals do not count.
        assert_eq!(
            offered(&mut ada, "kind=people&starting_with=SA").await?,
            ["Sam Same", "Sara Stone"]
        );
        assert!(
            !offered(&mut ada, "kind=people&starting_with=sam")
                .await?
                .contains(&"Tom Sam".to_owned())
        );
        // Two people of one name are one name.
        let sam = ada
            .send(
                "GET",
                &format!("{NAMES}?kind=people&starting_with=sam"),
                None,
            )
            .await?;
        assert_eq!(sam.body["names"], json!(["Sam Same"]));
        // The answer says when it was cut off: three names start with "s" past a limit of two.
        let cut = ada
            .send(
                "GET",
                &format!("{NAMES}?kind=people&starting_with=s&limit=2"),
                None,
            )
            .await?;
        assert_eq!(cut.body["names"].as_array().map(Vec::len), Some(2));
        assert_eq!(cut.body["truncated"], true, "{}", cut.body);
        let whole = ada
            .send(
                "GET",
                &format!("{NAMES}?kind=people&starting_with=sv&limit=2"),
                None,
            )
            .await?;
        assert_eq!(whole.body["truncated"], false, "{}", whole.body);
        // The characters of a pattern mean themselves.
        assert_eq!(
            offered(&mut ada, "kind=people&starting_with=50%25").await?,
            ["50% Percent"]
        );
        assert!(
            offered(&mut ada, "kind=people&starting_with=%25")
                .await?
                .is_empty()
        );
        assert!(
            offered(&mut ada, "kind=people&starting_with=_a")
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}
