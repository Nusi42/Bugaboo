//! Layouts over HTTP, the cases the review of M3 WP-09 asked for, on both databases (CFG-01,
//! ISS-17, ADR-0027, ADR-0019):
//!
//! - A lead who may not know an own field never reads its ID, not even in their draft, and a save
//!   of theirs keeps the field where it was.
//! - A layout is not taken away while somebody else is editing it; the editor's own lock goes with it.
//! - A person who may not decide what roles see gets no role names and no marks, and a save keeps
//!   the roles of the fields that are on the layout.
//! - A place with more own fields than a hundred can be customized and saved.
//! - A required field with a default is not asked for on the form, as the server does not ask.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_core::layout::{LayoutKind, built_in_fields};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{create_field, reader_of};
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, id_in, ok};
use super::layouts_http::{
    AN_EDITOR, ANOTHER_EDITOR, LAYOUTS, customize, editor_of, form_of, role_named, save, slot,
    take_lock,
};

/// What a lead who sees projects and manages issue types holds: nothing about roles or fields.
const LEAD: [Permission; 2] = [Permission::SeeProjects, Permission::ManageIssueTypes];

/// The token of an answer that took the lock.
fn token_of(taken: &Value) -> Result<String, Failure> {
    Ok(taken["lock_token"].as_str().ok_or("no token")?.to_owned())
}

/// The draft the person keeps for a layout, as the text.
async fn draft_of(browser: &mut Browser, layout: &str) -> Result<Option<String>, Failure> {
    let slot = browser
        .send(
            "GET",
            &format!("/api/v1/drafts?kind=layout&id={layout}&part=layout"),
            None,
        )
        .await?;
    Ok(ok(&slot)?["draft"]["text"].as_str().map(str::to_owned))
}

