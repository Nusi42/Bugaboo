//! *Check access* names the level of a field for one issue type over HTTP on both databases
//! (SEC-03, M3-F44, ADR-0027): the answer says whether the levels differ by type, and gives the
//! level for the type that is asked about.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{create_field, set_levels};
use super::http::rig_and_app;
use super::issues_site::{Site, id_in, ok};
use super::policy_rig::Member;

/// The type of the issue type a type nobody set a level for: the second of the first start.
async fn type_named(
    who: &mut super::http::Browser,
    name: &str,
) -> Result<String, bb_testing::Failure> {
    let types = who.send("GET", "/api/v1/issue-types", None).await?;
    ok(&types)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|each| each["name"] == name))
        .map(id_in)
        .transpose()?
        .ok_or_else(|| format!("no issue type {name}").into())
}

#[tokio::test]
async fn sec_03_check_access_names_the_level_per_issue_type() -> TestResult {
    each_database("sec_03_check_access_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Tester", "kind": "person", "applies_to": "issues"}),
            )
            .await?,
        )?;
        // A person who sees the Tester field, except on bugs.
        let role = site
            .org
            .role(
                "Careful",
                &[Permission::SeeProjects, Permission::CheckAccess],
            )
            .await?;
        set_levels(
            &site,
            &rig,
            role.id,
            &[
                (&tester, None, FieldLevel::CanSee),
                (&tester, Some(&site.kind), FieldLevel::Hidden),
            ],
        )
        .await?;
        let started = rig
            .person(&site.org.admin.session, "careful@example.test")
            .await?;
        let careful = Member {
            id: started.person.id,
            session: started.session_token,
        };
        site.org.give_everywhere(&careful, role.id).await?;
        let level_of = |answer: &Value| -> Value {
            answer["fields"]
                .as_array()
                .and_then(|fields| fields.iter().find(|each| each["subject"] == tester))
                .map_or(Value::Null, |found| found["level"].clone())
        };
        let path = format!("/api/v1/check-access/people/{}", careful.id);
        // For all types the person sees it, and the screen is told the levels differ by type.
        let all = ada.send("GET", &path, None).await?;
        let all = ok(&all)?.clone();
        assert_eq!(all["differs_by_type"], true);
        assert!(all.get("issue_type").is_none());
        assert_eq!(level_of(&all), "can-see");
        // For the type that has a level of its own it is hidden, and the answer names the type.
        let bug = ada
            .send("GET", &format!("{path}?issue_type={}", site.kind), None)
            .await?;
        let bug = ok(&bug)?.clone();
        assert_eq!(level_of(&bug), "hidden");
        assert_eq!(bug["issue_type"]["id"], site.kind);
        // A type nobody set a level for has the level for all types.
        let other = type_named(&mut ada, "Feature").await?;
        let other = ada
            .send("GET", &format!("{path}?issue_type={other}"), None)
            .await?;
        assert_eq!(level_of(ok(&other)?), "can-see");
        // An unknown type is not found.
        let nowhere = ada
            .send(
                "GET",
                &format!("{path}?issue_type=01890a5d-ac96-774b-bcce-b302099a8057"),
                None,
            )
            .await?;
        assert_eq!(nowhere.status, StatusCode::NOT_FOUND);
        assert_eq!(nowhere.code(), "resource.not-found");
        Ok(())
    })
    .await
}
