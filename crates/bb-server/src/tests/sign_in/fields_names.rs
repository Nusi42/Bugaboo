//! Who may be named, and what the list to choose from offers (CF-01, CF-07, ISS-01, ADR-0027): the
//! list for an own field of people judges the field like a change of it does, and *Undo* puts
//! back only what still exists.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{FIELDS, make_field, reader_of};
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, ok};

/// The permissions of somebody who sees and changes issues.
const CHANGER: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::SeeIssues,
    Permission::ChangeIssues,
];

/// The status and the code of an answer; no code for a success.
fn outcome(reply: &Reply) -> (StatusCode, &str) {
    (
        reply.status,
        if reply.status == StatusCode::OK {
            ""
        } else {
            reply.code()
        },
    )
}

/// The list of people for a field, with this query (`field=...&issue_type=...`).
async fn people_for(who: &mut Browser, site: &Site<'_>, query: &str) -> Result<Reply, Failure> {
    who.send(
        "GET",
        &format!("{PROJECTS}/{}/assignable-people?{query}", site.acme),
        None,
    )
    .await
}

/// A person field of issues, made as the administrator; its ID.
async fn person_field(ada: &mut Browser, name: &str) -> Result<String, Failure> {
    id_in(
        &make_field(
            ada,
            json!({"name": name, "kind": "person", "applies_to": "issues"}),
        )
        .await?,
    )
}

