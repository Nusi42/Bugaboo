//! Saving the description of an issue under its lock over HTTP (ISS-17, M3 WP-06): the lock, the
//! version and the draft that keeps the text, the description rendered by the server, and the
//! image hosts that the preview and the content security policy use.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bb_api::{LimitsConfig, Options, WebSource, router};
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tower::ServiceExt as _;

use super::http::{Browser, options, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};

const LOCKS: &str = "/api/v1/edit-locks";
const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";
const A_TOKEN: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

fn what(id: &str) -> Value {
    json!({"record": {"kind": "issue", "id": id}, "part": "description"})
}

/// The lock on the description of an issue; the token.
async fn lock(browser: &mut Browser, id: &str) -> Result<String, Failure> {
    let taken = browser
        .send(
            "POST",
            LOCKS,
            Some(json!({"what": what(id), "editor_id": AN_EDITOR})),
        )
        .await?;
    assert_eq!(taken.body["status"], "taken", "{}", taken.body);
    Ok(taken.body["lock_token"]
        .as_str()
        .ok_or("no token")?
        .to_owned())
}

/// The internal ID and the readable ID of an answer that made an issue.
fn ids_of(made: &Value) -> Result<(String, String), Failure> {
    let text = |name: &str| {
        made[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| Failure::from(format!("no {name}")))
    };
    Ok((text("id")?, text("key")?))
}

