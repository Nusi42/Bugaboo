//! Layouts over HTTP on both databases (CFG-01, ISS-02, ISS-17, ADR-0027): a Guest gets the short
//! form and a Team member the full one, field permissions win over a layout, a sub-project uses
//! its parent's layout until it has its own, and two people cannot change one layout at once.
//!
//! In plain words (the acceptance tests of M3 WP-09):
//! - *A Guest gets the short form and a Team member the full one.* The layout of the new-issue
//!   form lists the description for everybody, and the priority, who is assigned and a due date
//!   for Team members only. The Guest is asked for the description alone; the Team member for all
//!   four.
//! - *A layout never shows a field the role cannot see.* A layout that lists an own field for the
//!   Guest does not show it to a Guest whose level for it is *Hidden*, and the editor marks the
//!   row "Guest: field permissions" so the person who edits knows why.
//! - *A sub-project uses its parent's layout until it has its own.* The parent's layout reaches
//!   the sub-project; once the sub-project makes its own, the parent's is not touched.
//! - *Two people cannot change one layout at once.* The second editor is told who is editing, and
//!   a save without the lock is refused.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::access::FieldLevel;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{create_field, set_levels};
use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_in, ok};

pub(super) const LAYOUTS: &str = "/api/v1/layouts";
pub(super) const LOCKS: &str = "/api/v1/edit-locks";

/// An open editor, as the browser makes its ID.
pub(super) const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";
/// Another open editor.
pub(super) const ANOTHER_EDITOR: &str = "0b1c2d3e-4f5a-4b6c-8d7e-9f0a1b2c3d4e";

/// The editor view of the layout of the new-issue form of a project.
pub(super) async fn editor_of(browser: &mut Browser, project: &str) -> Result<Value, Failure> {
    let editor = browser
        .send(
            "GET",
            &format!("{LAYOUTS}/editor?project={project}&kind=new-issue"),
            None,
        )
        .await?;
    Ok(ok(&editor)?.clone())
}

/// Creates the layout of the new-issue form of a project (*Customize this form*); the editor view.
pub(super) async fn customize(browser: &mut Browser, project: &str) -> Result<Value, Failure> {
    let made = browser
        .send(
            "POST",
            LAYOUTS,
            Some(json!({"project": project, "kind": "new-issue"})),
        )
        .await?;
    Ok(ok(&made)?.clone())
}

