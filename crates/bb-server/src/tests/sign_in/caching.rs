//! Entity tags, `304` and what is never stored (PERF-03), over every route of the API description.
//!
//! The rules for every single request (what may be kept, what never is) are also checked on all
//! requests of all tests that use a `Browser` (`http::assert_cache_rules`).

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use bb_api::openapi::operations;
use bb_app::access::Permission;
use bb_app::context::SystemIds;
use bb_app::many_changes::ManyChangeStore;
use bb_app::ports::ChangeStore;
use bb_app::roles::CreateRole;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::id::{EntityId, EventId, ManyChangeRunId};
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus};
use bb_store::AuthStorage;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;
use tower::ServiceExt as _;

use super::http::{ADA_EMAIL, Browser, called, never_stored_reads, totp};
use super::rig::{PASSWORD, Rig};
use super::status::app_with_probe;

/// The download of a file by its signed link: its answer is the bytes of the file, kept apart from
/// the records, and what it may be kept as is proven with the downloads (`files_http`).
const SIGNED_DOWNLOAD: &str = "/api/v1/files/{file}";

/// The addresses of every `GET` route in the API description, except the signed download.
fn get_routes() -> Vec<String> {
    operations()
        .iter()
        .filter(|operation| operation.method == Method::GET && operation.path != SIGNED_DOWNLOAD)
        .map(|operation| operation.path.clone())
        .collect()
}

