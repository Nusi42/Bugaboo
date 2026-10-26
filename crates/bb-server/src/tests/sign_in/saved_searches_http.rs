//! Saved searches over HTTP (QRY-06, CFG-02, ADR-0034, SEC-06): a search is private when it is
//! made, shared with a project it reaches the people who see it, a search that is not for a person
//! is not found, as one that does not exist, and a page of forty runs as many statements as a page
//! of five. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_of, ok};
use super::perf_queries::{assert_as_many_statements, five_and_forty};

const SEARCHES: &str = "/api/v1/saved-searches";
/// The most statements a page of saved searches may run, however many rows there are (a ceiling
/// that a change to the code has to notice; the exact numbers are compared apart). Most of them are
/// the session and the rights of the person, which every read pays.
const AT_MOST: usize = 30;

async fn save(browser: &mut Browser, name: &str) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            SEARCHES,
            Some(json!({
                "name": name,
                "search_text": "status is open",
                "presentation": {
                    "view": "list",
                    "columns": [{"field": "title", "width": 300}, {"field": "status", "width": null}],
                    "spacing": "compact",
                    "group_by": "statusGroup"
                }
            })),
        )
        .await
}

async fn share_with_acme(
    browser: &mut Browser,
    search: &str,
    acme: &str,
) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{SEARCHES}/{search}/shares"),
            Some(json!({"kind": "project", "id": acme})),
        )
        .await
}

fn names(list: &Reply) -> Vec<String> {
    list.body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["name"].as_str().map(str::to_owned))
        .collect()
}

/// Ada's search "Open bugs", shared with ACME, and Bob, a team member who sees it: the ID.
async fn ada_shares_with_acme(site: &Site<'_>, ada: &mut Browser) -> Result<String, Failure> {
    let id = id_of(&save(ada, "Open bugs").await?)?;
    ok(&share_with_acme(ada, &id, &site.acme).await?)?;
    Ok(id)
}

