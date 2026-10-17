//! Where issue types and groups of labels are offered, over HTTP on both databases (ISS-02,
//! TAG-02, SEC-03, M3-F22, M3-F122): a project offers only the types it uses, and a Project lead
//! looks after the types and labels of their project and nothing else.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, ok};

const TYPES: &str = "/api/v1/issue-types";
const GROUPS: &str = "/api/v1/label-groups";

/// The ID of the issue type with this name.
async fn type_named(who: &mut Browser, name: &str) -> Result<String, Failure> {
    let types = who.send("GET", TYPES, None).await?;
    ok(&types)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|each| each["name"] == name))
        .map(id_in)
        .transpose()?
        .ok_or_else(|| format!("no issue type {name}").into())
}

/// The version of an issue type, as the list shows it.
async fn version_of(who: &mut Browser, issue_type: &str) -> Result<i64, Failure> {
    let types = who.send("GET", TYPES, None).await?;
    ok(&types)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|each| each["id"] == issue_type))
        .and_then(|found| found["version"].as_i64())
        .ok_or_else(|| "no version".into())
}

/// Says in which places an issue type is offered or not; the reply.
async fn offer(who: &mut Browser, issue_type: &str, changes: Value) -> Result<Reply, Failure> {
    let version = version_of(who, issue_type).await?;
    who.send(
        "PUT",
        &format!("{TYPES}/{issue_type}/offers"),
        Some(json!({"version": version, "changes": changes})),
    )
    .await
}

/// A new issue of the type in the project; the reply.
async fn create_in(who: &mut Browser, project: &str, issue_type: &str) -> Result<Reply, Failure> {
    who.send(
        "POST",
        &format!("{PROJECTS}/{project}/issues"),
        Some(json!({"issue_type": issue_type, "title": "An issue"})),
    )
    .await
}

fn code(reply: &Reply) -> (StatusCode, &str) {
    (reply.status, reply.code())
}