#[tokio::test]
async fn iss_17_the_description_is_saved_under_the_lock_and_rendered_by_the_server() -> TestResult {
    each_database("iss_17_save_description", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.make(&mut ada, "The login button does nothing").await?;
        let (id, key) = ids_of(ok(&made)?)?;
        let path = format!("{ISSUES}/{key}/description");
        let token = lock(&mut ada, &id).await?;
        let saved = ada
            .send(
                "PUT",
                &path,
                Some(json!({"version": 1, "lock_token": token, "text": "It **fails** on Monday."})),
            )
            .await?;
        assert_eq!(ok(&saved)?["version"], 2);
        assert_eq!(saved.body["description"], "It **fails** on Monday.");
        // The page asks for the issue and gets the text rendered by the server, with the mark of
        // the description in the API description: the same renderer as the preview.
        let read = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        assert_eq!(
            ok(&read)?["description_html"],
            "<p>It <strong>fails</strong> on Monday.</p>\n"
        );
        // The lock was let go by the save; a second save with the old token is refused and keeps
        // the text as a draft.
        let again = ada
            .send(
                "PUT",
                &path,
                Some(json!({"version": 2, "lock_token": token, "text": "Second try"})),
            )
            .await?;
        assert_eq!(again.status, StatusCode::CONFLICT);
        assert_eq!(again.code(), "edit.lock_lost");
        let draft = ada
            .send(
                "GET",
                &format!("/api/v1/drafts?kind=issue&id={id}&part=description"),
                None,
            )
            .await?;
        assert_eq!(draft.body["draft"]["text"], "Second try");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f50_a_change_never_carries_the_description_whoever_holds_the_lock() -> TestResult {
    each_database("m3_f50_patch_no_description", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.make(&mut ada, "Nobody overwrites the text").await?;
        let (id, key) = ids_of(ok(&made)?)?;
        let _held = lock(&mut ada, &id).await?;
        let path = format!("{ISSUES}/{key}");
        let refused = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "description": "Sneaked in"})),
            )
            .await?;
        assert!(refused.status.is_client_error(), "{}", refused.status);
        let read = ada.send("GET", &path, None).await?;
        assert_eq!(ok(&read)?["issue"]["description"], "");
        assert_eq!(read.body["issue"]["version"], 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ux_06_the_box_learns_who_is_editing_before_the_first_key() -> TestResult {
    each_database("ux_06_read_edit_lock", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let made = site.make(&mut ada, "Somebody is typing").await?;
        let (id, _) = ids_of(ok(&made)?)?;
        let ask = format!("{LOCKS}?kind=issue&id={id}&part=description");
        // Nobody edits: nothing is held, and asking takes no lock.
        let free = tim.send("GET", &ask, None).await?;
        assert_eq!(ok(&free)?["held"], Value::Null);
        let _token = lock(&mut ada, &id).await?;
        let held = tim.send("GET", &ask, None).await?;
        assert_eq!(ok(&held)?["held"]["yours_elsewhere"], false);
        assert!(
            held.body["held"]["holder"]["id"].is_string(),
            "{}",
            held.body
        );
        // Asking did not take the lock: Tim can still take it only after Ada lets go.
        let taken = tim
            .send(
                "POST",
                LOCKS,
                Some(json!({"what": what(&id), "editor_id": AN_EDITOR})),
            )
            .await?;
        assert_eq!(taken.body["status"], "held");
        // A person who may not change the issue is not told who edits.
        let refused = vera.send("GET", &ask, None).await?;
        assert_eq!(refused.status, StatusCode::FORBIDDEN);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0019_a_stale_version_gets_the_structured_answer_and_the_text_is_kept() -> TestResult {
    each_database("adr_0019_issue_stale", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.make(&mut ada, "Stale").await?;
        let (id, key) = ids_of(ok(&made)?)?;
        let token = lock(&mut ada, &id).await?;
        // Somebody changes the priority meanwhile.
        let other = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 1, "priority": "high"})),
            )
            .await?;
        assert_eq!(other.status, StatusCode::OK);
        let stale = ada
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/description"),
                Some(json!({"version": 1, "lock_token": token, "text": "My text"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT);
        assert_eq!(stale.code(), "edit.version_changed");
        assert_eq!(stale.body["conflict"]["version"], 2);
        assert_eq!(stale.body["conflict"]["fields"], json!(["priority"]));
        let draft = ada
            .send(
                "GET",
                &format!("/api/v1/drafts?kind=issue&id={id}&part=description"),
                None,
            )
            .await?;
        assert_eq!(draft.body["draft"]["text"], "My text");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_who_may_not_change_the_issue_gets_no_lock_and_no_save() -> TestResult {
    each_database("iss_17_lock_permission", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut viewer, _) = site.person(StartingRole::Viewer, "vera").await?;
        let made = site.make(&mut ada, "Not yours").await?;
        let (id, key) = ids_of(ok(&made)?)?;
        let refused = viewer
            .send(
                "POST",
                LOCKS,
                Some(json!({"what": what(&id), "editor_id": AN_EDITOR})),
            )
            .await?;
        assert_eq!(refused.status, StatusCode::FORBIDDEN);
        let no_save = viewer
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/description"),
                Some(json!({"version": 1, "lock_token": A_TOKEN, "text": "Mine"})),
            )
            .await?;
        assert_eq!(no_save.status, StatusCode::FORBIDDEN);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f10_the_image_hosts_come_split_and_an_image_from_one_is_shown() -> TestResult {
    each_database("m3_f10_image_hosts", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut cora, _) = site.person(StartingRole::Contributor, "cora").await?;
        let none = cora
            .send("GET", "/api/v1/content/image-hosts", None)
            .await?;
        assert_eq!(ok(&none)?["hosts"], json!([]));
        let changed = ada
            .send(
                "PUT",
                "/api/v1/settings/content.image-hosts",
                Some(json!({"value": "images.example.com, cdn.example.org"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        // Every signed-in person reads the list, already split.
        let hosts = cora
            .send("GET", "/api/v1/content/image-hosts", None)
            .await?;
        assert_eq!(
            ok(&hosts)?["hosts"],
            json!(["images.example.com", "cdn.example.org"])
        );
        // The server renders an image from a listed host as an image and from any other as a link.
        let made = site.make(&mut ada, "Pictures").await?;
        let (id, key) = ids_of(ok(&made)?)?;
        let token = lock(&mut ada, &id).await?;
        ada.send(
            "PUT",
            &format!("{ISSUES}/{key}/description"),
            Some(json!({
                "version": 1,
                "lock_token": token,
                "text": "![a](https://images.example.com/a.png) ![b](https://evil.test/b.png)"
            })),
        )
        .await?;
        let read = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        let html = ok(&read)?["description_html"].as_str().ok_or("no html")?;
        assert!(html.contains("<img"), "{html}");
        assert!(html.contains("images.example.com"), "{html}");
        assert_eq!(html.matches("<img").count(), 1, "{html}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f10_the_policy_of_a_page_lists_the_image_hosts_of_the_organization() -> TestResult {
    each_database("m3_f10_policy_of_a_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let folder = rig.folder.path().join("web");
        std::fs::create_dir(&folder)?;
        Options::write_test_app(&folder)?;
        let pages = router(Options {
            web: WebSource::Folder(folder),
            ..options(&rig, LimitsConfig::default())
        });
        let policy_of = async |address: &str| -> Result<String, Failure> {
            let response = pages
                .clone()
                .oneshot(Request::get(address).body(Body::empty())?)
                .await?;
            Ok(response
                .headers()
                .get("content-security-policy")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned())
        };
        let changed = ada
            .send(
                "PUT",
                "/api/v1/settings/content.image-hosts",
                Some(json!({"value": "images.example.com"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let after = policy_of("/").await?;
        assert!(
            after.contains("img-src 'self' data: https://images.example.com;"),
            "{after}"
        );
        // The answers of the API keep the plain policy: nothing there shows an image.
        let api = policy_of("/api/v1/content/image-hosts").await?;
        assert!(api.contains("img-src 'self' data:;"), "{api}");
        Ok(())
    })
    .await
}