#[tokio::test]
async fn qry_06_a_saved_search_is_private_and_found_by_its_address_only_where_it_is_for_the_person()
-> TestResult {
    each_database("qry_06_saved_search_private", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let made = save(&mut ada, "Open bugs").await?;
        let id = id_of(&made)?;
        assert_eq!(made.body["mine"], true);
        assert_eq!(made.body["presentation"]["spacing"], "compact");
        assert_eq!(made.body["presentation"]["columns"][0]["width"], 300);
        assert_eq!(made.body["shared_with"], json!([]));
        // Private: it is not found for Bob, in the same words as one that is not there.
        let hidden = bob.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        let missing = bob
            .send(
                "GET",
                &format!("{SEARCHES}/01a3185c-5000-73b2-8f37-20757db0778e"),
                None,
            )
            .await?;
        assert_eq!(hidden.status, StatusCode::NOT_FOUND);
        assert_eq!(
            (hidden.status, hidden.code()),
            (missing.status, missing.code())
        );
        assert_eq!(hidden.body["code"], "resource.not-found");
        // Shared with ACME, it is listed for Bob and opens, and Bob is told who made it and nothing
        // of whom else it is shared with.
        let shared = share_with_acme(&mut ada, &id, &site.acme).await?;
        assert_eq!(ok(&shared)?["shared_with"][0]["kind"], "project");
        assert_eq!(shared.body["shared_with"][0]["short_name"], "ACME");
        let path = format!("{SEARCHES}?scope=shared&fields=name,search_text,maker");
        let listed = bob.send("GET", &path, None).await?;
        assert_eq!(names(&listed), ["Open bugs"]);
        assert_eq!(listed.body["items"][0]["search_text"], "status is open");
        let opened = bob.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        assert_eq!(ok(&opened)?["mine"], false);
        assert_eq!(opened.body["shared_with"], json!([]));
        let own = bob.send("GET", SEARCHES, None).await?;
        assert!(names(&own).is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_only_the_maker_changes_a_saved_search_and_a_receiver_copies_it() -> TestResult {
    each_database("qry_06_saved_search_maker", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let id = ada_shares_with_acme(&site, &mut ada).await?;
        let renamed = bob
            .send(
                "PATCH",
                &format!("{SEARCHES}/{id}"),
                Some(json!({"version": 1, "name": "Mine now"})),
            )
            .await?;
        assert_eq!(renamed.code(), "access.forbidden");
        let copy = bob
            .send(
                "POST",
                &format!("{SEARCHES}/{id}/copy"),
                Some(json!({"name": "My open bugs"})),
            )
            .await?;
        assert_eq!(ok(&copy)?["mine"], true);
        assert_eq!(copy.body["shared_with"], json!([]));
        let theirs = ada
            .send("GET", &format!("{SEARCHES}/{}", id_of(&copy)?), None)
            .await?;
        assert_eq!(theirs.status, StatusCode::NOT_FOUND);
        // Ada changes it on the version she looked at, and not on an old one.
        let changed = ada
            .send(
                "PATCH",
                &format!("{SEARCHES}/{id}"),
                Some(json!({"version": 1, "name": "Bugs", "presentation": null})),
            )
            .await?;
        assert_eq!(ok(&changed)?["version"], 2);
        let stale = ada
            .send(
                "PATCH",
                &format!("{SEARCHES}/{id}"),
                Some(json!({"version": 1, "name": "Again"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_stopping_a_share_and_the_trash_take_a_saved_search_from_its_receivers() -> TestResult
{
    each_database("qry_06_saved_search_stops", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let id = ada_shares_with_acme(&site, &mut ada).await?;
        let shared = format!("{SEARCHES}?scope=shared");
        assert_eq!(names(&bob.send("GET", &shared, None).await?), ["Open bugs"]);
        let share = format!("{SEARCHES}/{id}/shares/project/{}", site.acme);
        let stopped = ada.send("DELETE", &share, None).await?;
        assert_eq!(ok(&stopped)?["shared_with"], json!([]));
        assert!(names(&bob.send("GET", &shared, None).await?).is_empty());
        // The trash takes it away and *Undo* brings it back, still shared.
        share_with_acme(&mut ada, &id, &site.acme).await?;
        let deleted = ada
            .send("DELETE", &format!("{SEARCHES}/{id}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let gone = bob.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);
        let back = ada
            .send("POST", &format!("{SEARCHES}/{id}/restore"), None)
            .await?;
        assert_eq!(ok(&back)?["shared_with"][0]["kind"], "project");
        let seen = bob.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        assert_eq!(seen.status, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_the_share_window_lists_what_the_person_may_share_with_and_the_project_has_its_list()
-> TestResult {
    each_database("qry_06_share_choices_and_project_list", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let projects = bob
            .send(
                "GET",
                &format!("{SEARCHES}/share-choices?kind=project"),
                None,
            )
            .await?;
        assert_eq!(names(&projects), ["Acme"]);
        let groups = bob
            .send("GET", &format!("{SEARCHES}/share-choices?kind=group"), None)
            .await?;
        assert!(names(&groups).is_empty());
        let made = id_of(&save(&mut bob, "ACME list").await?)?;
        // Nothing yet: the built-in default, and a search that is not shared cannot be the list.
        let list = ada
            .send("GET", &format!("{PROJECTS}/{}/list", site.acme), None)
            .await?;
        assert_eq!(ok(&list)?["search"], Value::Null);
        let early = ada
            .send(
                "PUT",
                &format!("{PROJECTS}/{}/list", site.acme),
                Some(json!({"search": made})),
            )
            .await?;
        assert_eq!(early.code(), "resource.not-found");
        share_with_acme(&mut bob, &made, &site.acme).await?;
        // A team member has no *Change project settings*; the administrator has.
        let denied = bob
            .send(
                "PUT",
                &format!("{PROJECTS}/{}/list", site.acme),
                Some(json!({"search": made})),
            )
            .await?;
        assert_eq!(denied.code(), "access.forbidden");
        let chosen = ada
            .send(
                "PUT",
                &format!("{PROJECTS}/ACME/list"),
                Some(json!({"search": made})),
            )
            .await?;
        assert_eq!(ok(&chosen)?["search"]["name"], "ACME list");
        let seen = bob
            .send("GET", &format!("{PROJECTS}/ACME/list"), None)
            .await?;
        assert_eq!(ok(&seen)?["search"]["id"], made);
        let cleared = ada
            .send(
                "PUT",
                &format!("{PROJECTS}/ACME/list"),
                Some(json!({"search": null})),
            )
            .await?;
        assert_eq!(ok(&cleared)?["search"], Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_saved_searches_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_saved_searches_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let (mut cleo, _) = site
            .person_called(StartingRole::Viewer, "cleo", "Cleo Cook")
            .await?;
        for number in 0..40 {
            // Two makers, so a page of the shared searches names more than one person.
            let maker = if number % 2 == 0 { &mut ada } else { &mut bob };
            let id = id_of(&save(maker, &format!("Search {number}")).await?)?;
            ok(&share_with_acme(maker, &id, &site.acme).await?)?;
        }
        // Ada made twenty of them; twenty more that she keeps to herself make forty of her own.
        for number in 40..60 {
            ok(&save(&mut ada, &format!("Private {number}")).await?)?;
        }
        for (who, scope) in [(&mut ada, "mine"), (&mut cleo, "shared")] {
            let (on_five, on_forty) = five_and_forty(who, &rig.database, |limit| {
                format!("{SEARCHES}?scope={scope}&limit={limit}&fields=name,maker,shared_with")
            })
            .await?;
            assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page");
        }
        Ok(())
    })
    .await
}