#[tokio::test]
async fn iss_02_a_project_offers_only_its_issue_types() -> TestResult {
    each_database("iss_02_project_offers_only_its_types", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?;
        let (bug, task) = (
            type_named(&mut ada, "Bug").await?,
            type_named(&mut ada, "Task").await?,
        );
        // A person who looks after the types of ACME only limits ACME to *Task*.
        let (mut lena, _) = site
            .person_in_acme(StartingRole::ProjectLead, "lena")
            .await?;
        let changed = offer(
            &mut lena,
            &bug,
            json!([{"project": "ACME", "offered": false}]),
        )
        .await?;
        ok(&changed)?;
        assert_eq!(changed.body["rows"][0]["offered"], false);
        assert_eq!(changed.body["rows"][0]["place"]["short_name"], "ACME");
        // A Bug is not made in ACME any more, and the answer says why.
        assert_eq!(
            code(&create_in(&mut ada, "ACME", &bug).await?),
            (StatusCode::CONFLICT, "issue.type_not_offered")
        );
        ok(&create_in(&mut ada, "ACME", &task).await?)?;
        // Any other project still offers both.
        ok(&create_in(&mut ada, "WEB", &bug).await?)?;
        // The list says it for the project that is asked about, and for none without one.
        let in_acme = ada
            .send(
                "GET",
                &format!(
                    "{TYPES}?project=ACME&fields=id,name,offered_here,own_offer,may_change_offer"
                ),
                None,
            )
            .await?;
        let row = |reply: &Reply, id: &str| -> Value {
            reply.body["items"]
                .as_array()
                .and_then(|items| items.iter().find(|each| each["id"] == id))
                .cloned()
                .unwrap_or(Value::Null)
        };
        assert_eq!(row(&in_acme, &bug)["offered_here"], false);
        assert_eq!(row(&in_acme, &bug)["own_offer"], false);
        assert_eq!(row(&in_acme, &task)["offered_here"], true);
        assert!(row(&in_acme, &task).get("own_offer").is_none());
        let in_web = ada
            .send(
                "GET",
                &format!("{TYPES}?project=WEB&fields=id,offered_here"),
                None,
            )
            .await?;
        assert_eq!(row(&in_web, &bug)["offered_here"], true);
        let without = ada
            .send("GET", &format!("{TYPES}?fields=id,name"), None)
            .await?;
        assert!(row(&without, &bug).get("offered_here").is_none());
        // The rows are read a page at a time.
        let rows = ada
            .send("GET", &format!("{TYPES}/{bug}/offers"), None)
            .await?;
        assert_eq!(ok(&rows)?["items"].as_array().map(Vec::len), Some(1));
        // A project the person does not see is not there for them.
        let nowhere = ada
            .send("GET", &format!("{TYPES}?project=NOWHERE"), None)
            .await?;
        assert_eq!(
            code(&nowhere),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_project_lead_manages_the_types_of_their_project_only() -> TestResult {
    each_database("sec_03_lead_manages_own_types", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?;
        let bug = type_named(&mut ada, "Bug").await?;
        let (mut lena, _) = site
            .person_in_acme(StartingRole::ProjectLead, "lena")
            .await?;
        // The whole organization is not the lead's; another project is not even there for them.
        let whole = lena
            .send(
                "POST",
                TYPES,
                Some(json!({"name": "Question", "icon": "star"})),
            )
            .await?;
        assert_eq!(code(&whole), (StatusCode::FORBIDDEN, "access.forbidden"));
        let elsewhere = lena
            .send(
                "POST",
                TYPES,
                Some(json!({"name": "Question", "icon": "star", "project": "WEB"})),
            )
            .await?;
        assert_eq!(
            code(&elsewhere),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        // In ACME the lead makes a type and looks after it.
        let made = lena
            .send(
                "POST",
                TYPES,
                Some(json!({"name": "Question", "icon": "star", "project": "ACME"})),
            )
            .await?;
        let made = ok(&made)?.clone();
        assert_eq!(made["created_in"]["kind"], "project");
        assert_eq!(made["created_in"]["short_name"], "ACME");
        assert_eq!(made["may_manage"], true);
        let id = id_in(&made)?;
        ok(&lena
            .send(
                "PATCH",
                &format!("{TYPES}/{id}"),
                Some(json!({"version": made["version"], "name": "Ask"})),
            )
            .await?)?;
        // The type of the organization is not the lead's to change, only to switch off in ACME.
        let renamed = lena
            .send(
                "PATCH",
                &format!("{TYPES}/{bug}"),
                Some(json!({"version": 1, "name": "Defect"})),
            )
            .await?;
        assert_eq!(code(&renamed), (StatusCode::FORBIDDEN, "access.forbidden"));
        ok(&offer(
            &mut lena,
            &bug,
            json!([{"project": "ACME", "offered": false}]),
        )
        .await?)?;
        let for_all = offer(&mut lena, &bug, json!([{"offered": false}])).await?;
        assert_eq!(code(&for_all), (StatusCode::FORBIDDEN, "access.forbidden"));
        let for_web = offer(
            &mut lena,
            &bug,
            json!([{"project": "WEB", "offered": false}]),
        )
        .await?;
        assert_eq!(
            code(&for_web),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        // The lead's type is offered in ACME and not in WEB.
        ok(&create_in(&mut ada, "ACME", &id).await?)?;
        assert_eq!(
            code(&create_in(&mut ada, "WEB", &id).await?),
            (StatusCode::CONFLICT, "issue.type_not_offered")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_02_a_label_group_of_a_project_is_offered_there_and_below() -> TestResult {
    each_database("tag_02_group_of_a_project", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?;
        ok(&ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "SUB", "name": "Sub"})),
            )
            .await?)?;
        let (mut lena, _) = site
            .person_in_acme(StartingRole::ProjectLead, "lena")
            .await?;
        let whole = lena
            .send(
                "POST",
                GROUPS,
                Some(json!({"name": "Area", "pick_one": false})),
            )
            .await?;
        assert_eq!(code(&whole), (StatusCode::FORBIDDEN, "access.forbidden"));
        let group = ok(&lena
            .send(
                "POST",
                GROUPS,
                Some(json!({"name": "Area", "pick_one": false, "project": "ACME"})),
            )
            .await?)?
        .clone();
        assert_eq!(group["created_in"]["short_name"], "ACME");
        assert_eq!(group["may_manage"], true);
        // The group is offered in ACME and below, and not in the other projects.
        let says = |listed: &Value| -> Value {
            listed["groups"]
                .as_array()
                .and_then(|groups| groups.iter().find(|each| each["id"] == group["id"]))
                .map_or(Value::Null, |found| found["offered_here"].clone())
        };
        let in_acme = ada.send("GET", "/api/v1/labels?project=ACME", None).await?;
        assert_eq!(says(ok(&in_acme)?), true);
        let in_web = ada.send("GET", "/api/v1/labels?project=WEB", None).await?;
        // A project the group cannot reach does not list it at all.
        assert_eq!(says(ok(&in_web)?), Value::Null);
        let in_sub = ada.send("GET", "/api/v1/labels?project=SUB", None).await?;
        assert_eq!(says(ok(&in_sub)?), true, "and in the sub-projects of ACME");
        let without = ada.send("GET", "/api/v1/labels", None).await?;
        assert_eq!(says(ok(&without)?), Value::Null);
        // A label in the group follows the group: the lead makes it; the group of nobody's project
        // takes no label from them.
        let label = lena
            .send(
                "POST",
                "/api/v1/labels",
                Some(json!({"name": "login", "group": group["id"]})),
            )
            .await?;
        ok(&label)?;
        // An issue of ACME takes the label, an issue of WEB does not.
        let acme_issue = ok(&site.create(&mut ada, "In ACME").await?)?["key"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        let web_issue = ok(&create_in(&mut ada, "WEB", &site.kind).await?)?["key"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        let label_id = id_in(ok(&label)?)?;
        let put = |issue: String| format!("{ISSUES}/{issue}/labels");
        ok(&ada
            .send("POST", &put(acme_issue), Some(json!({"label": label_id})))
            .await?)?;
        let refused = ada
            .send("POST", &put(web_issue), Some(json!({"label": label_id})))
            .await?;
        assert_eq!(code(&refused), (StatusCode::CONFLICT, "label.not_offered"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_subject_created_in_a_project_is_not_there_for_a_guest_a_sibling_lead_or_a_key()
-> TestResult {
    each_database("sec_03_subject_of_a_project", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let web = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let question = id_in(ok(&ada
            .send(
                "POST",
                TYPES,
                Some(json!({"name": "Question", "icon": "circle-help", "project": "ACME"})),
            )
            .await?)?)?;
        let area = id_in(ok(&ada
            .send(
                "POST",
                GROUPS,
                Some(json!({"name": "Area", "pick_one": false, "project": "ACME"})),
            )
            .await?)?)?;
        let (mut gus, _) = site.person_in(StartingRole::Guest, "gus", &web).await?;
        let (mut sam, _) = site
            .person_in(StartingRole::ProjectLead, "sam", &web)
            .await?;
        let web_id = web.parse().map_err(|_| "no project ID")?;
        let key = super::access_keys::create(
            &rig,
            &site.org.admin,
            bb_core::access_key::KeyChoice::Everything,
            &[],
            &[web_id],
        )
        .await?
        .secret;
        let mut keyed = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        let routes = [
            ("GET", TYPES.to_owned(), None),
            ("GET", "/api/v1/labels".to_owned(), None),
            ("GET", format!("{TYPES}/{question}/offers"), None),
            ("GET", format!("{GROUPS}/{area}/offers"), None),
            (
                "PUT",
                format!("{TYPES}/{question}/offers"),
                Some(json!({"version": 1, "changes": [{"project": "WEB", "offered": false}]})),
            ),
        ];
        for (who, label) in [
            (0, "a guest"),
            (1, "a lead of a sibling project"),
            (2, "a key"),
        ] {
            for (method, path, body) in &routes {
                let reply = match who {
                    0 => gus.send(method, path, body.clone()).await?,
                    1 => sam.send(method, path, body.clone()).await?,
                    _ => keyed.script(method, path, body.clone(), &key).await?,
                };
                let listed = reply.body.to_string();
                assert!(
                    !listed.contains("Question") && !listed.contains("Area"),
                    "{label}: {method} {path}: {listed}"
                );
                if path.contains("/offers") {
                    assert_eq!(code(&reply), (StatusCode::NOT_FOUND, "resource.not_found"));
                }
            }
        }
        Ok(())
    })
    .await
}
