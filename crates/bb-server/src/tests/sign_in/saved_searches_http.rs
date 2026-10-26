//! Saved searches over HTTP (QRY-06, CFG-02, ADR-0034, SEC-06): a search is private when it is
//! made, shared with a project it reaches the people who see it, a search that is not for a person
//! is not found, as one that does not exist, and a page of forty runs as many statements as a page
//! of five. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::access_keys_http::a_key_is_turned_away_first;
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_of, ok};
use super::perf_queries::{Counter, assert_as_many_statements, five_and_forty};

const SEARCHES: &str = "/api/v1/saved-searches";
/// The most statements a page of saved searches may run, however many rows there are (a ceiling
/// that a change to the code has to notice; the exact numbers are compared apart). Most of them are
/// the session, the rights of the person and the fields they may read (checking a shared search
/// needs them).
const AT_MOST: usize = 40;
/// The same for a guest, who sees the makers by their role only: the roles of all the makers of a
/// page are read once (not for each maker), which costs the reads of that one policy (measured: 43).
const AT_MOST_FOR_A_GUEST: usize = 43;

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
                    "columns": [
                        {"field": "title", "width": 300},
                        {"field": "status", "width": null}
                    ],
                    "spacing": "compact",
                    "group_by": "statusGroup"
                }
            })),
        )
        .await
}

async fn share_with_project(
    browser: &mut Browser,
    search: &str,
    project: &str,
) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{SEARCHES}/{search}/shares"),
            Some(json!({"kind": "project", "id": project})),
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
    ok(&share_with_project(ada, &id, &site.acme).await?)?;
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
        let shared = share_with_project(&mut ada, &id, &site.acme).await?;
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
async fn qry_06_stopping_a_share_and_the_trash_take_a_search_from_its_receivers() -> TestResult {
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
        share_with_project(&mut ada, &id, &site.acme).await?;
        let deleted = ada
            .send("DELETE", &format!("{SEARCHES}/{id}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let gone = bob.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);
        let back = ada
            .send("POST", &format!("{SEARCHES}/{id}/restore"), None)
            .await?;
        assert_eq!(ok(&back)?["search"]["shared_with"][0]["kind"], "project");
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
        share_with_project(&mut bob, &made, &site.acme).await?;
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
            ok(&share_with_project(maker, &id, &site.acme).await?)?;
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

#[tokio::test]
async fn qry_06_deleting_a_group_takes_the_searches_shared_with_it_along() -> TestResult {
    each_database("qry_06_group_deleted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let group = id_of(
            &ada.send(
                "POST",
                "/api/v1/groups",
                Some(json!({"name": "Night shift"})),
            )
            .await?,
        )?;
        let id = id_of(&save(&mut ada, "For the night").await?)?;
        let shared = ada
            .send(
                "POST",
                &format!("{SEARCHES}/{id}/shares"),
                Some(json!({"kind": "group", "id": group})),
            )
            .await?;
        assert_eq!(ok(&shared)?["shared_with"][0]["name"], "Night shift");
        let deleted = ada
            .send("DELETE", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let after = ada.send("GET", &format!("{SEARCHES}/{id}"), None).await?;
        assert_eq!(ok(&after)?["shared_with"], json!([]));
        Ok(())
    })
    .await
}

/// Ada saves a search with this text and shares it with ACME; its ID.
async fn ada_shares(
    site: &Site<'_>,
    ada: &mut Browser,
    name: &str,
    search_text: &str,
) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            SEARCHES,
            Some(json!({"name": name, "search_text": search_text})),
        )
        .await?;
    let id = id_of(&made)?;
    ok(&share_with_project(ada, &id, &site.acme).await?)?;
    Ok(id)
}

