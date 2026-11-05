//! Time defaults and *Billable* over HTTP (TIME-02, TIME-13, ADR-0035, decision 1): *Billable* offered and kept
//! only while the organization has it on, and what a project says of the defaults it inherits from the projects
//! above. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_in, key_of, ok};
use super::time_http::{MY_ENTRIES, add_choice, book, set_on_project, starter};

/// What the project `project` inherits for the field, as its page says.
async fn above_inherits(
    browser: &mut Browser,
    project: &str,
    field: &str,
) -> Result<Value, Failure> {
    let page = browser
        .send("GET", &format!("{PROJECTS}/{project}"), None)
        .await?;
    Ok(ok(&page)?["inherited_fields"][field].clone())
}

#[tokio::test]
async fn time_02_billable_is_offered_and_kept_only_while_the_organization_has_it_on() -> TestResult
{
    each_database("time_02_billable_switch", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let billable_entry = async |ada: &mut Browser| -> Result<Value, Failure> {
            let body =
                json!({"issue": issue, "date": "2027-01-14", "minutes": 10, "entry": {"billable": true}});
            let made = ada
                .send("POST", &format!("{MY_ENTRIES}?fields=billable"), Some(body))
                .await?;
            Ok(ok(&made)?["billable"].clone())
        };
        let switch = async |ada: &mut Browser, on: bool| -> Result<(), Failure> {
            let set = ada
                .send(
                    "PUT",
                    "/api/v1/settings/time.billable",
                    Some(json!({"value": on})),
                )
                .await?;
            ok(&set)?;
            Ok(())
        };
        // Off at the start: the box is not offered, and a mark sent anyway is not kept.
        assert_eq!(billable_entry(&mut ada).await?, Value::Null);
        switch(&mut ada, true).await?;
        assert_eq!(billable_entry(&mut ada).await?, json!(true));
        // Switched off again: the box goes, and nothing says what was marked.
        switch(&mut ada, false).await?;
        assert_eq!(billable_entry(&mut ada).await?, Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_a_projects_cost_center_is_the_default_of_its_sub_projects_issues() -> TestResult {
    each_database("time_13_cost_center_default", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let cost_center = starter(&mut ada, "cost-center").await?;
        let product = add_choice(&mut ada, &cost_center, "4100 Product development").await?;
        let sub = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "SUB", "name": "Sub"})),
            )
            .await?;
        let sub = id_in(ok(&sub)?)?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{sub}/issues"),
                Some(site.new_issue("Paper runs out")),
            )
            .await?;
        let issue = key_of(&made)?;
        let booked = book(&mut ada, &issue, 10).await?;
        assert_eq!(ok(&booked)?["own_fields"][&cost_center], Value::Null);
        // The project above sets it; the sub-project's issue starts with it, and the project says so.
        set_on_project(&mut ada, &site.acme, &cost_center, &product).await?;
        let booked = book(&mut ada, &issue, 10).await?;
        assert_eq!(ok(&booked)?["own_fields"][&cost_center], json!(product));
        let above = ada
            .send("GET", &format!("{PROJECTS}/{}", site.acme), None)
            .await?;
        assert_eq!(ok(&above)?["own_fields"][&cost_center], json!(product));
        assert!(
            ok(&above)?["changeable_fields"]
                .as_array()
                .is_some_and(|keys| keys.contains(&json!(cost_center)))
        );
        // The sub-project holds no value of its own: its page says what it inherits, and from where.
        let own = ada.send("GET", &format!("{PROJECTS}/{sub}"), None).await?;
        let own = ok(&own)?;
        assert_eq!(own["own_fields"][&cost_center], Value::Null);
        assert_eq!(own["inherited_fields"][&cost_center], json!(product));
        assert_eq!(
            own["inherited_from"][&cost_center]["project"]["id"],
            json!(site.acme)
        );
        // A project that sets the value itself inherits nothing for it.
        assert_eq!(
            above_inherits(&mut ada, &site.acme, &cost_center).await?,
            Value::Null
        );
        // A lead who sees only the sub-project is told the value comes from a project above they
        // cannot see: no name, no ID.
        let (mut lead, _) = site
            .person_in(StartingRole::ProjectLead, "lead", &sub)
            .await?;
        let seen = lead.send("GET", &format!("{PROJECTS}/{sub}"), None).await?;
        let seen = ok(&seen)?;
        assert_eq!(
            seen["inherited_fields"][&cost_center],
            json!(product),
            "{seen}"
        );
        assert_eq!(seen["inherited_from"][&cost_center], json!({}), "{seen}");
        // A reader who may not change the settings of the project is told nothing of what a project
        // above holds, not even that it holds something.
        for (role, name) in [
            (StartingRole::Viewer, "viewer"),
            (StartingRole::TeamMember, "member"),
        ] {
            let (mut reader, _) = site.person_in(role, name, &sub).await?;
            let page = reader
                .send("GET", &format!("{PROJECTS}/{sub}"), None)
                .await?;
            let page = ok(&page)?;
            assert_eq!(page["inherited_fields"], json!({}), "{name}: {page}");
            assert_eq!(page["inherited_from"], json!({}), "{name}: {page}");
        }
        Ok(())
    })
    .await
}
