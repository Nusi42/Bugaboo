//! Who may be named, and what the list to choose from offers (CF-01, CF-07, ISS-01, ADR-0027): the
//! list for an own field of people judges the field like a change of it does.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::fields_http::{FIELDS, id_of, make_field, reader_of};
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{PROJECTS, Site, ok};

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
    id_of(
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
        let question = id_of(ok(&ada
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
async fn cf_01_the_list_for_a_field_refuses_a_field_that_is_not_here_for_people() -> TestResult {
    each_database("cf_01_people_list_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = person_field(&mut ada, "Tester").await?;
        let secret = person_field(&mut ada, "Secret").await?;
        let note = id_of(
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
        let old_id = id_of(&old)?;
        let retired = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{old_id}"),
                Some(json!({"version": old["version"], "retired": true})),
            )
            .await?;
        ok(&retired)?;
        let beta = id_of(ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?)?;
        let elsewhere = id_of(
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