#[tokio::test]
async fn qry_06_a_search_that_does_not_check_for_the_receiver_is_not_there_for_any_call()
-> TestResult {
    each_database("qry_06_saved_search_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A guest sees people by their role only, so cannot search by who an issue is assigned to.
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        let (mut cleo, _) = site
            .person_called(StartingRole::TeamMember, "cleo", "Cleo Cook")
            .await?;
        let hidden = ada_shares(&site, &mut ada, "Hidden", "assignedTo is \"Ada Admin\"").await?;
        let seen = ada_shares(&site, &mut ada, "Seen", "").await?;
        let missing = "01a3185c-5000-73b2-8f37-20757db0778e";
        // The list holds the next search, not a gap; the address, the delete and the restore answer
        // as they do for a search that is not there.
        let page = dan
            .send("GET", &format!("{SEARCHES}?scope=shared&limit=1"), None)
            .await?;
        assert_eq!(names(&page), ["Seen"]);
        assert_eq!(page.body["items"][0]["id"], seen.as_str());
        for (method, suffix) in [
            ("GET", ""),
            ("DELETE", ""),
            ("POST", "/restore"),
            ("POST", "/copy"),
        ] {
            let body = (method == "POST" && suffix == "/copy").then(|| json!({}));
            let there = dan
                .send(
                    method,
                    &format!("{SEARCHES}/{hidden}{suffix}"),
                    body.clone(),
                )
                .await?;
            let nothing = dan
                .send(method, &format!("{SEARCHES}/{missing}{suffix}"), body)
                .await?;
            assert_eq!(there.status, StatusCode::NOT_FOUND, "{method}{suffix}");
            assert_eq!(
                (there.status, there.code()),
                (nothing.status, nothing.code())
            );
        }
        // Nothing of it was deleted: somebody who reads it cleanly still has it.
        let kept = cleo
            .send("GET", &format!("{SEARCHES}/{hidden}"), None)
            .await?;
        assert_eq!(kept.status, StatusCode::OK);
        // An administrator takes a private search out of the trash and learns nothing of it.
        let private = id_of(&save(&mut cleo, "Private of Cleo").await?)?;
        let gone = cleo
            .send("DELETE", &format!("{SEARCHES}/{private}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
        let back = ada
            .send("POST", &format!("{SEARCHES}/{private}/restore"), None)
            .await?;
        assert_eq!(ok(&back)?["search"], Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_many_searches_that_are_skipped_cost_no_more_statements_than_none() -> TestResult {
    each_database("perf_05_saved_searches_skipped", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        // Five that Dan reads; then forty that he cannot read follow them, so he looks at all of
        // them to know that the page has no sixth.
        for number in 0..5 {
            ada_shares(&site, &mut ada, &format!("Seen {number}"), "").await?;
        }
        let path = format!("{SEARCHES}?scope=shared&limit=5&fields=name,maker");
        let mut counter = Counter::on(&rig.database);
        ok(&dan.send("GET", &path, None).await?)?;
        counter.take();
        assert_eq!(names(&dan.send("GET", &path, None).await?).len(), 5);
        let on_none = counter.take();
        for number in 0..40 {
            let name = format!("Hidden {number}");
            ada_shares(&site, &mut ada, &name, "assignedTo is \"Ada Admin\"").await?;
        }
        counter.take();
        let skipping = dan.send("GET", &path, None).await?;
        assert_eq!(names(&skipping).len(), 5);
        assert_as_many_statements(
            on_none,
            counter.take(),
            AT_MOST_FOR_A_GUEST,
            "skipped searches",
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_two_saves_at_the_same_moment_cannot_both_take_the_last_place() -> TestResult {
    each_database("qry_06_saved_search_limit_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let mut again = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        // One place is left.
        for number in 0..499 {
            ok(&save(&mut ada, &format!("Search {number}")).await?)?;
        }
        let (first, second) = tokio::join!(save(&mut ada, "One"), save(&mut again, "Two"));
        let (first, second) = (first?, second?);
        let mut codes = [first.status, second.status];
        codes.sort();
        assert_eq!(codes, [StatusCode::OK, StatusCode::CONFLICT]);
        let refused = if first.status == StatusCode::OK {
            &second
        } else {
            &first
        };
        assert_eq!(refused.code(), "limit.reached");
        let all = ada
            .send("GET", &format!("{SEARCHES}?limit=100"), None)
            .await?;
        assert_eq!(all.status, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_the_shared_list_goes_on_past_a_walk_of_hidden_searches_on_both_databases()
-> TestResult {
    each_database("qry_06_saved_search_offset", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        // A hundred and five that Dan cannot read (more than one read of the store, so the offset
        // is used), then three that he can; he asks for two at a time, and the
        // cursor goes on by the last one shown.
        for number in 0..105 {
            let name = format!("Hidden {number}");
            ada_shares(&site, &mut ada, &name, "assignedTo is \"Ada Admin\"").await?;
        }
        for number in 0..3 {
            ada_shares(&site, &mut ada, &format!("Seen {number}"), "").await?;
        }
        let first = dan
            .send("GET", &format!("{SEARCHES}?scope=shared&limit=2"), None)
            .await?;
        assert_eq!(names(&first).len(), 2);
        let cursor = first.body["next_cursor"].as_str().ok_or("no cursor")?;
        let second = dan
            .send(
                "GET",
                &format!("{SEARCHES}?scope=shared&limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        // Two and one, none twice: the three that he reads, in whatever order they were made.
        let mut all = names(&first);
        all.extend(names(&second));
        all.sort();
        assert_eq!(all, ["Seen 0", "Seen 1", "Seen 2"]);
        assert_eq!(names(&second).len(), 1);
        assert_eq!(second.body["next_cursor"], Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_a_cursor_with_a_skip_that_does_not_fit_answers_request_bad_cursor() -> TestResult {
    each_database("qry_06_saved_search_forged_cursor", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        for path in [
            format!("{SEARCHES}?scope=shared&cursor=%2B9223372036854776000"),
            format!("{SEARCHES}?scope=shared&cursor=%2B499"),
            "/api/v1/access-keys/project-choices?cursor=%2B9223372036854776000".to_owned(),
        ] {
            let reply = ada.send("GET", &path, None).await?;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{path}");
            assert_eq!(reply.code(), "request.bad-cursor", "{path}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_forty_different_makers_cost_no_more_statements_than_five_for_a_guest() -> TestResult
{
    each_database("perf_05_saved_searches_makers", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut dan, _) = site
            .person_called(StartingRole::Guest, "dan", "Dan Dyer")
            .await?;
        // Forty people, each with a search of their own shared with ACME or with a second project.
        // A guest sees people by their role only, so each maker is shown by a role label.
        let beta = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?;
        let beta = id_of(&beta)?;
        for number in 0..40 {
            let (mut maker, _) = site
                .person_called(
                    StartingRole::TeamMember,
                    &format!("maker{number}"),
                    &format!("Maker {number}"),
                )
                .await?;
            let made = save(&mut maker, &format!("Search {number}")).await?;
            let project = if number % 2 == 0 { &site.acme } else { &beta };
            ok(&share_with_project(&mut maker, &id_of(&made)?, project).await?)?;
        }
        let (on_five, on_forty) = five_and_forty(&mut dan, &rig.database, |limit| {
            format!("{SEARCHES}?scope=shared&limit={limit}&fields=name,maker")
        })
        .await?;
        // The statements are the same because the labels are read at once, not because nothing is
        // read: a guest sees every maker as the role of a team member, with no name and no ID.
        let page = dan
            .send(
                "GET",
                &format!("{SEARCHES}?scope=shared&limit=40&fields=maker"),
                None,
            )
            .await?;
        for item in page.body["items"].as_array().ok_or("no items")? {
            assert_eq!(
                item["maker"],
                json!({"shown": "role", "role": "Team member"})
            );
        }
        assert_as_many_statements(on_five, on_forty, AT_MOST_FOR_A_GUEST, "more makers");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_is_turned_away_from_saved_searches_before_its_address_or_body_is_read()
-> TestResult {
    each_database("auth_07_saved_searches_key_first", async |db| {
        a_key_is_turned_away_first(
            db,
            &[
                ("POST", SEARCHES.to_owned(), Some(json!({}))),
                ("GET", format!("{SEARCHES}/not-an-id"), None),
                ("PATCH", format!("{SEARCHES}/not-an-id"), Some(json!([]))),
                ("DELETE", format!("{SEARCHES}/not-an-id"), None),
                (
                    "POST",
                    format!("{SEARCHES}/not-an-id/shares"),
                    Some(json!({"kind": "planet", "id": "x"})),
                ),
                (
                    "DELETE",
                    format!("{SEARCHES}/not-an-id/shares/planet/x"),
                    None,
                ),
                ("POST", format!("{SEARCHES}/not-an-id/restore"), None),
                (
                    "POST",
                    format!("{SEARCHES}/not-an-id/copy"),
                    Some(json!([])),
                ),
                ("GET", format!("{SEARCHES}?limit=many"), None),
                ("GET", format!("{SEARCHES}/share-choices"), None),
                (
                    "PUT",
                    "/api/v1/projects/not-a-project/list".to_owned(),
                    Some(json!({"search": "x"})),
                ),
            ],
        )
        .await
    })
    .await
}
