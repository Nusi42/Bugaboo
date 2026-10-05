//! The project routes over HTTP (PRJ-01, PRJ-02, PRJ-03): the answers a browser gets, with their
//! codes, and what a person who may not see a project is told.

use axum::http::StatusCode;
use bb_testing::each_database;
use serde_json::{Value, json};

use super::caching::invited_person;
use super::http::{Browser, called, rig_and_app, setup_body};
use super::rig::Outcome;

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

#[tokio::test]
async fn prj_01_projects_are_made_listed_and_changed_over_http() -> Outcome {
    each_database("prj_01_http", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut bob = invited_person(&mut ada, &app, "bob@example.test").await?;

        // Making: the top level, then a sub-project; the form's rules are the server's.
        let acme = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "ACME", "name": "Acme"})),
            )
            .await?;
        assert_eq!(acme.status, StatusCode::OK, "{}", acme.body);
        let acme_id = id_of(&acme.body)?;
        assert_eq!(acme.body["version"], 1);
        let bad = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "acme", "name": "Acme"})),
            )
            .await?;
        assert_eq!(bad.status, StatusCode::BAD_REQUEST);
        assert_eq!(bad.code(), "request.invalid");
        let taken = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{acme_id}/sub-projects"),
                Some(json!({"short_name": "ACME", "name": "Again"})),
            )
            .await?;
        assert_eq!(
            (taken.status, taken.code()),
            (StatusCode::CONFLICT, "name.taken")
        );
        let web = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{acme_id}/sub-projects"),
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?;
        assert_eq!(web.status, StatusCode::OK, "{}", web.body);
        assert_eq!(web.body["parent"], acme_id);

        // Listing: the top of the tree, what is inside, a search, fields, and pages.
        let top = ada.send("GET", PROJECTS, None).await?;
        assert_eq!(top.status, StatusCode::OK);
        assert_eq!(short_names(&top.body), ["ACME"]);
        assert_eq!(top.body["items"][0]["has_sub_projects"], true);
        assert!(top.body["next_cursor"].is_null());
        assert_eq!(top.header("cache-control"), "no-store");
        let inside = ada
            .send(
                "GET",
                &format!("{PROJECTS}?inside={acme_id}&fields=short_name,parent"),
                None,
            )
            .await?;
        assert_eq!(short_names(&inside.body), ["WEB"]);
        assert_eq!(inside.body["items"][0]["parent"]["short_name"], "ACME");
        assert!(inside.body["items"][0].get("name").is_none());
        let found = ada
            .send("GET", &format!("{PROJECTS}?search=site"), None)
            .await?;
        assert_eq!(short_names(&found.body), ["WEB"]);
        for short_name in ["CCC", "BBB"] {
            ada.send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": short_name, "name": short_name})),
            )
            .await?;
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

        // One project, by short name and by ID; the trail and what may be done here.
        let one = ada.send("GET", &format!("{PROJECTS}/WEB"), None).await?;
        assert_eq!(one.status, StatusCode::OK, "{}", one.body);
        assert_eq!(one.body["trail"][0]["short_name"], "ACME");
        assert_eq!(one.body["may_change_short_name"], true);
        assert!(
            one.body["permissions"]
                .as_array()
                .is_some_and(|all| all.contains(&json!("move-projects")))
        );
        // Changing: renaming, a short name (the old address keeps working), a stale version.
        let web_id = id_of(&web.body)?;
        let renamed = ada
            .send(
                "PATCH",
                &format!("{PROJECTS}/{web_id}"),
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
                &format!("{PROJECTS}/{web_id}"),
                Some(json!({"version": 1, "name": "Stale"})),
            )
            .await?;
        assert_eq!(
            (stale.status, stale.code()),
            (StatusCode::CONFLICT, "edit.version_changed")
        );

        // Bob has no role: the list is empty and says nothing more; a project is not found, and
        // exactly like one that does not exist; he may make nothing.
        let empty = bob.send("GET", PROJECTS, None).await?;
        assert_eq!(empty.status, StatusCode::OK);
        assert!(short_names(&empty.body).is_empty());
        let hidden = bob.send("GET", &format!("{PROJECTS}/ACME"), None).await?;
        let missing = bob.send("GET", &format!("{PROJECTS}/NOSUCH"), None).await?;
        assert_eq!(hidden.status, StatusCode::NOT_FOUND);
        assert_eq!(hidden.body["code"], "resource.not_found");
        assert_eq!(hidden.body["message_key"], missing.body["message_key"]);
        assert_eq!(hidden.body["title"], missing.body["title"]);
        let denied = bob
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "MINE", "name": "Mine"})),
            )
            .await?;
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
async fn prj_03_a_move_is_previewed_and_done_over_http() -> Outcome {
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
