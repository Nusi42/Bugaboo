//! The project routes over HTTP (PRJ-01, PRJ-02, PRJ-03): the answers a browser gets, with their
//! codes, and what a person who may not see a project is told.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::caching::invited_person;
use super::http::{Browser, called, rig_and_app, setup_body};

const PROJECTS: &str = "/api/v1/projects";

fn id_of(reply: &Value) -> Result<String, &'static str> {
    reply["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or("the answer holds no id")
}

fn short_names(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["short_name"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// An organization with its administrator signed in, and the router that serves it.
async fn administrator(db: &bb_testing::TestDatabase) -> Result<(Browser, axum::Router), Failure> {
    let (rig, app) = rig_and_app(db, 10).await?;
    let mut ada = Browser::new(&app, &called());
    ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
        .await?;
    Ok((ada, app))
}

/// Makes a project at the top level, or inside `parent`, and says what the server answered.
async fn make(
    browser: &mut Browser,
    parent: Option<&str>,
    short_name: &str,
    name: &str,
) -> Result<super::http::Reply, Failure> {
    let path = parent.map_or_else(
        || PROJECTS.to_owned(),
        |parent| format!("{PROJECTS}/{parent}/sub-projects"),
    );
    browser
        .send(
            "POST",
            &path,
            Some(json!({"short_name": short_name, "name": name})),
        )
        .await
}

#[tokio::test]
async fn prj_01_projects_are_made_over_http_by_the_rules_of_the_form() -> TestResult {
    each_database("prj_01_http_make", async |db| {
        let (mut ada, _app) = administrator(db).await?;
        let acme = make(&mut ada, None, "ACME", "Acme").await?;
        assert_eq!(acme.status, StatusCode::OK, "{}", acme.body);
        assert_eq!(acme.body["version"], 1);
        let acme_id = id_of(&acme.body)?;
        let bad = make(&mut ada, None, "acme", "Acme").await?;
        assert_eq!(
            (bad.status, bad.code()),
            (StatusCode::BAD_REQUEST, "request.invalid")
        );
        let taken = make(&mut ada, Some(&acme_id), "ACME", "Again").await?;
        assert_eq!(
            (taken.status, taken.code()),
            (StatusCode::CONFLICT, "name.taken")
        );
        let web = make(&mut ada, Some(&acme_id), "WEB", "Website").await?;
        assert_eq!(web.status, StatusCode::OK, "{}", web.body);
        assert_eq!(web.body["parent"], acme_id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_02_projects_are_listed_as_a_tree_a_search_and_pages_over_http() -> TestResult {
    each_database("prj_02_http_list", async |db| {
        let (mut ada, _app) = administrator(db).await?;
        let acme_id = id_of(&make(&mut ada, None, "ACME", "Acme").await?.body)?;
        make(&mut ada, Some(&acme_id), "WEB", "Website").await?;
        let top = ada.send("GET", PROJECTS, None).await?;
        assert_eq!(top.status, StatusCode::OK);
        assert_eq!(short_names(&top.body), ["ACME"]);
        assert_eq!(top.body["items"][0]["has_sub_projects"], true);
        assert!(top.body["next_cursor"].is_null());
        assert_eq!(top.header("cache-control"), "no-store");
        let path = format!("{PROJECTS}?inside={acme_id}&fields=short_name,parent");
        let inside = ada.send("GET", &path, None).await?;
        assert_eq!(short_names(&inside.body), ["WEB"]);
        assert_eq!(inside.body["items"][0]["parent"]["short_name"], "ACME");
        assert!(inside.body["items"][0].get("name").is_none());
        let found = ada
            .send("GET", &format!("{PROJECTS}?search=site"), None)
            .await?;
        assert_eq!(short_names(&found.body), ["WEB"]);
        for short_name in ["CCC", "BBB"] {
            make(&mut ada, None, short_name, short_name).await?;
        }
        let first = ada
            .send("GET", &format!("{PROJECTS}?limit=2"), None)
            .await?;
        let cursor = first.body["next_cursor"].as_str().ok_or("no next page")?;
        let second = ada
            .send("GET", &format!("{PROJECTS}?limit=2&cursor={cursor}"), None)
            .await?;
        assert_eq!(short_names(&first.body), ["ACME", "BBB"]);
        assert_eq!(short_names(&second.body), ["CCC"]);
        let bad_cursor = ada
            .send("GET", &format!("{PROJECTS}?cursor=nonsense"), None)
            .await?;
        assert_eq!(bad_cursor.code(), "request.bad_cursor");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_project_is_shown_by_its_address_and_changed_over_http() -> TestResult {
    each_database("prj_01_http_change", async |db| {
        let (mut ada, _app) = administrator(db).await?;
        let acme_id = id_of(&make(&mut ada, None, "ACME", "Acme").await?.body)?;
        let web = make(&mut ada, Some(&acme_id), "WEB", "Website").await?;
        let web_id = id_of(&web.body)?;
        let one = ada.send("GET", &format!("{PROJECTS}/WEB"), None).await?;
        assert_eq!(one.status, StatusCode::OK, "{}", one.body);
        assert_eq!(one.body["trail"][0]["short_name"], "ACME");
        assert_eq!(one.body["may_change_short_name"], true);
        assert!(
            one.body["permissions"]
                .as_array()
                .is_some_and(|all| all.contains(&json!("move-projects")))
        );
        // A short name changes and the old address keeps working; a stale version is stopped.
        let patch = format!("{PROJECTS}/{web_id}");
        let renamed = ada
            .send(
                "PATCH",
                &patch,
                Some(json!({"version": 1, "short_name": "SITE", "name": "The site"})),
            )
            .await?;
        assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
        assert_eq!(renamed.body["version"], 2);
        let old = ada.send("GET", &format!("{PROJECTS}/WEB"), None).await?;
        assert_eq!(old.body["project"]["short_name"], "SITE");
        let stale = ada
            .send(
                "PATCH",
                &patch,
                Some(json!({"version": 1, "name": "Stale"})),
            )
            .await?;
        assert_eq!(
            (stale.status, stale.code()),
            (StatusCode::CONFLICT, "edit.version_changed")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_person_without_a_role_sees_no_project_and_is_told_nothing() -> TestResult {
    each_database("prj_01_http_hidden", async |db| {
        let (mut ada, app) = administrator(db).await?;
        let mut bob = invited_person(&mut ada, &app, "bob@example.test").await?;
        make(&mut ada, None, "ACME", "Acme").await?;
        // The list is empty and says nothing more; a project is not found, exactly like one that
        // does not exist; he may make nothing.
        let empty = bob.send("GET", PROJECTS, None).await?;
        assert_eq!(empty.status, StatusCode::OK);
        assert!(short_names(&empty.body).is_empty());
        let hidden = bob.send("GET", &format!("{PROJECTS}/ACME"), None).await?;
        let missing = bob.send("GET", &format!("{PROJECTS}/NOSUCH"), None).await?;
        assert_eq!(hidden.status, StatusCode::NOT_FOUND);
        assert_eq!(hidden.body["code"], "resource.not_found");
        assert_eq!(hidden.body["message_key"], missing.body["message_key"]);
        assert_eq!(hidden.body["title"], missing.body["title"]);
        let denied = make(&mut bob, None, "MINE", "Mine").await?;
        assert_eq!(
            (denied.status, denied.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // Not signed in at all.
        let mut stranger = Browser::new(&app, &called());
        let nobody = stranger.send("GET", PROJECTS, None).await?;
        assert_eq!(
            (nobody.status, nobody.code()),
            (StatusCode::UNAUTHORIZED, "auth.not_signed_in")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_move_is_previewed_and_done_over_http() -> TestResult {
    each_database("prj_03_http", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let project =
            |short_name: &'static str| json!({"short_name": short_name, "name": short_name});
        let acme = id_of(
            &ada.send("POST", PROJECTS, Some(project("ACME")))
                .await?
                .body,
        )?;
        let web = id_of(&ada.send("POST", PROJECTS, Some(project("WEB"))).await?.body)?;

        let preview = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{web}/move-preview"),
                Some(json!({"to": acme})),
            )
            .await?;
        assert_eq!(preview.status, StatusCode::OK, "{}", preview.body);
        assert_eq!(preview.body["blocked"], false);
        assert_eq!(preview.body["to"]["short_name"], "ACME");
        assert!(preview.body["people"].as_array().is_some_and(Vec::is_empty));
        assert_eq!(preview.header("cache-control"), "no-store");
        let version = preview.body["preview"]
            .as_str()
            .ok_or("no version")?
            .to_owned();

        // A made up version is refused, and nothing moves.
        let wrong = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{web}/move"),
                Some(json!({"to": acme, "preview": "nonsense"})),
            )
            .await?;
        assert_eq!(
            (wrong.status, wrong.code()),
            (StatusCode::CONFLICT, "project.preview_changed")
        );
        // The project cannot go inside itself, and a text that is no ID is a project not found.
        let below = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{acme}/move-preview"),
                Some(json!({"to": acme})),
            )
            .await?;
        assert_eq!(
            (below.status, below.code()),
            (StatusCode::CONFLICT, "project.move_into_itself")
        );
        let nowhere = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{web}/move-preview"),
                Some(json!({"to": "nowhere"})),
            )
            .await?;
        assert_eq!(
            (nowhere.status, nowhere.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );

        let moved = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{web}/move"),
                Some(json!({"to": acme, "preview": version})),
            )
            .await?;
        assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
        assert_eq!(moved.body["parent"], acme);
        let one = ada.send("GET", &format!("{PROJECTS}/WEB"), None).await?;
        assert_eq!(one.body["trail"][0]["short_name"], "ACME");
        Ok(())
    })
    .await
}
