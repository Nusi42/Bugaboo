//! Dashboards over HTTP on SQLite and PostgreSQL (CFG-03, ADR-0050): a dashboard is private when
//! it is made and shared like a saved search with *Share dashboards*, a dashboard that is not for a
//! person is not found, only its maker changes, shares or deletes it, *Home* is the person's own,
//! the limits hold, no access key reaches a route, and a removal keeps only the shared ones.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_policy::Permission;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::access_keys_http::a_key_is_turned_away_first;
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_of, ok, text_in};
use super::removal_http::{remove, switch_off};

pub(super) const DASHBOARDS: &str = "/api/v1/dashboards";

/// An ID of a dashboard that does not exist.
pub(super) const MISSING: &str = "01a3185c-5000-73b2-8f37-20757db0778e";

/// A box at `(column, row, width, height)` with the members of its source and options.
pub(super) fn a_box(
    kind: &str,
    title: &str,
    (column, row, width, height): (u32, u32, u32, u32),
    more: &Value,
) -> Value {
    let mut body = json!({
        "kind": kind,
        "title": title,
        "column": column,
        "row": row,
        "width": width,
        "height": height,
    });
    if let (Some(body), Some(more)) = (body.as_object_mut(), more.as_object()) {
        body.extend(more.clone());
    }
    body
}

/// *My timer* at the top left of row `row`.
pub(super) fn timer(row: u32) -> Value {
    a_box("my-timer", "My timer", (0, row, 3, 2), &json!({}))
}

/// *Search results* of `text`.
pub(super) fn results(title: &str, text: &str, place: (u32, u32, u32, u32)) -> Value {
    a_box(
        "search-results",
        title,
        place,
        &json!({ "search_text": text }),
    )
}

/// Creates a dashboard as `browser`; the answer.
pub(super) async fn create(
    browser: &mut Browser,
    name: &str,
    boxes: Vec<Value>,
) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            DASHBOARDS,
            Some(json!({"name": name, "boxes": boxes})),
        )
        .await
}

/// Shares a dashboard with a group or a project; the answer.
pub(super) async fn share(
    browser: &mut Browser,
    dashboard: &str,
    (kind, id): (&str, &str),
) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{DASHBOARDS}/{dashboard}/shares"),
            Some(json!({"kind": kind, "id": id})),
        )
        .await
}

/// Makes a project at the top; its ID.
pub(super) async fn project(browser: &mut Browser, short_name: &str) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": short_name, "name": short_name})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// The names of the dashboards of a list.
fn names(list: &Reply) -> Vec<String> {
    list.body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["name"].as_str().map(str::to_owned))
        .collect()
}

/// The status and the code of an answer, to compare a refusal with another.
fn refusal(reply: &Reply) -> (StatusCode, String) {
    (reply.status, reply.code().to_owned())
}

