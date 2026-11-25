//! More of dashboards over HTTP on SQLite and PostgreSQL (CFG-03, ADR-0050): *Home* opens for every
//! person whatever they may see, the trash does not count against the limit, a box is named once,
//! a copy and a viewer get only the boxes they may read, a change keeps the boxes its maker cannot
//! see now, and two changes at the same moment pass the limits only once.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::dashboards_boxes_http::chart;
use super::dashboards_http::{DASHBOARDS, create, project, results, share, timer};
use super::http::{ADA_EMAIL, Browser, rig_and_app};
use super::issues_site::{Site, id_of, ok, text_in};

/// The titles of the boxes of a dashboard.
fn box_titles(answer: &Value) -> Vec<String> {
    answer["boxes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|each| each["title"].as_str().map(str::to_owned))
        .collect()
}

/// Sets the level of a field for a role everywhere, as an administrator.
async fn field_level(
    site: &Site<'_>,
    admin: &mut Browser,
    (role, field, level): (StartingRole, &str, &str),
) -> Result<(), Failure> {
    let set = admin
        .send(
            "PUT",
            &format!("/api/v1/field-permissions/{}/{field}", site.role_id(role)),
            Some(json!({"level": level})),
        )
        .await?;
    assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
    Ok(())
}

#[tokio::test]
async fn cfg_03_home_opens_for_a_guest_hr_and_a_person_whose_priority_is_hidden() -> TestResult {
    each_database("cfg_03_home_for_all", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let home = format!("{DASHBOARDS}/home");
        // A guest reads no *Due date* and no *Priority* at the start: *Due soon* and "My open
        // issues" are kept and left out of what they see; their *Home* opens with the timer.
        let (mut guest, _) = site.person_in_acme(StartingRole::Guest, "gus").await?;
        let guests = guest.send("GET", &home, None).await?;
        assert_eq!(box_titles(ok(&guests)?), ["My timer"], "{}", guests.body);
        let (mut hr, _) = site
            .person_called(StartingRole::Hr, "hal", "Hal Hart")
            .await?;
        ok(&hr.send("GET", &home, None).await?)?;
        // Without *Priority*, "My open issues" waits until the person may read it again.
        field_level(
            &site,
            &mut ada,
            (StartingRole::TeamMember, "priority", "hidden"),
        )
        .await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let hidden = bob.send("GET", &home, None).await?;
        assert!(
            !box_titles(ok(&hidden)?).contains(&"My open issues".to_owned()),
            "{}",
            hidden.body
        );
        field_level(
            &site,
            &mut ada,
            (StartingRole::TeamMember, "priority", "can-see"),
        )
        .await?;
        let shown = bob.send("GET", &home, None).await?;
        assert_eq!(
            box_titles(ok(&shown)?),
            ["My timer", "My open issues", "Due soon"]
        );
        assert_eq!(shown.body["id"], hidden.body["id"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_box_is_named_once_in_a_change() -> TestResult {
    each_database("cfg_03_box_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_, mut ada) = Site::start(&rig, &app).await?;
        let made = create(&mut ada, "Team", vec![timer(0)]).await?;
        let box_id = text_in(&made.body["boxes"][0], "id")?;
        let twice = |row: u32| {
            let mut each = timer(row);
            each["id"] = json!(box_id);
            each
        };
        let changed = ada
            .send(
                "PATCH",
                &format!("{DASHBOARDS}/{}", id_of(&made)?),
                Some(json!({"version": made.body["version"], "boxes": [twice(0), twice(2)]})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::BAD_REQUEST, "{}", changed.body);
        assert_eq!(changed.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_copy_and_a_viewer_get_only_the_boxes_they_may_read() -> TestResult {
    each_database("cfg_03_copy_and_sources", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        project(&mut ada, "OPS").await?;
        let saved = ada
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({"name": "Mine only", "search_text": ""})),
            )
            .await?;
        let from_saved = super::dashboards_http::a_box(
            "search-results",
            "Of my search",
            (0, 2, 6, 2),
            &json!({"saved_search": id_of(&saved)?}),
        );
        let mut by_priority = chart("", Some("priority"));
        by_priority["row"] = json!(4);
        by_priority["title"] = json!("By priority");
        let elsewhere = results("Ops", "project is OPS", (0, 8, 6, 2));
        let made = create(
            &mut ada,
            "Team",
            vec![timer(0), from_saved.clone(), by_priority.clone(), elsewhere],
        )
        .await?;
        let id = id_of(&made)?;
        ok(&share(&mut ada, &id, ("project", &site.acme)).await?)?;
        field_level(
            &site,
            &mut ada,
            (StartingRole::TeamMember, "priority", "hidden"),
        )
        .await?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        // The frame: only the timer, at the top.
        let frame = bob.send("GET", &format!("{DASHBOARDS}/{id}"), None).await?;
        assert_eq!(box_titles(ok(&frame)?), ["My timer"]);
        // The box routes and the preview refuse the others as for anything that is not there.
        for number in 1..=3 {
            let each = text_in(&made.body["boxes"][number], "id")?;
            let read = bob
                .send("GET", &format!("{DASHBOARDS}/{id}/boxes/{each}"), None)
                .await?;
            assert_eq!(
                read.status,
                StatusCode::NOT_FOUND,
                "{number}: {}",
                read.body
            );
        }
        for asked in [from_saved, by_priority] {
            let preview = bob
                .send("POST", &format!("{DASHBOARDS}/box-preview"), Some(asked))
                .await?;
            assert_eq!(preview.status, StatusCode::BAD_REQUEST, "{}", preview.body);
        }
        // A copy holds what Bob sees, and nothing else.
        let copy = bob
            .send("POST", &format!("{DASHBOARDS}/{id}/copy"), Some(json!({})))
            .await?;
        assert_eq!(box_titles(ok(&copy)?), ["My timer"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_change_keeps_the_boxes_its_maker_cannot_see_now() -> TestResult {
    each_database("cfg_03_keeps_unseen", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let mut by_priority = chart("", Some("priority"));
        by_priority["title"] = json!("By priority");
        let made = create(&mut bob, "Team", vec![by_priority, timer(4)]).await?;
        let path = format!("{DASHBOARDS}/{}", id_of(&made)?);
        field_level(
            &site,
            &mut ada,
            (StartingRole::TeamMember, "priority", "hidden"),
        )
        .await?;
        let frame = bob.send("GET", &path, None).await?;
        let seen = ok(&frame)?;
        assert_eq!(box_titles(seen), ["My timer"]);
        let mut kept = seen["boxes"][0].clone();
        if let Some(fields) = kept.as_object_mut() {
            fields.retain(|_, value| !value.is_null());
        }
        let added = results("Open", "status group is not Done", (3, 0, 6, 2));
        let changed = bob
            .send(
                "PATCH",
                &path,
                Some(json!({"version": seen["version"], "boxes": [kept, added]})),
            )
            .await?;
        assert_eq!(box_titles(ok(&changed)?), ["My timer", "Open"]);
        // The chart was kept; it shows again, below the boxes named in the change.
        field_level(
            &site,
            &mut ada,
            (StartingRole::TeamMember, "priority", "can-see"),
        )
        .await?;
        let again = bob.send("GET", &path, None).await?;
        let all = ok(&again)?;
        assert_eq!(box_titles(all), ["My timer", "Open", "By priority"]);
        assert_eq!(all["boxes"][2]["row"], json!(2), "{all}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_two_changes_at_once_pass_the_limits_only_once() -> TestResult {
    each_database("cfg_03_at_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_, mut ada) = Site::start(&rig, &app).await?;
        // Two first visits of *Home* at once make one *Home*.
        let mut first = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut second = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let home = format!("{DASHBOARDS}/home");
        let (one, other) = tokio::join!(
            first.send("GET", &home, None),
            second.send("GET", &home, None)
        );
        let (one, other) = (one?, other?);
        assert_eq!(ok(&one)?["id"], ok(&other)?["id"]);
        // With *Home*, 49 dashboards: two made at once, and only one of them is.
        for number in 2..50 {
            ok(&create(&mut ada, &format!("Board {number}"), Vec::new()).await?)?;
        }
        let (one, other) = tokio::join!(
            create(&mut first, "Last", Vec::new()),
            create(&mut second, "Last too", Vec::new())
        );
        let mut answers = [one?.status, other?.status];
        answers.sort();
        assert_eq!(answers, [StatusCode::OK, StatusCode::CONFLICT]);
        Ok(())
    })
    .await
}
