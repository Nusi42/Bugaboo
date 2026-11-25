//! Saved boards and calendars over HTTP (PLAN-06, PLAN-09, ADR-0034, amendment of 2026-11-24): the
//! options of the views are kept with the saved search and read back, and a shared board that
//! names a status the receiver cannot know does not check for them. Every flow runs on SQLite and
//! PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{Site, ok, text_in};
use super::views_site::{BOARD, own_workflow, project};

const SEARCHES: &str = "/api/v1/saved-searches";

/// The presentation of a saved board and calendar.
fn views(limit: u32) -> Value {
    json!({
        "view": "board",
        "columns": [],
        "group_by": null,
        "board": {
            "columns": null,
            "rows": "priority",
            "card_limits": [{"column": "Open", "limit": limit}, {"column": "Done", "limit": 9}],
        },
        "calendar": {"dates_field": "start date", "span": "week"},
    })
}

#[tokio::test]
async fn plan_06_a_saved_search_keeps_its_board_columns_rows_and_card_limits() -> TestResult {
    each_database("plan_06_saved_board", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let made = ok(&ada
            .send(
                "POST",
                SEARCHES,
                Some(json!({"name": "Work", "search_text": "", "presentation": views(3)})),
            )
            .await?)?
        .clone();
        let id = text_in(&made, "id")?;
        let read = ada
            .send(
                "GET",
                &format!("{SEARCHES}/{id}?fields=presentation,version"),
                None,
            )
            .await?;
        let presentation = ok(&read)?["presentation"].clone();
        assert_eq!(presentation["view"], "board");
        assert_eq!(presentation["board"], views(3)["board"], "{presentation}");
        assert_eq!(presentation["calendar"], views(3)["calendar"]);
        // A change keeps the new limits, in their order.
        let version = ok(&read)?["version"].clone();
        let changed = ada
            .send(
                "PATCH",
                &format!("{SEARCHES}/{id}"),
                Some(json!({"version": version, "presentation": views(5)})),
            )
            .await?;
        ok(&changed)?;
        let again = ada
            .send("GET", &format!("{SEARCHES}/{id}?fields=presentation"), None)
            .await?;
        assert_eq!(ok(&again)?["presentation"]["board"], views(5)["board"]);
        // Options a board cannot keep are refused: a limit of nothing, a column twice.
        let mut bad = views(0);
        let zero = ada
            .send(
                "POST",
                SEARCHES,
                Some(json!({"name": "Bad", "search_text": "", "presentation": bad.clone()})),
            )
            .await?;
        assert_eq!(zero.code(), "request.invalid", "{}", zero.body);
        bad["board"]["card_limits"] =
            json!([{"column": "Open", "limit": 2}, {"column": "open", "limit": 3}]);
        bad["board"]["columns"] = json!("title");
        let twice = ada
            .send(
                "POST",
                SEARCHES,
                Some(json!({"name": "Bad", "search_text": "", "presentation": bad})),
            )
            .await?;
        assert_eq!(twice.code(), "request.invalid", "{}", twice.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_shared_board_that_names_an_unknown_status_does_not_check_for_the_receiver()
-> TestResult {
    each_database("plan_06_shared_board", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // SEC has a workflow of its own with a status only its people know.
        let secret = project(&mut ada, "SEC").await?;
        own_workflow(
            &mut ada,
            &secret,
            (
                json!([
                    {"key": "open", "name": "Open", "status_group": "not-started"},
                    {"key": "vault", "name": "In the vault", "status_group": "done"},
                ]),
                json!([{"from": "open", "to": "vault", "button": "Lock away"}]),
            ),
        )
        .await?;
        let share = async |ada: &mut super::http::Browser, limit_on: &str| {
            let mut presentation = views(3);
            presentation["board"]["card_limits"] = json!([{"column": limit_on, "limit": 2}]);
            let made = ok(&ada
                .send(
                    "POST",
                    SEARCHES,
                    Some(
                        json!({"name": limit_on, "search_text": "", "presentation": presentation}),
                    ),
                )
                .await?)?
            .clone();
            let id = text_in(&made, "id")?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{SEARCHES}/{id}/shares"),
                    Some(json!({"kind": "project", "id": site.acme})),
                )
                .await?)?;
            Ok::<_, bb_testing::Failure>(id)
        };
        let known = share(&mut ada, "Open").await?;
        let unknown = share(&mut ada, "In the vault").await?;
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let listed = team
            .send("GET", &format!("{SEARCHES}?scope=shared"), None)
            .await?;
        let ids: Vec<&str> = ok(&listed)?["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|each| each["id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(ids, [known.as_str()], "{}", listed.body);
        // Its address answers as for a search that does not exist, and so does its board.
        let opened = team
            .send("GET", &format!("{SEARCHES}/{unknown}"), None)
            .await?;
        assert_eq!(opened.code(), "resource.not-found", "{}", opened.body);
        let as_board = team
            .send("POST", BOARD, Some(json!({"saved_search": unknown})))
            .await?;
        assert_eq!(as_board.code(), "resource.not-found", "{}", as_board.body);
        assert!(!as_board.body.to_string().contains("vault"));
        Ok(())
    })
    .await
}