/// The ID of a role of the editor by its name.
pub(super) fn role_named(editor: &Value, name: &str) -> Result<String, Failure> {
    editor["roles"]
        .as_array()
        .and_then(|roles| roles.iter().find(|role| role["name"] == name))
        .and_then(|role| role["id"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("no role {name}").into())
}

/// Takes the edit lock on a layout as `browser`; the answer.
pub(super) async fn take_lock(
    browser: &mut Browser,
    layout: &str,
    editor: &str,
) -> Result<Value, Failure> {
    let taken = browser
        .send(
            "POST",
            LOCKS,
            Some(json!({
                "target": {"record": {"kind": "layout", "id": layout}, "part": "layout"},
                "editor_id": editor,
            })),
        )
        .await?;
    Ok(ok(&taken)?.clone())
}

/// The keys of the fields a person is shown on the form of a project: all sections in order.
pub(super) async fn form_of(
    browser: &mut Browser,
    project: &str,
    kind: &str,
) -> Result<Vec<String>, Failure> {
    let form = browser
        .send(
            "GET",
            &format!("{LAYOUTS}/form?project={project}&kind={kind}"),
            None,
        )
        .await?;
    Ok(ok(&form)?["sections"]
        .as_array()
        .ok_or("no sections")?
        .iter()
        .flat_map(|section| section["fields"].as_array().into_iter().flatten())
        .filter_map(|field| field.as_str().map(str::to_owned))
        .collect())
}

/// Saves the sections of a layout under the lock; the reply.
pub(super) async fn save(
    browser: &mut Browser,
    layout: &str,
    version: &Value,
    token: &str,
    sections: Value,
) -> Result<super::http::Reply, Failure> {
    browser
        .send(
            "PUT",
            &format!("{LAYOUTS}/{layout}"),
            Some(json!({"version": version, "lock_token": token, "sections": sections})),
        )
        .await
}

pub(super) fn slot(editor: &Value) -> Result<(String, Value), Failure> {
    Ok((
        editor["layout"]["id"]
            .as_str()
            .ok_or("the place has no layout of its own")?
            .to_owned(),
        editor["layout"]["version"].clone(),
    ))
}

#[tokio::test]
async fn cfg_01_a_guest_gets_the_short_form_and_a_team_member_the_full_one() -> TestResult {
    each_database("cfg_01_short_and_full", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let due = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Due date", "kind": "date", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        // Before anybody customizes the form, everybody is asked for the same fields (the ones
        // their level lets them fill in).
        let before_guest = form_of(&mut gus, "ACME", "new-issue").await?;
        assert_eq!(before_guest, ["description"]);
        assert_eq!(
            form_of(&mut tim, "ACME", "new-issue").await?,
            ["description", "priority"]
        );

        // The lead customizes the form: the description for everybody; the priority, who is
        // assigned and the due date for Team members only.
        let editor = customize(&mut ada, "ACME").await?;
        let team = role_named(&editor, "Team member")?;
        let everybody: Vec<&str> = editor["roles"]
            .as_array()
            .ok_or("no roles")?
            .iter()
            .filter_map(|role| role["id"].as_str())
            .collect();
        let (layout, version) = slot(&editor)?;
        let lock = take_lock(&mut ada, &layout, AN_EDITOR).await?;
        let token = lock["lock_token"].as_str().ok_or("no token")?.to_owned();
        let saved = save(
            &mut ada,
            &layout,
            &version,
            &token,
            json!([{
                "tab": "",
                "title": "About the issue",
                "fields": [
                    {"field": "description", "roles": everybody},
                    {"field": "priority", "roles": [&team]},
                    {"field": "assigned-to", "roles": [&team]},
                    {"field": &due, "roles": [&team]},
                ],
            }]),
        )
        .await?;
        let saved = ok(&saved)?;
        assert_eq!(saved["layout"]["version"], 2);

        // The Guest is asked for the description alone, the Team member for all four.
        assert_eq!(
            form_of(&mut gus, "ACME", "new-issue").await?,
            ["description"]
        );
        assert_eq!(
            form_of(&mut tim, "ACME", "new-issue").await?,
            ["description", "priority", "assigned-to", due.as_str()]
        );
        // The layout of the new-issue form does not touch the issue page.
        assert!(
            form_of(&mut gus, "ACME", "issue-page")
                .await?
                .contains(&"priority".to_owned())
        );
        // The Guest is not let into the editor.
        let refused = gus
            .send(
                "GET",
                &format!("{LAYOUTS}/editor?project=ACME&kind=new-issue"),
                None,
            )
            .await?;
        assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_01_a_layout_never_shows_a_field_the_role_cannot_see_and_the_editor_says_so()
-> TestResult {
    each_database("cfg_01_field_permissions_win", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let cost = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Cost center", "kind": "text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let editor = customize(&mut ada, "ACME").await?;
        let (team, guest) = (
            role_named(&editor, "Team member")?,
            role_named(&editor, "Guest")?,
        );
        // The Guest role may not see the field, whatever a layout says.
        let guest_role: bb_core::id::RoleId = guest.parse().map_err(|_| "no role ID")?;
        set_levels(
            &site,
            &rig,
            guest_role,
            &[(&cost, None, FieldLevel::Hidden)],
        )
        .await?;
        let (layout, version) = slot(&editor)?;
        let token = take_lock(&mut ada, &layout, AN_EDITOR).await?["lock_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned();
        let saved = save(
            &mut ada,
            &layout,
            &version,
            &token,
            json!([{
                "tab": "",
                "title": "",
                "fields": [{"field": &cost, "roles": [&team, &guest]}],
            }]),
        )
        .await?;
        let saved = ok(&saved)?;
        // The row is marked for the Guest (who may not see the field) and not for the Team member.
        let row = &saved["sections"][0]["fields"][0];
        assert_eq!(row["name"], "Cost center");
        let limited = row["limited_for"].as_array().ok_or("no marks")?;
        assert!(limited.contains(&json!(guest)));
        assert!(!limited.contains(&json!(team)));

        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        // The layout lists the field for the Guest, and the Guest is not shown it.
        assert!(form_of(&mut gus, "ACME", "new-issue").await?.is_empty());
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        assert_eq!(
            form_of(&mut tim, "ACME", "new-issue").await?,
            [cost.as_str()]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_01_a_sub_project_uses_its_parents_layout_until_it_has_its_own() -> TestResult {
    each_database("cfg_01_sub_project", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "WEB", "name": "Web"})),
            )
            .await?;
        ok(&made)?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;

        // The parent takes the priority off the form.
        let editor = customize(&mut ada, "ACME").await?;
        let team = role_named(&editor, "Team member")?;
        let (layout, version) = slot(&editor)?;
        let token = take_lock(&mut ada, &layout, AN_EDITOR).await?["lock_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned();
        ok(&save(
            &mut ada,
            &layout,
            &version,
            &token,
            json!([{"tab": "", "title": "", "fields": [{"field": "description", "roles": [&team]}]}]),
        )
        .await?)?;
        // The sub-project has no layout of its own: it uses the parent's, and says so.
        assert_eq!(
            form_of(&mut tim, "WEB", "new-issue").await?,
            ["description"]
        );
        let below = editor_of(&mut ada, "WEB").await?;
        assert!(below["layout"].is_null());
        assert_eq!(below["inherited_from"]["project"]["short_name"], "ACME");

        // Once it makes its own, its form follows its own layout; the parent's does not change.
        let own = customize(&mut ada, "WEB").await?;
        assert_eq!(
            own["sections"][0]["fields"][0]["field"],
            "description",
            "the copy starts as the layout it replaces"
        );
        let (own_layout, own_version) = slot(&own)?;
        let own_token = take_lock(&mut ada, &own_layout, AN_EDITOR).await?["lock_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned();
        ok(&save(
            &mut ada,
            &own_layout,
            &own_version,
            &own_token,
            json!([{"tab": "", "title": "", "fields": [
                {"field": "description", "roles": [&team]},
                {"field": "priority", "roles": [&team]},
            ]}]),
        )
        .await?)?;
        assert_eq!(
            form_of(&mut tim, "WEB", "new-issue").await?,
            ["description", "priority"]
        );
        assert_eq!(
            form_of(&mut tim, "ACME", "new-issue").await?,
            ["description"]
        );
        // Taking the sub-project's layout away gives it the parent's again.
        let (own_layout, own_version) = slot(&editor_of(&mut ada, "WEB").await?)?;
        let gone = ada
            .send(
                "DELETE",
                &format!("{LAYOUTS}/{own_layout}?version={own_version}"),
                None,
            )
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        assert_eq!(
            form_of(&mut tim, "WEB", "new-issue").await?,
            ["description"]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_two_people_cannot_change_one_layout_at_once() -> TestResult {
    each_database("iss_17_layout_lock", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lena, _) = site.person(StartingRole::ProjectLead, "lena").await?;
        let editor = customize(&mut ada, "ACME").await?;
        let (layout, version) = slot(&editor)?;
        let sections = json!([{"tab": "", "title": "Mine", "fields": []}]);

        // Ada is editing: Lena is told so, and cannot take the lock.
        let ada_lock = take_lock(&mut ada, &layout, AN_EDITOR).await?;
        assert_eq!(ada_lock["status"], "taken");
        let lena_lock = take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?;
        assert_eq!(lena_lock["status"], "held");

        // A save without the lock is refused and writes nothing, even with the right version.
        let wrong = save(
            &mut lena,
            &layout,
            &version,
            "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f",
            sections.clone(),
        )
        .await?;
        assert_eq!(wrong.status, StatusCode::CONFLICT, "{}", wrong.body);
        assert_eq!(wrong.code(), "edit.taken-over");
        assert_eq!(
            editor_of(&mut ada, "ACME").await?["layout"]["version"],
            version
        );

        // Ada, who holds it, saves; the lock is let go, so Lena can take it afterwards and sees
        // Ada's sections.
        let token = ada_lock["lock_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned();
        let saved = save(&mut ada, &layout, &version, &token, sections).await?;
        assert_eq!(ok(&saved)?["sections"][0]["title"], "Mine");
        let lena_lock = take_lock(&mut lena, &layout, ANOTHER_EDITOR).await?;
        assert_eq!(lena_lock["status"], "taken");
        // A save on the version Lena started from before Ada's is a conflict.
        let stale = save(
            &mut lena,
            &layout,
            &version,
            lena_lock["lock_token"].as_str().ok_or("no token")?,
            json!([]),
        )
        .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert_eq!(stale.code(), "edit.version-changed");
        Ok(())
    })
    .await
}