#[tokio::test]
async fn cfg_03_a_dashboard_is_private_when_made_and_shared_like_a_saved_search() -> TestResult {
    each_database("cfg_03_private", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let made = create(&mut ada, "Team", vec![timer(0)]).await?;
        let id = id_of(&made)?;
        assert_eq!(made.body["mine"], true, "{}", made.body);
        assert_eq!(made.body["home"], false);
        assert_eq!(made.body["shared_with"], json!([]));
        // Private: Bob is told nothing, in the words of a dashboard that is not there.
        let hidden = bob.send("GET", &format!("{DASHBOARDS}/{id}"), None).await?;
        let missing = bob
            .send("GET", &format!("{DASHBOARDS}/{MISSING}"), None)
            .await?;
        assert_eq!(hidden.status, StatusCode::NOT_FOUND);
        assert_eq!(refusal(&hidden), refusal(&missing));
        let shared_list = format!("{DASHBOARDS}?scope=shared");
        assert!(names(&bob.send("GET", &shared_list, None).await?).is_empty());
        // Shared with ACME, it is listed for Bob and opens; he is not told whom else it reaches.
        let shared = share(&mut ada, &id, ("project", &site.acme)).await?;
        assert_eq!(ok(&shared)?["shared_with"][0]["short_name"], "ACME");
        assert_eq!(names(&bob.send("GET", &shared_list, None).await?), ["Team"]);
        let opened = bob.send("GET", &format!("{DASHBOARDS}/{id}"), None).await?;
        assert_eq!(ok(&opened)?["mine"], false);
        assert_eq!(opened.body["shared_with"], json!([]));
        assert!(names(&bob.send("GET", DASHBOARDS, None).await?).is_empty());
        // A share with a group reaches the people in it, also where they see no project of it.
        let ops = project(&mut ada, "OPS").await?;
        let (mut cy, cy_id) = site
            .person_in(StartingRole::Contributor, "cy", &ops)
            .await?;
        let cy_path = format!("{DASHBOARDS}/{id}");
        assert_eq!(
            cy.send("GET", &cy_path, None).await?.status,
            StatusCode::NOT_FOUND
        );
        let group = ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Web"})))
            .await?;
        let group = id_of(&group)?;
        let joined = ada
            .send(
                "PUT",
                &format!("/api/v1/groups/{group}/members/{cy_id}"),
                None,
            )
            .await?;
        assert!(joined.status.is_success(), "{}", joined.body);
        ok(&share(&mut ada, &id, ("group", &group)).await?)?;
        assert_eq!(ok(&cy.send("GET", &cy_path, None).await?)?["name"], "Team");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_share_dashboards_is_needed_to_share_and_starts_with_the_roles_of_share_searches()
-> TestResult {
    // The roles that share searches at the start share dashboards, and no other role does.
    for role in StartingRole::ALL.iter().copied() {
        let holds = role.permissions();
        assert_eq!(
            holds.contains(&Permission::ShareSearches),
            holds.contains(&Permission::ShareDashboards),
            "{role:?}"
        );
    }
    each_database("cfg_03_share_dashboards", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _) = Site::start(&rig, &app).await?;
        let choices = format!("{DASHBOARDS}/share-choices?kind=project");
        // A guest of ACME does not hold the permission: it is refused, and ACME is no choice.
        let (mut guest, _) = site.person_in_acme(StartingRole::Guest, "gus").await?;
        let made = create(&mut guest, "Mine", vec![timer(0)]).await?;
        let refused = share(&mut guest, &id_of(&made)?, ("project", &site.acme)).await?;
        assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
        assert_eq!(refused.code(), "access.forbidden");
        assert_eq!(
            ok(&guest.send("GET", &choices, None).await?)?["items"],
            json!([])
        );
        // A viewer of ACME holds it from the start.
        let (mut viewer, _) = site.person_in_acme(StartingRole::Viewer, "vic").await?;
        let listed = viewer.send("GET", &choices, None).await?;
        assert_eq!(ok(&listed)?["items"][0]["short_name"], "ACME");
        let made = create(&mut viewer, "Ours", vec![timer(0)]).await?;
        ok(&share(&mut viewer, &id_of(&made)?, ("project", &site.acme)).await?)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_viewer_the_dashboard_does_not_reach_gets_not_found() -> TestResult {
    each_database("cfg_03_not_reached", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let ops = project(&mut ada, "OPS").await?;
        let made = create(&mut ada, "Ops", vec![timer(0)]).await?;
        let id = id_of(&made)?;
        let box_id = text_in(&made.body["boxes"][0], "id")?;
        ok(&share(&mut ada, &id, ("project", &ops)).await?)?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        for (method, tail, body) in [
            ("GET", String::new(), None),
            ("GET", format!("/boxes/{box_id}"), None),
            (
                "PATCH",
                String::new(),
                Some(json!({"version": 1, "name": "Mine"})),
            ),
            ("DELETE", String::new(), None),
            ("POST", "/restore".to_owned(), None),
            ("POST", "/copy".to_owned(), Some(json!({}))),
        ] {
            let reached = bob
                .send(method, &format!("{DASHBOARDS}/{id}{tail}"), body.clone())
                .await?;
            let missing = bob
                .send(method, &format!("{DASHBOARDS}/{MISSING}{tail}"), body)
                .await?;
            assert_eq!(reached.status, StatusCode::NOT_FOUND, "{method} {tail}");
            assert_eq!(refusal(&reached), refusal(&missing), "{method} {tail}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_only_the_maker_changes_shares_or_deletes_and_others_save_a_copy() -> TestResult {
    each_database("cfg_03_maker", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = create(&mut ada, "Team", vec![timer(0)]).await?;
        let id = id_of(&made)?;
        let path = format!("{DASHBOARDS}/{id}");
        ok(&share(&mut ada, &id, ("project", &site.acme)).await?)?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let changed = bob
            .send("PATCH", &path, Some(json!({"version": 1, "name": "Bob's"})))
            .await?;
        let shared = share(&mut bob, &id, ("project", &site.acme)).await?;
        let stopped = bob
            .send(
                "DELETE",
                &format!("{path}/shares/project/{}", site.acme),
                None,
            )
            .await?;
        let deleted = bob.send("DELETE", &path, None).await?;
        for refused in [&changed, &shared, &stopped, &deleted] {
            assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
            assert_eq!(refused.code(), "access.forbidden");
        }
        // Everyone who sees it may save a copy, which is private and their own.
        let copy = bob
            .send("POST", &format!("{path}/copy"), Some(json!({})))
            .await?;
        let copied = ok(&copy)?;
        assert_eq!(copied["mine"], true, "{copied}");
        assert_eq!(copied["name"], "Team");
        assert_eq!(copied["boxes"].as_array().map(Vec::len), Some(1));
        let copy_path = format!("{DASHBOARDS}/{}", text_in(copied, "id")?);
        assert_eq!(
            ada.send("GET", &copy_path, None).await?.status,
            StatusCode::NOT_FOUND
        );
        // The maker deletes it into the trash and takes it back.
        assert_eq!(
            ada.send("DELETE", &path, None).await?.status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            bob.send("GET", &path, None).await?.status,
            StatusCode::NOT_FOUND
        );
        let restored = ada.send("POST", &format!("{path}/restore"), None).await?;
        assert_eq!(ok(&restored)?["dashboard"]["name"], "Team");
        // Whoever may change organization settings deletes any, also one they do not see.
        let gone = ada.send("DELETE", &copy_path, None).await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_stale_change_of_a_dashboard_is_refused_with_the_newer_version() -> TestResult {
    each_database("cfg_03_stale", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_, mut ada) = Site::start(&rig, &app).await?;
        let made = create(&mut ada, "Team", vec![timer(0)]).await?;
        let path = format!("{DASHBOARDS}/{}", id_of(&made)?);
        let version = made.body["version"].clone();
        let first = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "name": "Ours"})),
            )
            .await?;
        assert_eq!(ok(&first)?["name"], "Ours");
        let stale = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "name": "Theirs"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert_eq!(stale.code(), "edit.version-changed");
        assert_eq!(stale.body["conflict"]["version"], first.body["version"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_limits_of_boxes_and_dashboards_are_counted_inside_the_change() -> TestResult {
    each_database("cfg_03_limits", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_, mut ada) = Site::start(&rig, &app).await?;
        let boxes = |count: u32| (0..count).map(|at| timer(at * 2)).collect::<Vec<_>>();
        let too_many = create(&mut ada, "Full", boxes(21)).await?;
        // Too many boxes has a code of its own, so the screen can tell it from a box that does
        // not check (M10-F55).
        assert_eq!(too_many.status, StatusCode::CONFLICT, "{}", too_many.body);
        assert_eq!(too_many.code(), "dashboard.too-many-boxes");
        // Boxes may not overlap either.
        let over = create(&mut ada, "Over", vec![timer(0), timer(1)]).await?;
        assert_eq!(over.code(), "request.invalid");
        ok(&create(&mut ada, "Full", boxes(20)).await?)?;
        // *Home* counts among the 50 of a person; the trash does not.
        ok(&ada.send("GET", &format!("{DASHBOARDS}/home"), None).await?)?;
        let mut last = String::new();
        for number in 3..=50 {
            last = id_of(&create(&mut ada, &format!("Board {number}"), Vec::new()).await?)?;
        }
        let refused = create(&mut ada, "One more", Vec::new()).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "limit.reached");
        let trashed = format!("{DASHBOARDS}/{last}");
        let deleted = ada.send("DELETE", &trashed, None).await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        ok(&create(&mut ada, "One more", Vec::new()).await?)?;
        // Taking one out of the trash past the limit is refused the same way.
        let restored = ada
            .send("POST", &format!("{trashed}/restore"), None)
            .await?;
        assert_eq!(restored.status, StatusCode::CONFLICT, "{}", restored.body);
        assert_eq!(restored.code(), "limit.reached");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_home_is_the_persons_own_dashboard_made_from_todays_blocks() -> TestResult {
    each_database("cfg_03_home", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let home_path = format!("{DASHBOARDS}/home");
        let home = ada.send("GET", &home_path, None).await?;
        let first = ok(&home)?;
        assert_eq!(first["home"], true, "{first}");
        let kinds_and_titles: Vec<(Value, Value)> = first["boxes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|each| (each["kind"].clone(), each["title"].clone()))
            .collect();
        // *My timer* and *Due soon* side by side, *My open issues* under them, the whole width.
        assert_eq!(
            kinds_and_titles,
            [
                (json!("my-timer"), json!("My timer")),
                (json!("search-results"), json!("Due soon")),
                (json!("search-results"), json!("My open issues")),
            ]
        );
        assert_eq!(
            first["boxes"][2]["width"],
            json!(bb_core::dashboard::GRID_COLUMNS),
            "{first}"
        );
        let again = ada.send("GET", &home_path, None).await?;
        assert_eq!(ok(&again)?["id"], first["id"], "made once");
        assert!(names(&ada.send("GET", DASHBOARDS, None).await?).is_empty());
        let id = text_in(first, "id")?;
        // Nobody else reaches it, and it is never shared or deleted.
        let (mut bob, _) = site
            .person_called(StartingRole::Administrator, "bob", "Bob Baker")
            .await?;
        let path = format!("{DASHBOARDS}/{id}");
        assert_eq!(
            bob.send("GET", &path, None).await?.status,
            StatusCode::NOT_FOUND
        );
        let shared = share(&mut ada, &id, ("project", &site.acme)).await?;
        assert_eq!(shared.status, StatusCode::FORBIDDEN);
        assert_eq!(
            ada.send("DELETE", &path, None).await?.status,
            StatusCode::FORBIDDEN
        );
        // *Customize* is its change.
        let customized = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": first["version"], "boxes": [timer(0)]})),
            )
            .await?;
        assert_eq!(ok(&customized)?["boxes"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_an_access_key_reaches_no_dashboard_route() -> TestResult {
    each_database("cfg_03_keys", async |db| {
        let one = format!("{DASHBOARDS}/{MISSING}");
        a_key_is_turned_away_first(
            db,
            &[
                ("GET", DASHBOARDS.to_owned(), None),
                ("POST", DASHBOARDS.to_owned(), Some(json!({"name": "Mine"}))),
                ("GET", format!("{DASHBOARDS}/home"), None),
                ("GET", format!("{DASHBOARDS}/ready-made-boxes"), None),
                (
                    "GET",
                    format!("{DASHBOARDS}/share-choices?kind=group"),
                    None,
                ),
                ("POST", format!("{DASHBOARDS}/box-preview"), Some(timer(0))),
                ("GET", one.clone(), None),
                ("PATCH", one.clone(), Some(json!({"version": 1}))),
                ("DELETE", one.clone(), None),
                ("POST", format!("{one}/restore"), None),
                ("POST", format!("{one}/copy"), Some(json!({}))),
                (
                    "POST",
                    format!("{one}/shares"),
                    Some(json!({"kind": "group", "id": MISSING})),
                ),
                ("DELETE", format!("{one}/shares/group/{MISSING}"), None),
                ("GET", format!("{one}/boxes/{MISSING}"), None),
            ],
        )
        .await
    })
    .await
}

#[tokio::test]
async fn priv_04_removal_deletes_private_dashboards_and_keeps_shared_ones_as_former_user()
-> TestResult {
    each_database("priv_04_dashboards", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let private = id_of(&create(&mut bob, "Private", vec![timer(0)]).await?)?;
        let shared = id_of(&create(&mut bob, "Shared", vec![timer(0)]).await?)?;
        ok(&share(&mut bob, &shared, ("project", &site.acme)).await?)?;
        ok(&bob.send("GET", &format!("{DASHBOARDS}/home"), None).await?)?;
        switch_off(&mut ada, bob_id).await?;
        // The plan says what goes (the private one and *Home*) and what stays.
        let plan = ada
            .send(
                "GET",
                &format!("/api/v1/people/{bob_id}/removal-plan"),
                None,
            )
            .await?;
        let rows_of = |list: &str| {
            plan.body[list]
                .as_array()
                .into_iter()
                .flatten()
                .find(|kind| kind["title"] == "Dashboards")
                .map(|kind| kind["rows"].clone())
        };
        assert_eq!(rows_of("deleted_now"), Some(json!(2)), "{}", plan.body);
        assert_eq!(
            rows_of("kept_as_former_user"),
            Some(json!(1)),
            "{}",
            plan.body
        );
        // The boxes go and stay with their dashboards, so the plan does not count them apart.
        assert!(
            !plan.body.to_string().contains("Boxes of dashboards"),
            "{}",
            plan.body
        );
        let done = remove(&mut ada, bob_id, json!({"comments": "keep"})).await?;
        assert_eq!(done["status"], 204, "{done}");
        // The shared one stays, made by a former user; the private one is gone.
        let opened = ada
            .send("GET", &format!("{DASHBOARDS}/{shared}"), None)
            .await?;
        assert_eq!(ok(&opened)?["maker"]["shown"], "former", "{}", opened.body);
        let gone = ada
            .send("DELETE", &format!("{DASHBOARDS}/{private}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NOT_FOUND, "{}", gone.body);
        let kept = ada
            .send("DELETE", &format!("{DASHBOARDS}/{shared}"), None)
            .await?;
        assert_eq!(kept.status, StatusCode::NO_CONTENT, "{}", kept.body);
        Ok(())
    })
    .await
}
