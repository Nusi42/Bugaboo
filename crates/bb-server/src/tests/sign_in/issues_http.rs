//! Issues over HTTP (ISS-01, ISS-02, SEC-04): what people make, see and change, what a person who
//! may not see an issue is told, and the types an issue can have.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, key_in, ok};

const TYPES: &str = "/api/v1/issue-types";

/// The titles of a page, in the order of the page.
fn titles(list: &Reply) -> Vec<&str> {
    list.body["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["title"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// The titles of a page, sorted: issues made in the same millisecond (the clock of the tests
/// stands still) are in no order.
fn sorted_titles(list: &Reply) -> Vec<&str> {
    let mut found = titles(list);
    found.sort_unstable();
    found
}

/// An error answer without the ID of the request, which is the one thing that differs.
fn without_request_id(reply: &Reply) -> Value {
    let mut body = reply.body.clone();
    if let Some(members) = body.as_object_mut() {
        members.remove("request_id");
    }
    body
}

#[tokio::test]
async fn iss_01_a_person_makes_and_changes_an_issue_they_may() -> TestResult {
    each_database("iss_01_create_and_change", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut cora, cora_id) = site.person(StartingRole::Contributor, "cora").await?;
        // A Contributor makes an issue and changes the fields she may change in her own.
        let made = site.create(&mut cora, "The login button does nothing").await?;
        let issue = ok(&made)?;
        assert_eq!(
            issue["status"],
            json!({"key": "open", "name": "Open", "group": "not-started"})
        );
        assert_eq!(issue["priority"], "normal");
        assert_eq!(issue["version"], 1);
        assert_eq!(issue["reported_by"]["id"], cora_id.to_string());
        assert!(issue.get("closed_at").is_none());
        let key = key_in(issue)?;
        assert!(key.starts_with("ACME-"), "{key}");
        let path = format!("{ISSUES}/{key}");
        let renamed = cora
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "title": "The login button does nothing at all"})),
            )
            .await?;
        assert_eq!(ok(&renamed)?["version"], 2);
        assert_eq!(renamed.body["title"], "The login button does nothing at all");
        // The status is a field she can see and not change.
        let status = cora
            .send("PATCH", &path, Some(json!({"version": 2, "status": "done"})))
            .await?;
        assert_eq!(
            (status.status, status.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // Another person's issue is not hers to change, though she sees it.
        let adas = site.create(&mut ada, "Ada's issue").await?;
        let adas_path = format!("{ISSUES}/{}", key_in(ok(&adas)?)?);
        let theirs = cora
            .send("PATCH", &adas_path, Some(json!({"version": 1, "title": "Mine"})))
            .await?;
        assert_eq!(theirs.code(), "access.forbidden");
        let seen = cora.send("GET", &adas_path, None).await?;
        assert_eq!(ok(&seen)?["may_change"], false);
        // The administrator changes the status of Cora's issue; the closed date follows the
        // group Done, and leaves with it.
        let done = ada
            .send("PATCH", &path, Some(json!({"version": 2, "status": "done"})))
            .await?;
        assert_eq!(ok(&done)?["status"]["group"], "done");
        assert!(done.body["closed_at"].is_string());
        let reopened = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "status": "in-progress", "assigned_to": [cora_id.to_string()]})),
            )
            .await?;
        assert!(ok(&reopened)?.get("closed_at").is_none());
        assert_eq!(reopened.body["assigned_to"][0]["id"], cora_id.to_string());
        // A person who only looks cannot make one.
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let refused = site.create(&mut vera, "Not for her").await?;
        assert_eq!(refused.code(), "access.forbidden");
        // An empty request and a bad title are invalid.
        let nothing = ada.send("PATCH", &path, Some(json!({"version": 4}))).await?;
        assert_eq!(nothing.code(), "request.invalid");
        let blank = site.create(&mut ada, "   ").await?;
        assert_eq!(blank.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_a_hidden_issue_answers_not_found_like_a_missing_one() -> TestResult {
    each_database("iss_01_hidden_is_missing", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.create(&mut ada, "A secret").await?;
        let random_key = key_in(ok(&made)?)?;
        let internal = ok(&made)?["id"].as_str().ok_or("no id")?.to_owned();
        let trashed = key_in(ok(&site.create(&mut ada, "In the trash").await?)?)?;
        let deleted = ada
            .send("DELETE", &format!("{ISSUES}/{trashed}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        // The project counts from now on: the issue has a number, and its random ID is an old one.
        let switched = ada
            .send(
                "PUT",
                "/api/v1/projects/ACME/settings/issues.id-mode",
                Some(json!({"value": "counting"})),
            )
            .await?;
        ok(&switched)?;
        let counted = ada
            .send("GET", &format!("{ISSUES}/{random_key}"), None)
            .await?;
        let counting_key = key_in(&ok(&counted)?["issue"])?;
        assert_ne!(counting_key, random_key);
        let in_trash_now = ada
            .send("GET", &format!("{ISSUES}/{trashed}"), None)
            .await?;
        assert_eq!(in_trash_now.code(), "resource.not-found", "trashed is gone");
        let missing = format!("{ISSUES}/ACME-ZZZZZ");
        let elsewhere = format!("{ISSUES}/NOPE-ZZZZZ");
        let no_such_id = format!("{ISSUES}/{}", "01900000-0000-7000-8000-000000000000");
        // A Guest sees the project and no issue of others; a person with no role sees nothing.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let nobody_email = "nobody@example.test";
        rig.person(&site.org.admin.session, nobody_email).await?;
        let mut nobody = super::http::Browser::signed_in_as(&app, nobody_email).await?;
        // Every way to name the hidden issue: the key now, the old random key, the internal ID,
        // and the issue in the trash (by the key it had when it was trashed).
        let hidden_paths = [
            format!("{ISSUES}/{counting_key}"),
            format!("{ISSUES}/{random_key}"),
            format!("{ISSUES}/{internal}"),
            format!("{ISSUES}/{trashed}"),
        ];
        for browser in [&mut gus, &mut nobody] {
            for path in &hidden_paths {
                for method in ["GET", "DELETE"] {
                    let hidden = browser.send(method, path, None).await?;
                    let gone = browser.send(method, &missing, None).await?;
                    let far = browser.send(method, &elsewhere, None).await?;
                    let unknown_id = browser.send(method, &no_such_id, None).await?;
                    assert_eq!(
                        (hidden.status, hidden.code()),
                        (StatusCode::NOT_FOUND, "resource.not-found"),
                        "{method} {path}"
                    );
                    assert_eq!(without_request_id(&hidden), without_request_id(&gone));
                    assert_eq!(without_request_id(&hidden), without_request_id(&far));
                    assert_eq!(without_request_id(&hidden), without_request_id(&unknown_id));
                }
                let change = json!({"version": 1, "title": "Hello"});
                let hidden = browser.send("PATCH", path, Some(change.clone())).await?;
                let gone = browser.send("PATCH", &missing, Some(change)).await?;
                assert_eq!(hidden.code(), "resource.not-found", "PATCH {path}");
                assert_eq!(without_request_id(&hidden), without_request_id(&gone));
                let restore = browser
                    .send("POST", &format!("{path}/restore"), None)
                    .await?;
                let none = browser
                    .send("POST", &format!("{missing}/restore"), None)
                    .await?;
                assert_eq!(restore.code(), "resource.not-found", "restore {path}");
                assert_eq!(without_request_id(&restore), without_request_id(&none));
            }
        }
        // The hidden issues are still there for the person who may see them.
        assert_eq!(
            ada.send("GET", &format!("{ISSUES}/{internal}"), None)
                .await?
                .status,
            StatusCode::OK
        );
        // The project of a person with no role does not exist for them either.
        let listed = nobody
            .send("GET", &format!("{PROJECTS}/ACME/issues"), None)
            .await?;
        assert_eq!(listed.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_who_may_change_their_own_issues_may_change_its_type_but_not_that_of_others()
-> TestResult {
    each_database("iss_01_own_issue_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut cora, _) = site.person(StartingRole::Contributor, "cora").await?;
        let types = ada.send("GET", TYPES, None).await?;
        let bug = types.body["items"][0]["id"]
            .as_str()
            .ok_or("no bug")?
            .to_owned();
        // Changing the type takes the right to change the issue and no field of its own
        // (`Evaluation::may_change_issue`): *Change own issues* is enough on one's own.
        let own = site.create(&mut cora, "Mine").await?;
        let own_path = format!("{ISSUES}/{}", key_in(ok(&own)?)?);
        let moved = cora
            .send(
                "PATCH",
                &own_path,
                Some(json!({"version": 1, "issue_type": bug})),
            )
            .await?;
        assert_eq!(ok(&moved)?["issue_type"]["name"], "Bug");
        let theirs = site.create(&mut ada, "Ada's").await?;
        let their_path = format!("{ISSUES}/{}", key_in(ok(&theirs)?)?);
        let refused = cora
            .send(
                "PATCH",
                &their_path,
                Some(json!({"version": 1, "issue_type": bug})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_a_guest_sees_only_the_issues_they_reported() -> TestResult {
    each_database("iss_01_guest_sees_own", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        ok(&site.create(&mut ada, "Ada's issue").await?)?;
        let own = site.create(&mut gus, "Gus found a bug").await?;
        let own_key = key_in(ok(&own)?)?;
        let list = format!("{PROJECTS}/ACME/issues?fields=title");
        let seen = gus.send("GET", &list, None).await?;
        assert_eq!(titles(&seen), ["Gus found a bug"]);
        let everything = tim.send("GET", &list, None).await?;
        assert_eq!(
            sorted_titles(&everything),
            ["Ada's issue", "Gus found a bug"]
        );
        // He opens and changes his own, within the fields he may change.
        let path = format!("{ISSUES}/{own_key}");
        let opened = gus.send("GET", &path, None).await?;
        assert_eq!(ok(&opened)?["may_change"], true);
        assert_eq!(opened.body["issue"]["title"], "Gus found a bug");
        let changed = gus
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "title": "Gus found a worse bug"})),
            )
            .await?;
        assert_eq!(ok(&changed)?["title"], "Gus found a worse bug");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_a_stale_version_is_refused() -> TestResult {
    each_database("iss_01_stale_version", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let made = site.create(&mut ada, "Two people at once").await?;
        let path = format!("{ISSUES}/{}", key_in(ok(&made)?)?);
        let first = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "priority": "high"})),
            )
            .await?;
        assert_eq!(ok(&first)?["version"], 2);
        // Ada looked at version 1: the refusal is `edit.version-changed` with the structured
        // answer (ADR-0019): who changed it, which fields, and what they are now.
        let late = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "priority": "low"})),
            )
            .await?;
        assert_eq!(
            (late.status, late.code()),
            (StatusCode::CONFLICT, "edit.version-changed")
        );
        let conflict = &late.body["conflict"];
        assert_eq!(conflict["version"], 2, "{conflict}");
        assert_eq!(conflict["fields"], json!(["priority"]), "{conflict}");
        assert_eq!(conflict["current"]["priority"], "high", "{conflict}");
        assert!(conflict["changed_by"].is_object(), "{conflict}");
        assert!(conflict["changed_at"].is_string(), "{conflict}");
        let current = ada.send("GET", &path, None).await?;
        assert_eq!(ok(&current)?["issue"]["priority"], "high");
        assert_eq!(current.body["issue"]["version"], 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_an_issue_goes_to_the_trash_and_comes_back_with_undo() -> TestResult {
    each_database("iss_01_trash_and_undo", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut cora, _) = site.person(StartingRole::Contributor, "cora").await?;
        let made = site.create(&mut cora, "Mistake").await?;
        let key = key_in(ok(&made)?)?;
        let path = format!("{ISSUES}/{key}");
        // A Contributor may not delete, not even her own.
        let refused = cora.send("DELETE", &path, None).await?;
        assert_eq!(refused.code(), "access.forbidden");
        let deleted = ada.send("DELETE", &path, None).await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let list = format!("{PROJECTS}/ACME/issues?fields=title");
        assert!(titles(&ada.send("GET", &list, None).await?).is_empty());
        assert_eq!(
            ada.send("GET", &path, None).await?.code(),
            "resource.not-found"
        );
        assert_eq!(
            ada.send("DELETE", &path, None).await?.code(),
            "resource.not-found",
            "it is in the trash already"
        );
        assert_eq!(
            cora.send("POST", &format!("{path}/restore"), None)
                .await?
                .code(),
            "resource.not-found",
            "whoever may not restore is not told that it is in the trash"
        );
        let back = ada.send("POST", &format!("{path}/restore"), None).await?;
        assert_eq!(ok(&back)?["key"], key);
        assert_eq!(titles(&ada.send("GET", &list, None).await?), ["Mistake"]);
        assert_eq!(
            ada.send("POST", &format!("{path}/restore"), None)
                .await?
                .code(),
            "resource.not-found",
            "it is not in the trash"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_02_a_retired_type_keeps_its_issues_and_takes_no_new_ones() -> TestResult {
    each_database("iss_02_retired_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = site.create(&mut ada, "A task").await?;
        let path = format!("{ISSUES}/{}", key_in(ok(&made)?)?);
        let types = ada.send("GET", TYPES, None).await?;
        let bug = types.body["items"][0]["id"]
            .as_str()
            .ok_or("no bug")?
            .to_owned();
        let names: Vec<&str> = types.body["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(names, ["Bug", "Task", "Feature", "Goal"]);
        let retired = ada
            .send(
                "PATCH",
                &format!("{TYPES}/{}", site.kind),
                Some(json!({"version": 1, "retired": true})),
            )
            .await?;
        assert_eq!(ok(&retired)?["retired"], true);
        // No new issue of the type; the old one keeps it and can still be changed.
        let refused = site.create(&mut ada, "Another task").await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::CONFLICT, "issue.type-retired")
        );
        let kept = ada.send("GET", &path, None).await?;
        assert_eq!(ok(&kept)?["issue"]["issue_type"]["name"], "Task");
        assert_eq!(kept.body["issue"]["issue_type"]["retired"], true);
        let changed = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "priority": "high"})),
            )
            .await?;
        assert_eq!(ok(&changed)?["issue_type"]["retired"], true);
        // Moving an issue to a retired type is refused as well; to another type it works.
        let back_to_bug = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 2, "issue_type": bug})),
            )
            .await?;
        assert_eq!(ok(&back_to_bug)?["issue_type"]["name"], "Bug");
        let to_retired = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "issue_type": site.kind})),
            )
            .await?;
        assert_eq!(to_retired.code(), "issue.type-retired");
        // Offered again, the type takes issues again.
        let again = ada
            .send(
                "PATCH",
                &format!("{TYPES}/{}", site.kind),
                Some(json!({"version": 2, "retired": false})),
            )
            .await?;
        assert_eq!(ok(&again)?["retired"], false);
        ok(&site.create(&mut ada, "A task again").await?)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_02_only_people_who_manage_issue_types_create_and_change_them() -> TestResult {
    each_database("iss_02_manage_types", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let (mut lena, _) = site.person(StartingRole::ProjectLead, "lena").await?;
        // Everybody who is signed in reads the list.
        assert_eq!(tim.send("GET", TYPES, None).await?.status, StatusCode::OK);
        let body = json!({"name": "Question", "icon": "circle-help"});
        let refused = tim.send("POST", TYPES, Some(body.clone())).await?;
        assert_eq!(refused.code(), "access.forbidden");
        let made = lena.send("POST", TYPES, Some(body.clone())).await?;
        assert_eq!(ok(&made)?["name"], "Question");
        assert_eq!(made.body["icon"], "circle-help");
        let id = made.body["id"].as_str().ok_or("no id")?.to_owned();
        let taken = ada
            .send(
                "POST",
                TYPES,
                Some(json!({"name": "question", "icon": "bug"})),
            )
            .await?;
        assert_eq!(
            (taken.status, taken.code()),
            (StatusCode::CONFLICT, "name.taken")
        );
        let icon = ada
            .send(
                "PATCH",
                &format!("{TYPES}/{id}"),
                Some(json!({"version": 1, "name": "Ask", "icon": "lightbulb"})),
            )
            .await?;
        assert_eq!(ok(&icon)?["icon"], "lightbulb");
        assert_eq!(icon.body["version"], 2);
        let stale = ada
            .send(
                "PATCH",
                &format!("{TYPES}/{id}"),
                Some(json!({"version": 1, "name": "Old"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed");
        let unknown = json!({"name": "Odd", "icon": "not-an-icon"});
        // An icon outside the fixed list is not a valid request.
        assert_eq!(
            ada.send("POST", TYPES, Some(unknown)).await?.code(),
            "request.invalid"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_01_a_stale_change_after_undo_or_a_new_mode_names_who_counted_the_version_up()
-> TestResult {
    each_database("iss_01_stale_after_trash_and_mode", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut cora, _) = site.person(StartingRole::Contributor, "cora").await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let made = site.create(&mut cora, "Looked at version 1").await?;
        let path = format!("{ISSUES}/{}", key_in(ok(&made)?)?);
        // The trash and its undo count the version up (3), by Ada: no field of the issue changed.
        let deleted = ada.send("DELETE", &path, None).await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        ok(&ada.send("POST", &format!("{path}/restore"), None).await?)?;
        let late = tim
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "priority": "low"})),
            )
            .await?;
        assert_eq!(late.code(), "edit.version-changed");
        let conflict = &late.body["conflict"];
        assert_eq!(conflict["version"], 3, "{conflict}");
        assert_eq!(conflict["fields"], json!([]), "{conflict}");
        assert!(!conflict["changed_by"].is_null(), "{conflict}");
        // A new mode of IDs renews the key and counts the version up again (4).
        let switched = ada
            .send(
                "PUT",
                "/api/v1/projects/ACME/settings/issues.id-mode",
                Some(json!({"value": "counting"})),
            )
            .await?;
        ok(&switched)?;
        let key = key_in(&ok(&ada.send("GET", &path, None).await?)?["issue"])?;
        let late = tim
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 3, "priority": "low"})),
            )
            .await?;
        assert_eq!(late.code(), "edit.version-changed");
        let conflict = &late.body["conflict"];
        assert_eq!(conflict["version"], 4, "{conflict}");
        assert_eq!(conflict["fields"], json!([]), "{conflict}");
        assert!(!conflict["changed_by"].is_null(), "{conflict}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_02_two_administrators_renaming_the_same_type_get_who_and_which_field() -> TestResult {
    each_database("iss_02_stale_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut ben, _) = site.person(StartingRole::Administrator, "ben").await?;
        let path = format!("{TYPES}/{}", site.kind);
        let first = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "name": "Defect"})),
            )
            .await?;
        assert_eq!(ok(&first)?["version"], 2);
        let late = ben
            .send("PATCH", &path, Some(json!({"version": 1, "name": "Fault"})))
            .await?;
        assert_eq!(late.code(), "edit.version-changed");
        let conflict = &late.body["conflict"];
        assert_eq!(conflict["version"], 2, "{conflict}");
        assert_eq!(conflict["fields"], json!(["name"]), "{conflict}");
        assert_eq!(conflict["current"]["name"], "Defect", "{conflict}");
        assert!(conflict["changed_by"].is_object(), "{conflict}");
        Ok(())
    })
    .await
}