/// A run of a change to many issues that the person asked for and that is done, for the route that
/// names one: its ID. A run exists only after more than a thousand issues were changed at once, so
/// the row is written as the first request of such a change writes it.
async fn a_run_of(rig: &Rig, person: &str) -> Result<String, Failure> {
    let mut ids = SystemIds;
    let run = ManyChangeRun {
        id: ManyChangeRunId::generate(&mut ids)?,
        person: person.parse().map_err(|_| "no person")?,
        action: json!({"kind": "delete"}),
        items: Vec::new(),
        status: ManyChangeStatus::Done,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: rig.clock_now(),
        finished_at: Some(rig.clock_now()),
    };
    let event = DomainEvent {
        id: EventId::generate(&mut ids)?,
        time: rig.clock_now(),
        actor: Actor::Person(run.person),
        channel: Channel::Ui,
        entity: EntityRef {
            kind: EntityKind::from(RecordKind::ChangeRun),
            id: EntityId::from(run.id),
        },
        changes: Vec::new(),
        reason: None,
    };
    let stored = run.clone();
    ChangeStore::record_change(&rig.storage, &event, move |transaction| {
        Box::pin(async move { AuthStorage::insert_many_changes_run(transaction, &stored).await })
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(run.id.to_string())
}

/// An upload that was started on the issue, for the route that names one: its ID.
async fn an_upload_on(ada: &mut Browser, readable_id: &str) -> Result<String, Failure> {
    let started = ada
        .send(
            "POST",
            &format!("/api/v1/issues/{readable_id}/uploads"),
            Some(json!({"name": "notes.txt", "size": 10})),
        )
        .await?;
    assert_eq!(started.status, StatusCode::OK);
    Ok(started.body["id"].as_str().ok_or("no upload")?.to_owned())
}

/// The ID of a role, for the routes that name one.
async fn a_role(rig: &Rig) -> Result<String, Failure> {
    let admin = rig.sign_in(ADA_EMAIL, PASSWORD).await?.session_token;
    let role = rig
        .api()
        .create_role(
            Rig::info(),
            CreateRole {
                session_token: admin,
                name: "Checker".to_owned(),
                permissions: vec![Permission::SeeProjects],
                two_step: false,
            },
        )
        .await?
        .into_value();
    Ok(role.id.to_string())
}

/// The ID of a comment written on the issue, for the routes that name a comment.
async fn a_comment_on(ada: &mut Browser, readable_id: &str) -> Result<String, Failure> {
    let written = ada
        .send(
            "POST",
            &format!("/api/v1/issues/{readable_id}/comments"),
            Some(json!({"text": "A comment to read the versions of."})),
        )
        .await?;
    assert_eq!(written.status, StatusCode::OK);
    Ok(written.body["id"].as_str().ok_or("no comment")?.to_owned())
}

/// An issue in the project EXAMPLE, for the route that names one: its readable ID.
async fn an_issue_in_example(ada: &mut Browser) -> Result<String, Failure> {
    let types = ada.send("GET", "/api/v1/issue-types", None).await?;
    let issue = ada
        .send(
            "POST",
            "/api/v1/projects/EXAMPLE/issues",
            Some(json!({
                "issue_type": types.body["items"][0]["id"],
                "title": "An issue to ask for"
            })),
        )
        .await?;
    assert_eq!(issue.status, StatusCode::OK, "{}", issue.body);
    Ok(issue.body["key"].as_str().ok_or("no issue")?.to_owned())
}

/// An own field, for the route that names one: its ID.
async fn a_field(ada: &mut Browser) -> Result<String, Failure> {
    let field = ada
        .send(
            "POST",
            "/api/v1/fields",
            Some(json!({"name": "Customer", "kind": "text", "applies_to": "issues"})),
        )
        .await?;
    assert_eq!(field.status, StatusCode::OK, "{}", field.body);
    Ok(field.body["id"].as_str().ok_or("no field")?.to_owned())
}

/// The IDs of a comment on the issue and of an own field.
async fn a_comment_and_a_field(
    ada: &mut Browser,
    readable_id: &str,
) -> Result<(String, String), Failure> {
    Ok((a_comment_on(ada, readable_id).await?, a_field(ada).await?))
}

/// The address of a route that is about something named in its query: the draft and lock routes
/// name a record (the example project), the layout routes the project and a kind of form.
fn with_query(template: &str, filled: String, project: &str, project_id: &str) -> String {
    match template {
        "/api/v1/drafts" | "/api/v1/edit-locks" => {
            format!("{filled}?kind=project&id={project_id}&part=description")
        }
        "/api/v1/layouts/form" | "/api/v1/layouts/editor" => {
            format!("{filled}?project={project}&kind=new-issue")
        }
        "/api/v1/workflows" | "/api/v1/workflows/editor" => format!("{filled}?project={project}"),
        _ => filled,
    }
}

#[tokio::test]
async fn perf_03_every_get_answers_an_etag_and_304_on_a_match() -> TestResult {
    each_database("perf_03_http_etags", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;

        // A role and a project for the routes that name one.
        let role = a_role(&rig).await?;
        let made = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "EXAMPLE", "name": "Example"})),
            )
            .await?;
        assert_eq!(made.status, StatusCode::OK);
        // A group and the person themselves, for the routes of people and groups.
        let group = ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Checkers"})))
            .await?;
        assert_eq!(group.status, StatusCode::OK);
        let (group_id, project_id, person_id) = (
            group.body["id"].as_str().ok_or("no group")?.to_owned(),
            made.body["id"].as_str().ok_or("no project")?.to_owned(),
            ada.send("GET", "/api/v1/auth/me", None).await?.body["id"]
                .as_str()
                .ok_or("no person")?
                .to_owned(),
        );
        let readable_id = an_issue_in_example(&mut ada).await?;
        // A comment and an own field, for the routes that name one.
        let (comment_id, field_id) = a_comment_and_a_field(&mut ada, &readable_id).await?;
        let run_id = a_run_of(&rig, &person_id).await?;
        let upload_id = an_upload_on(&mut ada, &readable_id).await?;
        let routes = get_routes();
        assert!(routes.len() >= 9, "{routes:?}");
        let (mut kept, mut never_stored) = (0, 0);
        for template in &routes {
            // A route with a part to fill in needs an example address here; fail loudly so a
            // new route is never left out of this test.
            // The routes of *Who has which role* take the ID of a project, not a short name.
            let project = if template.contains("assignment") {
                &project_id
            } else {
                "EXAMPLE"
            };
            let filled = template
                .replace("{role_id}", &role)
                .replace("{group_id}", &group_id)
                .replace("{person_id}", &person_id)
                .replace("{person}", &person_id)
                .replace("{issue}", &readable_id)
                .replace("{comment}", &comment_id)
                .replace("{field}", &field_id)
                .replace("{run}", &run_id)
                .replace("{upload}", &upload_id)
                .replace("{project}", project);
            let filled = with_query(template, filled, project, &project_id);
            let filled = if template.ends_with("/status-changes") {
                format!("{filled}?person={person_id}")
            } else {
                filled
            };
            let path = &filled;
            assert!(
                !path.contains('{'),
                "{path} has a part to fill in: add an example address to this test"
            );
            ada.clear_headers();
            let first = ada.send("GET", path, None).await?;
            assert_eq!(first.status, StatusCode::OK, "{path}");

            if never_stored_reads().contains(template) {
                // Personal data: no tag to ask with, and a tag or `*` changes nothing.
                never_stored += 1;
                assert_eq!(first.header("cache-control"), "no-store", "{path}");
                assert_eq!(first.header("etag"), "", "{path}");
                ada.set_header("if-none-match", "*");
                let asked = ada.send("GET", path, None).await?;
                assert_eq!(asked.status, StatusCode::OK, "{path}");
                continue;
            }

            kept += 1;
            let tag = first.header("etag").to_owned();
            assert!(tag.starts_with("W/\""), "{path}: {tag}");
            assert_eq!(first.header("cache-control"), "private, no-cache", "{path}");

            // The tag it gave: no body, the same tag.
            ada.set_header("if-none-match", &tag);
            let again = ada.send("GET", path, None).await?;
            assert_eq!(again.status, StatusCode::NOT_MODIFIED, "{path}");
            assert_eq!(again.header("etag"), tag, "{path}");
            assert!(again.body.is_null(), "{path}");

            // Some other tag: the answer in full.
            ada.set_header("if-none-match", "W/\"another\"");
            let other = ada.send("GET", path, None).await?;
            assert_eq!(other.status, StatusCode::OK, "{path}");
        }
        assert!(
            kept >= 4 && never_stored >= 5,
            "{kept} kept, {never_stored} never stored"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_a_read_asked_about_again_is_not_modified_when_nothing_changed() -> TestResult {
    each_database("perf_03_http_not_modified", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let path = "/api/v1/auth/sign-up-policy";
        let first = ada.send("GET", path, None).await?;
        let tag = first.header("etag").to_owned();
        ada.set_header("if-none-match", &tag);
        let same = ada.send("GET", path, None).await?;
        assert_eq!(same.status, StatusCode::NOT_MODIFIED);
        assert!(same.body.is_null());

        // The answer changes: the old tag no longer matches, and the new answer has a new one.
        let changed = ada
            .send(
                "PUT",
                path,
                Some(json!({"enabled": true, "domains": ["example.test"], "version": 0})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK);
        let after = ada.send("GET", path, None).await?;
        assert_eq!(after.status, StatusCode::OK);
        assert_ne!(after.header("etag"), tag);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_answers_with_personal_data_are_never_stored_for_anybody() -> TestResult {
    each_database("perf_03_http_personal_data", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let mut bob = ada.invite("bob@example.test", "Bob").await?;
        for person in [&mut ada, &mut bob] {
            for path in [
                "/api/v1/auth/me",
                "/api/v1/auth/sessions",
                "/api/v1/auth/two-step",
            ] {
                let reply = person.send("GET", path, None).await?;
                assert_eq!(reply.status, StatusCode::OK, "{path}");
                assert_eq!(reply.header("cache-control"), "no-store", "{path}");
                assert_eq!(reply.header("etag"), "", "{path}");
            }
        }
        // Different people, different answers, and neither is kept for the next person at the
        // same computer.
        let ada_me = ada.send("GET", "/api/v1/auth/me", None).await?;
        let bob_me = bob.send("GET", "/api/v1/auth/me", None).await?;
        assert_ne!(ada_me.body["email"], bob_me.body["email"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_answers_with_a_secret_are_not_stored_or_compressed() -> TestResult {
    each_database("perf_03_http_secret_answers", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        // The browser accepts every kind of compression it knows.
        ada.set_header("accept-encoding", "br, gzip, deflate, zstd");

        let secret_answers = [
            ada.set_up(&rig).await?,
            ada.send(
                "POST",
                "/api/v1/auth/invitations",
                Some(json!({"email": "bob@example.test"})),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/code-app",
                Some(json!({"password": PASSWORD})),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/code-app/confirm",
                Some(json!({
                    "code": totp(&rig, ADA_EMAIL).await?,
                    "password": PASSWORD,
                })),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/backup-codes",
                Some(json!({"password": PASSWORD})),
            )
            .await?,
        ];
        for reply in &secret_answers {
            assert_eq!(reply.status, StatusCode::OK);
            assert_eq!(reply.header("cache-control"), "no-store");
            assert_eq!(reply.header("etag"), "");
            assert_eq!(reply.header("content-encoding"), "");
        }
        // An answer that sets a cookie is never compressed either (signing in has no secret
        // field, but the session it starts is one).
        let mut again = Browser::new(&app, &called());
        again.set_header("accept-encoding", "br, gzip, deflate, zstd");
        let signed_in = again.sign_in(ADA_EMAIL, PASSWORD).await?;
        assert!(!signed_in.cookies().is_empty());
        assert_eq!(signed_in.header("content-encoding"), "");
        // The control: this very server does compress a big answer that holds no secret (the API
        // description), so the plain answers above are plain by the rule and not by chance.
        let request = Request::get("/api/v1/openapi.json")
            .header("accept-encoding", "br, gzip")
            .body(Body::empty())?;
        let big = app.clone().oneshot(request).await?;
        assert_eq!(big.headers()["content-encoding"], "br");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_signing_out_clears_what_the_browser_cached() -> TestResult {
    each_database("sec_12_http_clear_cache", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        let signed_in = ada.set_up(&rig).await?;
        // Signing in does not clear anything.
        assert_eq!(signed_in.header("clear-site-data"), "");

        // Signing out ends the session, forgets the cookies and clears the cache.
        let out = ada.send("POST", "/api/v1/auth/sign-out", None).await?;
        assert_eq!(out.status, StatusCode::NO_CONTENT);
        assert_eq!(out.header("clear-site-data"), "\"cache\"");
        assert_eq!(out.header("cache-control"), "no-store");

        // The same for signing out everywhere.
        let mut again = Browser::new(&app, &called());
        again.sign_in(ADA_EMAIL, PASSWORD).await?;
        let everywhere = again.send("DELETE", "/api/v1/auth/sessions", None).await?;
        assert_eq!(everywhere.status, StatusCode::NO_CONTENT);
        assert_eq!(everywhere.header("clear-site-data"), "\"cache\"");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_ending_the_own_session_is_signing_out_but_ending_another_is_not() -> TestResult {
    each_database("sec_12_http_end_own_session", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut laptop = Browser::signed_in(&app, &rig).await?;
        let mut phone = Browser::new(&app, &called());
        phone.sign_in(ADA_EMAIL, PASSWORD).await?;
        let listed = laptop.send("GET", "/api/v1/auth/sessions", None).await?;
        let sessions = listed.body["sessions"].as_array().ok_or("no sessions")?;
        let id_of = |current: bool| {
            sessions
                .iter()
                .find(|session| session["current"] == current)
                .and_then(|session| session["id"].as_str())
                .map(str::to_owned)
        };
        let own = id_of(true).ok_or("no current session")?;
        let other = id_of(false).ok_or("no other session")?;

        // Another session: ended, and this browser keeps its cookies and its cache.
        let ended = laptop
            .send("DELETE", &format!("/api/v1/auth/sessions/{other}"), None)
            .await?;
        assert_eq!(ended.status, StatusCode::NO_CONTENT);
        assert_eq!(ended.header("clear-site-data"), "");
        assert_eq!(
            laptop.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );

        // The own session: the same as signing out, cookies and cache.
        let own_end = laptop
            .send("DELETE", &format!("/api/v1/auth/sessions/{own}"), None)
            .await?;
        assert_eq!(own_end.status, StatusCode::NO_CONTENT);
        assert_eq!(own_end.header("clear-site-data"), "\"cache\"");
        let after = laptop.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(after.status, StatusCode::UNAUTHORIZED);
        Ok(())
    })
    .await
}
