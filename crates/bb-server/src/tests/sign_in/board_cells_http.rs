//! The cells of a board over HTTP (PLAN-06, ADR-0034, amendment of 2026-11-24): a status key two
//! workflows name differently is counted once, *Show more* reads the next cards of one cell by its
//! cursor, and a saved board shows the options it keeps. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{Site, ok, text_in};
use super::views_site::{
    BOARD, acme_board, board, cards, column_keys, issue_in, keys_in, own_workflow, project,
};

/// The counts of the columns of a board, added up.
fn counted(read: &Value) -> u64 {
    read["columns"]
        .as_array()
        .map(|columns| {
            columns
                .iter()
                .filter_map(|column| column["count"].as_u64())
                .sum()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn plan_06_a_status_key_two_workflows_name_differently_is_counted_once() -> TestResult {
    each_database("plan_06_shared_key", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // ACME calls the key `open` *To do*; WEB keeps *Simple*, where it is *Open*.
        own_workflow(
            &mut ada,
            &site.acme,
            (
                json!([
                    {"key": "open", "name": "To do", "status_group": "not-started"},
                    {"key": "done", "name": "Done", "status_group": "done"},
                ]),
                json!([
                    {"from": "open", "to": "done", "button": "Finish"},
                    {"from": "done", "to": "open", "button": "Reopen"},
                ]),
            ),
        )
        .await?;
        let web = project(&mut ada, "WEB").await?;
        let in_acme = issue_in(&mut ada, (&site.acme, &site.kind), "Acme", json!({})).await?;
        let in_web = issue_in(&mut ada, (&web, &site.kind), "Web", json!({})).await?;
        let read = board(&mut ada, json!({"search_text": ""})).await?;
        // Two issues, two cards, and the columns count two: the key stands in one column.
        assert_eq!(counted(&read), 2, "{read}");
        let shown: Vec<&str> = cards(&read)
            .iter()
            .filter_map(|card| card["issue"]["readable_id"].as_str())
            .collect();
        assert_eq!(shown.len(), 2, "{read}");
        assert!(shown.contains(&in_acme.as_str()) && shown.contains(&in_web.as_str()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_show_more_reads_the_next_cards_of_one_cell_by_its_cursor() -> TestResult {
    each_database("plan_06_show_more", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut made = Vec::new();
        for title in ["A", "B", "C"] {
            made.push(issue_in(&mut ada, (&site.acme, &site.kind), title, json!({})).await?);
        }
        let mut ask = json!({"search_text": "sort by title", "project": site.acme, "limit": 1});
        let first = board(&mut ada, ask.clone()).await?;
        let mut seen = keys_in(&first, "Open")?;
        let mut cursor = first["cells"][0]["next_cursor"].clone();
        while let Some(more) = cursor.as_str() {
            ask["more"] = json!(more);
            let next = board(&mut ada, ask.clone()).await?;
            seen.extend(keys_in(&next, "Open")?);
            assert_eq!(next["cells"].as_array().map(Vec::len), Some(1), "{next}");
            cursor = next["cells"][0]["next_cursor"].clone();
        }
        assert_eq!(seen, made, "{first}");
        // A cursor of another board does not open.
        ask["search_text"] = json!("");
        ask["more"] = json!(
            first["cells"][0]["next_cursor"]
                .as_str()
                .ok_or("no cursor")?
        );
        let refused = ada.send("POST", BOARD, Some(ask)).await?;
        assert_eq!(refused.code(), "request.bad-cursor", "{}", refused.body);
        // The number of cards is checked once, by the use case.
        let too_many = ada
            .send("POST", BOARD, Some(json!({"search_text": "", "limit": 51})))
            .await?;
        assert_eq!(too_many.code(), "request.invalid", "{}", too_many.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_saved_board_shows_the_options_it_keeps() -> TestResult {
    each_database("plan_06_saved_options", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        issue_in(&mut ada, (&site.acme, &site.kind), "One", json!({})).await?;
        let made = ok(&ada
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({
                    "name": "By group",
                    "search_text": "",
                    "presentation": {
                        "view": "board",
                        "columns": [],
                        "group_by": null,
                        "board": {"columns": "status group", "rows": null, "card_limits": []},
                    },
                })),
            )
            .await?)?
        .clone();
        let id = text_in(&made, "id")?;
        let kept = board(&mut ada, json!({"saved_search": id})).await?;
        assert_eq!(
            column_keys(&kept),
            ["Not started", "In progress", "Done"],
            "{kept}"
        );
        // Options in the request win over the kept ones.
        let mut asked = acme_board(&site, &json!({}));
        asked["saved_search"] = json!(id);
        asked
            .as_object_mut()
            .ok_or("no body")?
            .remove("search_text");
        let named = board(&mut ada, asked).await?;
        assert!(column_keys(&named).contains(&"Open".to_owned()), "{named}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_an_issue_of_two_people_shown_alike_is_one_card_in_their_row() -> TestResult {
    each_database("plan_06_alike", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, lea) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let (_, ben) = site
            .person_called(StartingRole::TeamMember, "ben", "Ben Berg")
            .await?;
        let both = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Pair work",
            json!({"assigned_to": [lea.to_string(), ben.to_string()]}),
        )
        .await?;
        // A Contributor sees both as a Team member: one row, the issue once in it.
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
        let read = board(
            &mut contributor,
            acme_board(&site, &json!({"rows": "assigned to"})),
        )
        .await?;
        assert_eq!(read["rows"].as_array().map(Vec::len), Some(1), "{read}");
        assert_eq!(read["rows"][0]["issues"], 1, "{read}");
        assert_eq!(read["cells"].as_array().map(Vec::len), Some(1), "{read}");
        assert_eq!(read["cells"][0]["count"], 1);
        assert_eq!(read["cells"][0]["next_cursor"], Value::Null);
        assert_eq!(keys_in(&read, "Open")?, std::slice::from_ref(&both));
        // Ada sees them by name: two rows, the issue in each.
        let named = board(&mut ada, acme_board(&site, &json!({"rows": "assigned to"}))).await?;
        assert_eq!(named["rows"].as_array().map(Vec::len), Some(2), "{named}");
        assert_eq!(keys_in(&named, "Open")?, [both.clone(), both]);
        Ok(())
    })
    .await
}