#[tokio::test]
async fn cfg_01_a_lead_who_may_not_know_a_field_never_reads_its_id_and_keeps_it_on_save()
-> TestResult {
    each_database("cfg_01_unknown_field", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let salary = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Salary band", "kind": "text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let editor = customize(&mut ada, "ACME").await?;
        let team = role_named(&editor, "Team member")?;
        let (layout, version) = slot(&editor)?;
        let lock = token_of(&take_lock(&mut ada, &layout, AN_EDITOR).await?)?;
        ok(&save(
            &mut ada,
            &layout,
            &version,
            &lock,
            json!([{"tab": "", "title": "", "fields": [
                {"field": "description", "roles": [&team]},
                {"field": &salary, "roles": [&team]},
            ]}]),
        )
        .await?)?;

        // The lead may not see the field anywhere: the editor does not know it.
        let mut lena = reader_of(
            &site,
            &rig,
            "lena@example.test",
            &LEAD,
            &[(&salary, None, FieldLevel::Hidden)],
        )
        .await?;
        let seen = editor_of(&mut lena, "ACME").await?;
        assert!(!seen.to_string().contains(&salary), "{seen}");
        let (_, current) = slot(&seen)?;

        // A save on an old version is refused; what Lena sent is her draft, and the draft holds
        // nothing of the field she cannot know.
        let lena_lock = token_of(&take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?)?;
        let stale = save(
            &mut lena,
            &layout,
            &version,
            &lena_lock,
            json!([{"tab": "", "title": "Mine", "fields": [{"field": "description", "roles": []}]}]),
        )
        .await?;
        assert_eq!(stale.code(), "edit.version_changed", "{}", stale.body);
        let draft = draft_of(&mut lena, &layout).await?.ok_or("no draft")?;
        assert!(draft.contains("Mine"), "{draft}");
        assert!(!draft.contains(&salary), "{draft}");

        // A save on the current version goes through, and the field is still on the layout.
        let lena_lock = token_of(&take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?)?;
        ok(&save(
            &mut lena,
            &layout,
            &current,
            &lena_lock,
            json!([{"tab": "", "title": "Mine", "fields": [{"field": "description", "roles": []}]}]),
        )
        .await?)?;
        let after = editor_of(&mut ada, "ACME").await?;
        assert_eq!(after["sections"][0]["title"], "Mine");
        assert!(after.to_string().contains(&salary), "{after}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_layout_is_not_taken_away_while_somebody_else_edits_it() -> TestResult {
    each_database("iss_17_delete_and_lock", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lena, _) = site.person(StartingRole::ProjectLead, "lena").await?;
        let editor = customize(&mut ada, "ACME").await?;
        let (layout, version) = slot(&editor)?;
        let delete = format!("{LAYOUTS}/{layout}?version={version}");

        // Lena is editing: Ada is told so and the layout stays.
        let held = take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?;
        assert_eq!(held["status"], "taken");
        let refused = ada.send("DELETE", &delete, None).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "edit.in_progress");
        assert!(!editor_of(&mut ada, "ACME").await?["layout"].is_null());
        // Ada's reading of the lock names Lena, as the reader may know her.
        let asked = take_lock(&mut ada, &layout, AN_EDITOR).await?;
        assert_eq!(asked["status"], "held");

        // Lena gives up her lock; Ada takes one of her own, and taking the layout away lets it go.
        let released = lena
            .send(
                "POST",
                &format!("{}/release", super::layouts_http::LOCKS),
                Some(json!({
                    "what": {"record": {"kind": "layout", "id": layout}, "part": "layout"},
                    "lock_token": token_of(&held)?,
                })),
            )
            .await?;
        assert!(released.status.is_success(), "{}", released.body);
        let own = take_lock(&mut ada, &layout, AN_EDITOR).await?;
        assert_eq!(own["status"], "taken");
        let gone = ada.send("DELETE", &delete, None).await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        assert!(editor_of(&mut ada, "ACME").await?["layout"].is_null());
        // Nothing holds the layout's place any more: a new layout there can be locked at once.
        let again = customize(&mut ada, "ACME").await?;
        let (second, _) = slot(&again)?;
        assert_eq!(
            take_lock(&mut lena, &second, ANOTHER_EDITOR).await?["status"],
            "taken"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_01_a_lead_who_may_not_decide_what_roles_see_gets_no_roles_and_keeps_them() -> TestResult
{
    each_database("cfg_01_roles_need_permission", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let editor = customize(&mut ada, "ACME").await?;
        assert_eq!(editor["choose_roles"], true);
        let (team, guest) = (
            role_named(&editor, "Team member")?,
            role_named(&editor, "Guest")?,
        );
        let (layout, version) = slot(&editor)?;
        let lock = token_of(&take_lock(&mut ada, &layout, AN_EDITOR).await?)?;
        ok(&save(
            &mut ada,
            &layout,
            &version,
            &lock,
            json!([{"tab": "", "title": "", "fields": [
                {"field": "description", "roles": [&team, &guest]},
                {"field": "priority", "roles": [&team]},
            ]}]),
        )
        .await?)?;

        // The lead sees fields and sections, but no role name, no row of roles and no mark.
        let mut lena = reader_of(&site, &rig, "lena@example.test", &LEAD, &[]).await?;
        let seen = editor_of(&mut lena, "ACME").await?;
        assert_eq!(seen["choose_roles"], false);
        assert_eq!(seen["roles"], json!([]));
        for row in seen["sections"][0]["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(seen["available"].as_array().into_iter().flatten())
        {
            assert_eq!(row["roles"], json!([]), "{row}");
            assert_eq!(row["limited_for"], json!([]), "{row}");
        }
        assert!(!seen.to_string().contains("Guest"), "{seen}");

        // The lead moves the priority out and adds the assignee. The roles of the description and
        // the priority stay as they were; the field that is new is shown to every role.
        let (_, current) = slot(&seen)?;
        let lena_lock = token_of(&take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?)?;
        ok(&save(
            &mut lena,
            &layout,
            &current,
            &lena_lock,
            json!([{"tab": "", "title": "", "fields": [
                {"field": "priority", "roles": []},
                {"field": "description", "roles": []},
                {"field": "assigned-to", "roles": []},
            ]}]),
        )
        .await?)?;
        let after = editor_of(&mut ada, "ACME").await?;
        let rows = after["sections"][0]["fields"].as_array().ok_or("no rows")?;
        let roles_of = |field: &str| -> Result<Vec<String>, Failure> {
            let row = rows
                .iter()
                .find(|row| row["field"] == field)
                .ok_or("no row")?;
            let mut roles: Vec<String> = row["roles"]
                .as_array()
                .ok_or("no roles")?
                .iter()
                .filter_map(|role| role.as_str().map(str::to_owned))
                .collect();
            roles.sort();
            Ok(roles)
        };
        let mut both = vec![team.clone(), guest.clone()];
        both.sort();
        assert_eq!(roles_of("description")?, both);
        assert_eq!(roles_of("priority")?, [team]);
        let mut every: Vec<String> = after["roles"]
            .as_array()
            .ok_or("no roles")?
            .iter()
            .filter_map(|role| role["id"].as_str().map(str::to_owned))
            .collect();
        every.sort();
        assert_eq!(roles_of("assigned-to")?, every);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_01_a_place_with_many_own_fields_can_be_customized_and_saved() -> TestResult {
    each_database("cfg_01_many_own_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        // More than a hundred, which was the most a layout held once.
        for number in 0..101 {
            create_field(
                &mut ada,
                json!({"name": format!("Field {number}"), "kind": "text", "applies_to": "issues"}),
            )
            .await?;
        }
        let made = ada
            .send(
                "POST",
                LAYOUTS,
                Some(json!({"project": "ACME", "kind": "issue-page"})),
            )
            .await?;
        let editor = ok(&made)?.clone();
        let placed: usize = editor["sections"]
            .as_array()
            .ok_or("no sections")?
            .iter()
            .map(|section| section["fields"].as_array().map_or(0, Vec::len))
            .sum();
        assert_eq!(
            placed,
            101 + built_in_fields(LayoutKind::IssuePage).len(),
            "{editor}"
        );
        // A save of the whole layout is not refused for its size.
        let (layout, version) = slot(&editor)?;
        let lock = token_of(&take_lock(&mut ada, &layout, AN_EDITOR).await?)?;
        let sections: Vec<Value> = editor["sections"]
            .as_array()
            .ok_or("no sections")?
            .iter()
            .map(|section| {
                json!({
                    "tab": section["tab"],
                    "title": section["title"],
                    "fields": section["fields"].as_array().into_iter().flatten()
                        .map(|row| json!({"field": row["field"], "roles": row["roles"]}))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        let saved = save(&mut ada, &layout, &version, &lock, json!(sections)).await?;
        ok(&saved)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_03_a_required_field_with_a_default_is_not_asked_for_on_the_form() -> TestResult {
    each_database("cf_03_required_with_default", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let asked = id_in(
            &create_field(
                &mut ada,
                json!({
                    "name": "Customer", "kind": "text", "applies_to": "issues",
                    "rules": {"required": true},
                }),
            )
            .await?,
        )?;
        let defaulted = id_in(
            &create_field(
                &mut ada,
                json!({
                    "name": "Region", "kind": "text", "applies_to": "issues",
                    "rules": {"required": true}, "default_value": "North",
                }),
            )
            .await?,
        )?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let editor = customize(&mut ada, "ACME").await?;
        let (layout, version) = slot(&editor)?;
        let team = role_named(&editor, "Team member")?;
        let lock = token_of(&take_lock(&mut ada, &layout, AN_EDITOR).await?)?;
        let saved = save(
            &mut ada,
            &layout,
            &version,
            &lock,
            json!([{"tab": "", "title": "", "fields": [{"field": "description", "roles": [&team]}]}]),
        )
        .await?;
        let saved = ok(&saved)?;
        // The form keeps the field that must be asked for, and not the one that has a default.
        assert_eq!(
            form_of(&mut tim, "ACME", "new-issue").await?,
            ["description", asked.as_str()]
        );
        // The editor marks the same field as required.
        let mark = |field: &str| -> Option<bool> {
            saved["available"]
                .as_array()?
                .iter()
                .find(|row| row["field"] == field)
                .and_then(|row| row["required"].as_bool())
        };
        assert_eq!(mark(&asked), Some(true));
        assert_eq!(mark(&defaulted), Some(false));
        Ok(())
    })
    .await
}
