//! Pins over HTTP on SQLite and PostgreSQL (CFG-05, ADR-0050, decision 12): a person pins projects
//! and saved searches for themselves, keeps them in their own order, never sees a pin to something
//! they may no longer open, loses a pin with its target, and holds at most thirty.

use axum::http::StatusCode;
use bb_app::ports::PersonData as _;
use bb_core::id::{ProjectId, RoleId};
use bb_core::pin::PINS_MAX;
use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::access_keys_http::a_key_is_turned_away_first;
use super::dashboards_http::{DASHBOARDS, MISSING, create, share, timer};
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{Site, id_of, ok};
use super::perf_queries::Counter;
use super::removal_http::{remove, switch_off};

const PINS: &str = "/api/v1/me/pins";
const ORDER: &str = "/api/v1/me/pins/order";
const SEARCHES: &str = "/api/v1/saved-searches";

/// Pins `target` of `kind` as `browser`; the answer.
async fn pin(browser: &mut Browser, kind: &str, target: &str) -> Result<Reply, Failure> {
    browser
        .send("POST", PINS, Some(json!({"kind": kind, "target": target})))
        .await
}

/// Takes the pin away; the answer.
async fn unpin(browser: &mut Browser, kind: &str, target: &str) -> Result<Reply, Failure> {
    browser
        .send(
            "DELETE",
            &format!("{PINS}?kind={kind}&target={target}"),
            None,
        )
        .await
}