#[tokio::test]
async fn cf_06_the_list_for_a_field_follows_its_level_for_the_issue_type() -> TestResult {
    each_database("cf_06_people_list_per_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = person_field(&mut ada, "Tester").await?;
        let question = id_in(ok(&ada
            .send(
                "POST",
                "/api/v1/issue-types",
                Some(json!({"name": "Question", "icon": "star"})),
            )
            .await?)?)?;
        // Changeable in general, but hidden for Bug.
        let mut tim = reader_of(
            &site,
            &rig,
            "tim@example.test",
            &CHANGER,
            &[
                (&tester, None, FieldLevel::CanChange),
                (&tester, Some(&site.bug), FieldLevel::Hidden),
            ],
        )
        .await?;
        let query = format!("field={tester}&issue_type={}", site.bug);
        let for_bug = people_for(&mut tim, &site, &query).await?;
        assert_eq!(
            outcome(&for_bug),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let query = format!("field={tester}&issue_type={question}");
        let for_question = people_for(&mut tim, &site, &query).await?;
        assert_eq!(
            outcome(&for_question),
            (StatusCode::OK, ""),
            "{}",
            for_question.body
        );
        // An issue decides its type: the list for an issue of type Bug is refused as well.
        let made = ok(&site.make(&mut ada, "A bug").await?)?.clone();
        let key = made["key"].as_str().ok_or("no key")?;
        let for_issue = people_for(&mut tim, &site, &format!("field={tester}&issue={key}")).await?;
        assert_eq!(
            outcome(&for_issue),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_06_a_project_field_is_not_opened_by_an_issue_type() -> TestResult {
    each_database("cf_06_project_field_no_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let sponsor = id_in(
            &make_field(
                &mut ada,
                json!({"name": "Sponsor", "kind": "person", "applies_to": "projects"}),
            )
            .await?,
        )?;
        // A project has no type of issue: the level for Bug is no business of a project field.
        let mut tim = reader_of(
            &site,
            &rig,
            "tim@example.test",
            &[Permission::SeeProjects, Permission::ChangeProjectSettings],
            &[
                (&sponsor, None, FieldLevel::CanChange),
                (&sponsor, Some(&site.bug), FieldLevel::Hidden),
            ],
        )
        .await?;
        let query = format!("field={sponsor}&issue_type={}", site.bug);
        let listed = people_for(&mut tim, &site, &query).await?;
        assert_eq!(outcome(&listed), (StatusCode::OK, ""), "{}", listed.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_01_the_list_for_a_field_refuses_a_field_that_is_not_here_for_people() -> TestResult {
    each_database("cf_01_people_list_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = person_field(&mut ada, "Tester").await?;
        let secret = person_field(&mut ada, "Secret").await?;
        let note = id_in(
            &make_field(
                &mut ada,
                json!({"name": "Note", "kind": "text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let old = make_field(
            &mut ada,
            json!({"name": "Old", "kind": "person", "applies_to": "issues"}),
        )
        .await?;
        let old_id = id_in(&old)?;
        let retired = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{old_id}"),
                Some(json!({"version": old["version"], "retired": true})),
            )
            .await?;
        ok(&retired)?;
        let beta = id_in(ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?)?;
        let elsewhere = id_in(
            &make_field(
                &mut ada,
                json!({
                    "name": "Beta only",
                    "kind": "person",
                    "applies_to": "issues",
                    "scope": {"projects": [beta]},
                }),
            )
            .await?,
        )?;
        let fine = people_for(&mut ada, &site, &format!("field={tester}")).await?;
        assert_eq!(outcome(&fine), (StatusCode::OK, ""), "{}", fine.body);
        // A field the person may not even see is refused like one that is not there.
        let mut tim = reader_of(
            &site,
            &rig,
            "tim@example.test",
            &CHANGER,
            &[(&secret, None, FieldLevel::Hidden)],
        )
        .await?;
        let hidden = people_for(&mut tim, &site, &format!("field={secret}")).await?;
        assert_eq!(
            outcome(&hidden),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // Not a field of people, retired, or not used in this project: it is not here.
        for (field, what) in [
            (note, "text"),
            (old_id, "retired"),
            (elsewhere, "elsewhere"),
        ] {
            let refused = people_for(&mut ada, &site, &format!("field={field}")).await?;
            assert_eq!(
                outcome(&refused),
                (StatusCode::BAD_REQUEST, "field.not_here"),
                "{what}: {}",
                refused.body
            );
        }
        Ok(())
    })
    .await
}

/// Changes the own fields of an issue on the version it has now; the answer.
async fn set(who: &mut Browser, issue: &Value, fields: Value) -> Result<Reply, Failure> {
    let path = format!("{ISSUES}/{}", issue["key"].as_str().ok_or("no key")?);
    let version = ok(&who.send("GET", &path, None).await?)?["issue"]["version"].clone();
    who.send(
        "PATCH",
        &path,
        Some(json!({"version": version, "fields": fields})),
    )
    .await
}

#[tokio::test]
async fn cf_07_undo_needs_the_right_to_change_the_field() -> TestResult {
    each_database("cf_07_undo_level", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = person_field(&mut ada, "Tester").await?;
        let (_, vic) = site.person(StartingRole::Viewer, "vic").await?;
        let me = id_in(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let made = ok(&site.make(&mut ada, "Undo me").await?)?.clone();
        ok(&set(&mut ada, &made, json!({ &tester: vic.to_string() })).await?)?;
        ok(&set(&mut ada, &made, json!({ &tester: me })).await?)?;
        // Somebody who only sees the field cannot put back what it held before.
        let mut sees = reader_of(
            &site,
            &rig,
            "sue@example.test",
            &CHANGER,
            &[(&tester, None, FieldLevel::CanSee)],
        )
        .await?;
        let undone = set(&mut sees, &made, json!({ &tester: vic.to_string() })).await?;
        assert_eq!(
            outcome(&undone),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // Somebody who may change it can.
        let undone = set(&mut ada, &made, json!({ &tester: vic.to_string() })).await?;
        assert_eq!(outcome(&undone), (StatusCode::OK, ""), "{}", undone.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_undo_does_not_put_back_a_group_or_an_issue_that_is_gone() -> TestResult {
    each_database("cf_07_undo_gone", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let team = id_in(
            &make_field(
                &mut ada,
                json!({"name": "Team", "kind": "group", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let twin = id_in(
            &make_field(
                &mut ada,
                json!({"name": "Twin", "kind": "issue", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let group = id_in(ok(&ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Core"})))
            .await?)?)?;
        let other = id_in(ok(&ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Edge"})))
            .await?)?)?;
        let made = ok(&site.make(&mut ada, "Undo me").await?)?.clone();
        let gone = ok(&site.make(&mut ada, "Soon gone").await?)?.clone();
        let both = |group: &str, issue: &Value| json!({ &team: group, &twin: issue["id"] });
        ok(&set(&mut ada, &made, both(&group, &gone)).await?)?;
        ok(&set(&mut ada, &made, both(&other, &made)).await?)?;
        // The group and the issue go away; the last change still is the one to put back.
        let deleted = ada
            .send("DELETE", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
        let trashed = ada
            .send(
                "DELETE",
                &format!("{ISSUES}/{}", gone["key"].as_str().ok_or("no key")?),
                None,
            )
            .await?;
        assert_eq!(trashed.status, StatusCode::NO_CONTENT, "{}", trashed.body);
        for (name, values) in [
            ("group", json!({ &team: group })),
            ("issue", json!({ &twin: gone["id"] })),
        ] {
            let undone = set(&mut ada, &made, values).await?;
            assert_eq!(
                outcome(&undone),
                (StatusCode::BAD_REQUEST, "field.unknown_reference"),
                "{name}: {}",
                undone.body
            );
        }
        Ok(())
    })
    .await
}
