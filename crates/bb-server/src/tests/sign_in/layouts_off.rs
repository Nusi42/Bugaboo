//! A field that is taken off a layout keeps the roles it had (CFG-01, ADR-0027, M3-F107): a lead
//! who may not decide what roles see takes a field off the form, saves, and puts it back, and the
//! field is shown to the roles it was shown to before, not to every role. Both databases.

use bb_app::access::Permission;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::reader_of;
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::layouts_http::{
    AN_EDITOR, ANOTHER_EDITOR, customize, editor_of, role_named, save, slot, take_lock,
};

/// What a lead who sees projects and manages issue types holds: nothing about roles or fields.
const LEAD: [Permission; 2] = [Permission::SeeProjects, Permission::ManageIssueTypes];

/// The sorted roles an editor answer shows for a field of its first section.
fn roles_of(editor: &Value, field: &str) -> Result<Vec<String>, Failure> {
    let row = editor["sections"][0]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["field"] == field)
        .ok_or("the field is not on the layout")?;
    let mut roles: Vec<String> = row["roles"]
        .as_array()
        .ok_or("no roles")?
        .iter()
        .filter_map(|role| role.as_str().map(str::to_owned))
        .collect();
    roles.sort();
    Ok(roles)
}

/// Saves `fields` as the one section of the layout of ACME, as `browser` with the editor `who`.
async fn save_fields(
    browser: &mut Browser,
    layout: &str,
    who: &str,
    fields: Value,
) -> Result<Value, Failure> {
    let editor = editor_of(browser, "ACME").await?;
    let (_, version) = slot(&editor)?;
    let taken = take_lock(browser, layout, who).await?;
    let token = taken["lock_token"].as_str().ok_or("no token")?.to_owned();
    let saved = save(
        browser,
        layout,
        &version,
        &token,
        json!([{"tab": "", "title": "", "fields": fields}]),
    )
    .await?;
    Ok(ok(&saved)?.clone())
}

#[tokio::test]
async fn cfg_01_a_field_put_back_keeps_its_roles() -> TestResult {
    each_database("cfg_01_field_put_back", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let editor = customize(&mut ada, "ACME").await?;
        let (team, guest) = (
            role_named(&editor, "Team member")?,
            role_named(&editor, "Guest")?,
        );
        let (layout, _) = slot(&editor)?;
        save_fields(
            &mut ada,
            &layout,
            AN_EDITOR,
            json!([
                {"field": "description", "roles": [&team, &guest]},
                {"field": "priority", "roles": [&team]},
            ]),
        )
        .await?;
        // The lead takes the priority off the form, and later puts it back. The lead cannot tick roles.
        let mut lena = reader_of(&site, &rig, "lena@example.test", &LEAD, &[]).await?;
        save_fields(
            &mut lena,
            &layout,
            ANOTHER_EDITOR,
            json!([{"field": "description", "roles": []}]),
        )
        .await?;
        let off = editor_of(&mut ada, "ACME").await?;
        assert!(
            roles_of(&off, "priority").is_err(),
            "the priority is off the form: {off}"
        );
        save_fields(
            &mut lena,
            &layout,
            ANOTHER_EDITOR,
            json!([
                {"field": "description", "roles": []},
                {"field": "priority", "roles": []},
            ]),
        )
        .await?;
        let back = editor_of(&mut ada, "ACME").await?;
        assert_eq!(roles_of(&back, "priority")?, [team]);
        Ok(())
    })
    .await
}