/// The pins `browser` sees, as `(kind, name)` in order.
async fn shown(browser: &mut Browser) -> Result<Vec<(String, String)>, Failure> {
    let list = browser.send("GET", PINS, None).await?;
    Ok(ok(&list)?["items"]
        .as_array()
        .ok_or("no items")?
        .iter()
        .map(|item| {
            (
                item["kind"].as_str().unwrap_or_default().to_owned(),
                item["name"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect())
}

/// Makes a saved search of the person; its ID.
async fn search(browser: &mut Browser, name: &str) -> Result<String, Failure> {
    id_of(
        &browser
            .send(
                "POST",
                SEARCHES,
                Some(json!({"name": name, "search_text": ""})),
            )
            .await?,
    )
}

fn pair(kind: &str, name: &str) -> (String, String) {
    (kind.to_owned(), name.to_owned())
}

#[tokio::test]
async fn cfg_05_a_person_pins_a_project_a_saved_search_and_a_dashboard() -> TestResult {
    each_database("cfg_05_pins", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        assert!(shown(&mut ada).await?.is_empty());
        let mine = search(&mut ada, "Open bugs").await?;
        let first = pin(&mut ada, "project", &site.acme).await?;
        assert_eq!(first.status, StatusCode::NO_CONTENT, "{}", first.body);
        pin(&mut ada, "saved-search", &mine).await?;
        // Pinning what is pinned changes nothing.
        let again = pin(&mut ada, "saved-search", &mine).await?;
        assert_eq!(again.status, StatusCode::NO_CONTENT, "{}", again.body);
        let board = id_of(&create(&mut ada, "Team board", vec![timer(0)]).await?)?;
        let pinned = pin(&mut ada, "dashboard", &board).await?;
        assert_eq!(pinned.status, StatusCode::NO_CONTENT, "{}", pinned.body);
        assert_eq!(
            shown(&mut ada).await?,
            [
                pair("project", "Acme"),
                pair("saved-search", "Open bugs"),
                pair("dashboard", "Team board")
            ]
        );
        let list = ok(&ada.send("GET", PINS, None).await?)?.clone();
        assert_eq!(list["items"][0]["id"], json!(site.acme));
        assert_eq!(list["items"][0]["short_name"], json!("ACME"));
        assert_eq!(list["items"][1]["id"], json!(mine));
        // A pin is the person's own: Bob sees none of Ada's.
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        assert!(shown(&mut bob).await?.is_empty());
        let removed = unpin(&mut ada, "project", &site.acme).await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        assert_eq!(
            shown(&mut ada).await?,
            [
                pair("saved-search", "Open bugs"),
                pair("dashboard", "Team board")
            ]
        );
        // Taking away what is not pinned changes nothing.
        let none = unpin(&mut ada, "project", &site.acme).await?;
        assert_eq!(none.status, StatusCode::NO_CONTENT, "{}", none.body);
        // Something that is not there, or that the person may not open, cannot be pinned.
        let (mut cy, _) = site.person_in_acme(StartingRole::Viewer, "cy").await?;
        let hidden = pin(&mut cy, "saved-search", &mine).await?;
        assert_eq!(hidden.status, StatusCode::NOT_FOUND, "{}", hidden.body);
        let nowhere = pin(&mut ada, "dashboard", MISSING).await?;
        assert_eq!(nowhere.status, StatusCode::NOT_FOUND, "{}", nowhere.body);
        let bad = pin(&mut ada, "saved-search", "not-an-id").await?;
        assert_eq!(bad.status, StatusCode::NOT_FOUND, "{}", bad.body);
        let unknown = pin(&mut ada, "folder", &mine).await?;
        assert!(unknown.status.is_client_error(), "{}", unknown.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_pins_keep_the_persons_order_and_move_up_and_down() -> TestResult {
    each_database("cfg_05_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let one = search(&mut ada, "One").await?;
        let two = search(&mut ada, "Two").await?;
        pin(&mut ada, "saved-search", &two).await?;
        pin(&mut ada, "project", &site.acme).await?;
        pin(&mut ada, "saved-search", &one).await?;
        assert_eq!(
            shown(&mut ada).await?,
            [
                pair("saved-search", "Two"),
                pair("project", "Acme"),
                pair("saved-search", "One")
            ]
        );
        // The whole order, as the screen sends it after *Move up* of the last pin.
        let moved = ada
            .send(
                "PUT",
                ORDER,
                Some(json!({"pins": [
                    {"kind": "saved-search", "target": two},
                    {"kind": "saved-search", "target": one},
                    {"kind": "project", "target": site.acme},
                ]})),
            )
            .await?;
        assert_eq!(moved.status, StatusCode::NO_CONTENT, "{}", moved.body);
        assert_eq!(
            shown(&mut ada).await?,
            [
                pair("saved-search", "Two"),
                pair("saved-search", "One"),
                pair("project", "Acme")
            ]
        );
        // A new pin goes to the end.
        let three = search(&mut ada, "Three").await?;
        pin(&mut ada, "saved-search", &three).await?;
        assert_eq!(shown(&mut ada).await?[3], pair("saved-search", "Three"));
        // An order that names a pin twice or one that is not pinned is refused and changes nothing.
        let twice = ada
            .send(
                "PUT",
                ORDER,
                Some(json!({"pins": [
                    {"kind": "saved-search", "target": two},
                    {"kind": "saved-search", "target": two},
                ]})),
            )
            .await?;
        assert_eq!(twice.status, StatusCode::BAD_REQUEST, "{}", twice.body);
        let stranger = ada
            .send(
                "PUT",
                ORDER,
                Some(json!({"pins": [{"kind": "project", "target": "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c"}]})),
            )
            .await?;
        assert_eq!(stranger.status, StatusCode::BAD_REQUEST, "{}", stranger.body);
        assert_eq!(shown(&mut ada).await?[0], pair("saved-search", "Two"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_a_pin_to_something_the_person_no_longer_sees_is_left_out_without_a_word()
-> TestResult {
    each_database("cfg_05_unseen", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let id = search(&mut ada, "Shared one").await?;
        ok(&ada
            .send(
                "POST",
                &format!("{SEARCHES}/{id}/shares"),
                Some(json!({"kind": "project", "id": site.acme})),
            )
            .await?)?;
        pin(&mut bob, "project", &site.acme).await?;
        pin(&mut bob, "saved-search", &id).await?;
        assert_eq!(
            shown(&mut bob).await?,
            [pair("project", "Acme"), pair("saved-search", "Shared one")]
        );
        // Ada stops sharing: the pin is not in Bob's answer, with no word and no count.
        ok(&ada
            .send(
                "DELETE",
                &format!("{SEARCHES}/{id}/shares/project/{}", site.acme),
                None,
            )
            .await?)?;
        let list = bob.send("GET", PINS, None).await?;
        assert_eq!(ok(&list)?["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            list.body
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>()),
            Some(vec!["items".to_owned()]),
            "{}",
            list.body
        );
        // It comes back when Bob may open the search again.
        ok(&ada
            .send(
                "POST",
                &format!("{SEARCHES}/{id}/shares"),
                Some(json!({"kind": "project", "id": site.acme})),
            )
            .await?)?;
        assert_eq!(shown(&mut bob).await?.len(), 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_a_pin_of_a_deleted_target_is_deleted_with_it() -> TestResult {
    each_database("cfg_05_deleted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let id = search(&mut ada, "Short lived").await?;
        ok(&ada
            .send(
                "POST",
                &format!("{SEARCHES}/{id}/shares"),
                Some(json!({"kind": "project", "id": site.acme})),
            )
            .await?)?;
        pin(&mut ada, "saved-search", &id).await?;
        pin(&mut bob, "saved-search", &id).await?;
        let deleted = ada
            .send("DELETE", &format!("{SEARCHES}/{id}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        // Taking the search back from the trash does not bring the pins back.
        ok(&ada
            .send("POST", &format!("{SEARCHES}/{id}/restore"), None)
            .await?)?;
        assert!(shown(&mut ada).await?.is_empty());
        assert!(shown(&mut bob).await?.is_empty());
        let rows = rig.storage.person_rows(bob_id, "pin", 0).await?;
        assert!(rows.rows.is_empty(), "{:?}", rows.rows);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_at_most_30_pins() -> TestResult {
    each_database("cfg_05_limit", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let mut ids = Vec::new();
        for number in 0..=PINS_MAX {
            ids.push(search(&mut ada, &format!("Search {number}")).await?);
        }
        for id in &ids[..PINS_MAX] {
            let pinned = pin(&mut ada, "saved-search", id).await?;
            assert_eq!(pinned.status, StatusCode::NO_CONTENT, "{}", pinned.body);
        }
        let over = pin(&mut ada, "saved-search", &ids[PINS_MAX]).await?;
        assert_eq!(over.status, StatusCode::CONFLICT, "{}", over.body);
        assert_eq!(over.code(), "limit.reached");
        assert_eq!(shown(&mut ada).await?.len(), PINS_MAX);
        // Pinning what is pinned is no new pin, so it does not hit the limit.
        let same = pin(&mut ada, "saved-search", &ids[0]).await?;
        assert_eq!(same.status, StatusCode::NO_CONTENT, "{}", same.body);
        // Taking one away makes room.
        unpin(&mut ada, "saved-search", &ids[0]).await?;
        let room = pin(&mut ada, "saved-search", &ids[PINS_MAX]).await?;
        assert_eq!(room.status, StatusCode::NO_CONTENT, "{}", room.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_an_access_key_reaches_no_pin_route() -> TestResult {
    each_database("cfg_05_keys", async |db| {
        let target = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";
        a_key_is_turned_away_first(
            db,
            &[
                ("GET", PINS.to_owned(), None),
                (
                    "POST",
                    PINS.to_owned(),
                    Some(json!({"kind": "project", "target": target})),
                ),
                (
                    "DELETE",
                    format!("{PINS}?kind=project&target={target}"),
                    None,
                ),
                ("PUT", ORDER.to_owned(), Some(json!({"pins": []}))),
            ],
        )
        .await
    })
    .await
}

#[tokio::test]
async fn priv_04_removal_deletes_the_persons_pins() -> TestResult {
    each_database("priv_04_pins", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let private = search(&mut bob, "Bob's own").await?;
        pin(&mut bob, "project", &site.acme).await?;
        pin(&mut bob, "saved-search", &private).await?;
        // Ada pins a project too: hers stay.
        pin(&mut ada, "project", &site.acme).await?;
        // *Download my data* holds the pins of the person.
        assert!(
            rig.storage
                .kinds_of(bob_id)
                .await?
                .iter()
                .any(|kind| kind.kind == "pin")
        );
        switch_off(&mut ada, bob_id).await?;
        let removed = remove(&mut ada, bob_id, json!({"comments": "keep"})).await?;
        assert_eq!(removed["status"], json!(204), "{removed}");
        let rows = rig.storage.person_rows(bob_id, "pin", 0).await?;
        assert!(rows.rows.is_empty(), "{:?}", rows.rows);
        assert_eq!(shown(&mut ada).await?, [pair("project", "Acme")]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_an_order_names_only_pins_the_person_sees_and_says_nothing_of_why() -> TestResult {
    each_database("cfg_05_order_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let unshared = search(&mut ada, "Unshared later").await?;
        let trashed = search(&mut ada, "Trashed later").await?;
        for id in [&unshared, &trashed] {
            ok(&ada
                .send(
                    "POST",
                    &format!("{SEARCHES}/{id}/shares"),
                    Some(json!({"kind": "project", "id": site.acme})),
                )
                .await?)?;
            pin(&mut bob, "saved-search", id).await?;
        }
        let order = |id: &str| json!({"pins": [{"kind": "saved-search", "target": id}]});
        let seen = bob.send("PUT", ORDER, Some(order(&unshared))).await?;
        assert_eq!(seen.status, StatusCode::NO_CONTENT, "{}", seen.body);
        ok(&ada
            .send(
                "DELETE",
                &format!("{SEARCHES}/{unshared}/shares/project/{}", site.acme),
                None,
            )
            .await?)?;
        ada.send("DELETE", &format!("{SEARCHES}/{trashed}"), None)
            .await?;
        // Unshared, trashed and never pinned are one answer.
        let mut answers = Vec::new();
        for id in [
            unshared.as_str(),
            trashed.as_str(),
            "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c",
        ] {
            let refused = bob.send("PUT", ORDER, Some(order(id))).await?;
            answers.push((refused.status, refused.code().to_owned()));
        }
        assert_eq!(answers[0].0, StatusCode::BAD_REQUEST, "{answers:?}");
        assert!(
            answers.iter().all(|answer| *answer == answers[0]),
            "{answers:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_a_person_who_loses_sight_of_a_project_loses_the_pin_until_it_returns() -> TestResult
{
    each_database("cfg_05_sight", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let role: RoleId = site
            .role_id(StartingRole::TeamMember)
            .parse()
            .map_err(|_| "a role ID")?;
        let (mut bob, member) = site.person_holding(role, "bob", &site.acme).await?;
        // Bob sees LAB through a role of his; taking the row away takes it from him, giving it back returns it.
        let made = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "LAB", "name": "Lab"})),
            )
            .await?;
        let lab = id_of(&made)?;
        let lab_id: ProjectId = lab.parse().map_err(|_| "a project ID")?;
        let row = site
            .org
            .give(&member, role, lab_id)
            .await
            .map_err(|error| error.to_string())?;
        pin(&mut bob, "project", &lab).await?;
        assert_eq!(shown(&mut bob).await?, [pair("project", "Lab")]);
        let removed = ada
            .send("DELETE", &format!("/api/v1/assignments/{}", row.id), None)
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        assert!(shown(&mut bob).await?.is_empty());
        let again = pin(&mut bob, "project", &lab).await?;
        assert_eq!(again.status, StatusCode::NOT_FOUND, "{}", again.body);
        site.org
            .give(&member, role, lab_id)
            .await
            .map_err(|error| error.to_string())?;
        assert_eq!(shown(&mut bob).await?, [pair("project", "Lab")]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_two_pins_at_the_same_moment_cannot_both_take_the_last_place() -> TestResult {
    each_database("cfg_05_limit_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let mut again = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        let mut ids = Vec::new();
        for number in 0..=PINS_MAX {
            ids.push(search(&mut ada, &format!("Search {number}")).await?);
        }
        // One place is left.
        for id in &ids[..PINS_MAX - 1] {
            pin(&mut ada, "saved-search", id).await?;
        }
        let (first, second) = tokio::join!(
            pin(&mut ada, "saved-search", &ids[PINS_MAX - 1]),
            pin(&mut again, "saved-search", &ids[PINS_MAX])
        );
        let (first, second) = (first?, second?);
        let mut codes = [first.status, second.status];
        codes.sort();
        assert_eq!(codes, [StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
        assert_eq!(shown(&mut ada).await?.len(), PINS_MAX);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_pinning_a_search_that_is_not_there_runs_the_reads_of_one_that_is_not_the_persons()
-> TestResult {
    each_database("cfg_05_same_reads", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let private = search(&mut ada, "Ada only").await?;
        let nowhere = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";
        let mut counter = Counter::on(&rig.database);
        // A first look warms what is kept between requests; it is not counted.
        pin(&mut bob, "saved-search", nowhere).await?;
        counter.take_logged();
        let missing = pin(&mut bob, "saved-search", nowhere).await?;
        let (on_missing, missing_log) = counter.take_logged();
        let hidden = pin(&mut bob, "saved-search", &private).await?;
        let (on_hidden, hidden_log) = counter.take_logged();
        assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
        assert_eq!(hidden.status, StatusCode::NOT_FOUND, "{}", hidden.body);
        assert!(on_missing > 0, "the counter sees nothing");
        assert_eq!(
            on_missing, on_hidden,
            "the same reads, whether the search is there or not: {missing_log:?} against {hidden_log:?}"
        );
        Ok(())
    })
    .await
}

/// A dashboard of `maker` shared with ACME, so a team member sees it; its ID.
async fn shared_board(site: &Site<'_>, maker: &mut Browser, name: &str) -> Result<String, Failure> {
    let id = id_of(&create(maker, name, vec![timer(0)]).await?)?;
    ok(&share(maker, &id, ("project", &site.acme)).await?)?;
    Ok(id)
}

#[tokio::test]
async fn cfg_05_a_pin_to_a_dashboard_follows_who_may_open_it_and_goes_with_its_trash() -> TestResult
{
    each_database("cfg_05_dashboard_pins", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let unshared = shared_board(&site, &mut ada, "Left out later").await?;
        let trashed = shared_board(&site, &mut ada, "Trashed later").await?;
        for id in [&unshared, &trashed] {
            pin(&mut bob, "dashboard", id).await?;
        }
        assert_eq!(shown(&mut bob).await?.len(), 2);
        // Ada stops sharing one: it is left out of Bob's answer, and a new pin of it is not found.
        let stopped = ada
            .send(
                "DELETE",
                &format!("{DASHBOARDS}/{unshared}/shares/project/{}", site.acme),
                None,
            )
            .await?;
        assert!(stopped.status.is_success(), "{}", stopped.body);
        assert_eq!(shown(&mut bob).await?, [pair("dashboard", "Trashed later")]);
        let again = pin(&mut bob, "dashboard", &unshared).await?;
        assert_eq!(again.status, StatusCode::NOT_FOUND, "{}", again.body);
        // An order names only what Bob sees: the unshared dashboard is refused like one never pinned.
        let order = |id: &str| json!({"pins": [{"kind": "dashboard", "target": id}]});
        let hidden = bob.send("PUT", ORDER, Some(order(&unshared))).await?;
        let never = bob.send("PUT", ORDER, Some(order(MISSING))).await?;
        assert_eq!(hidden.status, StatusCode::BAD_REQUEST, "{}", hidden.body);
        assert_eq!((hidden.status, hidden.code()), (never.status, never.code()));
        // The trash takes the pin with the dashboard, and taking it out does not bring the pin back.
        let deleted = ada
            .send("DELETE", &format!("{DASHBOARDS}/{trashed}"), None)
            .await?;
        assert!(deleted.status.is_success(), "{}", deleted.body);
        assert!(shown(&mut bob).await?.is_empty());
        let rows = rig.storage.person_rows(bob_id, "pin", 0).await?;
        // The pin of the unshared dashboard is still stored (it comes back with the share); the trashed one is gone.
        assert_eq!(rows.rows.len(), 1, "{:?}", rows.rows);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_pinning_a_dashboard_that_is_not_there_runs_the_reads_of_one_that_is_not_the_persons()
-> TestResult {
    each_database("cfg_05_same_reads_dashboard", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let private = id_of(&create(&mut ada, "Ada only", vec![timer(0)]).await?)?;
        let mut counter = Counter::on(&rig.database);
        // A first look warms what is kept between requests; it is not counted.
        pin(&mut bob, "dashboard", MISSING).await?;
        counter.take_logged();
        let missing = pin(&mut bob, "dashboard", MISSING).await?;
        let (on_missing, missing_log) = counter.take_logged();
        let hidden = pin(&mut bob, "dashboard", &private).await?;
        let (on_hidden, hidden_log) = counter.take_logged();
        assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
        assert_eq!(hidden.status, StatusCode::NOT_FOUND, "{}", hidden.body);
        assert!(on_missing > 0, "the counter sees nothing");
        assert_eq!(
            on_missing, on_hidden,
            "the same reads, whether the dashboard is there or not: {missing_log:?} against {hidden_log:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_05_the_home_of_a_person_cannot_be_pinned() -> TestResult {
    each_database("cfg_05_home", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let home = ada.send("GET", &format!("{DASHBOARDS}/home"), None).await?;
        let home = id_of(&home)?;
        let refused = pin(&mut ada, "dashboard", &home).await?;
        assert_eq!(refused.status, StatusCode::NOT_FOUND, "{}", refused.body);
        assert!(shown(&mut ada).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_removal_deletes_the_pins_to_the_private_dashboards_of_the_removed() -> TestResult {
    each_database("priv_04_pins_to_dashboards", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let (mut cy, cy_id) = site.person(StartingRole::TeamMember, "cy").await?;
        // Bob shares a dashboard, Cy pins it, Bob stops sharing it; it is private again.
        let board = id_of(&create(&mut bob, "Bob's board", vec![timer(0)]).await?)?;
        ok(&share(&mut bob, &board, ("project", &site.acme)).await?)?;
        pin(&mut cy, "dashboard", &board).await?;
        assert_eq!(shown(&mut cy).await?, [pair("dashboard", "Bob's board")]);
        let stopped = bob
            .send(
                "DELETE",
                &format!("{DASHBOARDS}/{board}/shares/project/{}", site.acme),
                None,
            )
            .await?;
        assert!(stopped.status.is_success(), "{}", stopped.body);
        assert!(shown(&mut cy).await?.is_empty());
        // The pin row is still there until the dashboard goes with its maker.
        let before = rig.storage.person_rows(cy_id, "pin", 0).await?;
        assert_eq!(before.rows.len(), 1, "{:?}", before.rows);
        switch_off(&mut ada, bob_id).await?;
        let removed = remove(&mut ada, bob_id, json!({"comments": "keep"})).await?;
        assert_eq!(removed["status"], json!(204), "{removed}");
        let after = rig.storage.person_rows(cy_id, "pin", 0).await?;
        assert!(after.rows.is_empty(), "{:?}", after.rows);
        Ok(())
    })
    .await
}
