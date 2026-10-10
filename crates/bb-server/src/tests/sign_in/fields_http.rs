//! Own fields over HTTP on both databases (CF-01, CF-03, CF-04, CF-06, CF-07, CF-08): a wrong value
//! is refused with a sentence, a field is used only where it is meant, the level of a role counts
//! (also for one issue type), a hidden field is in no answer, and combining two choices moves every
//! value and leaves a record.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::IssueTypeId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

const FIELDS: &str = "/api/v1/fields";

/// Makes a field as `browser`; the answer, which has to be a success.
async fn make_field(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let made = browser.send("POST", FIELDS, Some(body)).await?;
    Ok(ok(&made)?.clone())
}

/// The ID of a field or a choice in an answer.
fn id_of(value: &Value) -> Result<String, Failure> {
    Ok(value["id"].as_str().ok_or("no ID")?.to_owned())
}

/// The path of the issues of ACME.
fn acme_issues() -> String {
    format!("{PROJECTS}/ACME/issues")
}

/// Makes an issue of the type `bug` in ACME with these values of own fields; the answer.
async fn make_with(ada: &mut Browser, bug: &str, fields: Value) -> Result<Reply, Failure> {
    let body = json!({"issue_type": bug, "title": "An issue", "fields": fields});
    ada.send("POST", &acme_issues(), Some(body)).await
}

/// A field *Severity* with three choices, for issues.
async fn severity(ada: &mut Browser) -> Result<Value, Failure> {
    make_field(
        ada,
        json!({
            "name": "Severity",
            "kind": "pick-one",
            "applies_to": "issues",
            "choices": [{"name": "Minor"}, {"name": "Major"}, {"name": "Critical"}],
        }),
    )
    .await
}

#[tokio::test]
async fn cf_03_a_required_field_and_a_rule_refuse_a_wrong_value_with_a_sentence() -> TestResult {
    each_database("cf_03_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let customer = make_field(
            &mut ada,
            json!({
                "name": "Customer",
                "kind": "text",
                "applies_to": "issues",
                "rules": {"required": true, "max_length": 5, "pattern": "[A-Za-z]+"},
            }),
        )
        .await?;
        let key = id_of(&customer)?;
        let refused = |reply: &Reply, code: &str| {
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::BAD_REQUEST, code),
                "{}",
                reply.body
            );
        };
        refused(
            &make_with(&mut ada, &site.bug, json!({})).await?,
            "field.required",
        );
        refused(
            &make_with(&mut ada, &site.bug, json!({ &key: "Customer" })).await?,
            "field.too_long",
        );
        refused(
            &make_with(&mut ada, &site.bug, json!({ &key: "Ab1" })).await?,
            "field.no_match",
        );
        refused(
            &make_with(&mut ada, &site.bug, json!({ &key: 7 })).await?,
            "field.wrong_value",
        );
        let made = make_with(&mut ada, &site.bug, json!({ &key: "Acme" })).await?;
        let issue = ok(&made)?;
        assert_eq!(issue["own_fields"][&key], "Acme");
        // A change is refused the same way, and a required field cannot be emptied.
        let path = format!("{ISSUES}/{}", issue["key"].as_str().ok_or("no key")?);
        let version = &issue["version"];
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: "Acmeee" }})),
            )
            .await?,
            "field.too_long",
        );
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: null }})),
            )
            .await?,
            "field.required",
        );
        // A choice the field does not offer.
        let level = severity(&mut ada).await?;
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({
                    "version": version,
                    "fields": { id_of(&level)?: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c" },
                })),
            )
            .await?,
            "field.unknown_choice",
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_03_a_field_for_one_project_is_not_offered_elsewhere() -> TestResult {
    each_database("cf_03_scope", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let beta = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project")?
            .to_owned();
        let field = make_field(
            &mut ada,
            json!({
                "name": "Customer",
                "kind": "text",
                "applies_to": "issues",
                "scope": {"projects": [site.acme.clone()]},
            }),
        )
        .await?;
        let key = id_of(&field)?;
        // Offered in ACME, not in BETA.
        let in_acme = ada.send("GET", &format!("{FIELDS}?project=ACME"), None).await?;
        assert_eq!(ok(&in_acme)?["fields"].as_array().map(Vec::len), Some(1));
        let in_beta = ada.send("GET", &format!("{FIELDS}?project=BETA"), None).await?;
        assert_eq!(ok(&in_beta)?["fields"].as_array().map(Vec::len), Some(0));
        let body = |project: &str| {
            json!({"issue_type": site.bug, "title": "Here", "fields": { &key: "Acme" }, "project": project})
        };
        let there = ada
            .send("POST", &format!("{PROJECTS}/{beta}/issues"), Some(body("beta")))
            .await?;
        assert_eq!(
            (there.status, there.code()),
            (StatusCode::BAD_REQUEST, "field.not_here"),
            "{}",
            there.body
        );
        let here = ada.send("POST", &acme_issues(), Some(body("acme"))).await?;
        assert_eq!(ok(&here)?["own_fields"][&key], "Acme");
        // Where the field is used, listed before a change.
        let usage = ada
            .send("GET", &format!("{FIELDS}/{key}/used-by"), None)
            .await?;
        assert_eq!(ok(&usage)?["whole_organization"], false);
        assert_eq!(ok(&usage)?["projects"][0]["short_name"], "ACME");
        Ok(())
    })
    .await
}

/// The key of a field, by its word or its ID.
fn key_of(code: &str) -> Result<FieldKey, Failure> {
    FieldKey::from_code(code).ok_or_else(|| format!("no field {code}").into())
}

/// A person with a role of their own that has `permissions` and the given levels, for the whole
/// organization or, with an issue type, for that type only: `(field, issue type, level)`.
async fn reader_of(
    site: &Site<'_>,
    rig: &Rig,
    email: &str,
    permissions: &[Permission],
    levels: &[(&str, Option<&str>, FieldLevel)],
) -> Result<Browser, Failure> {
    let role = site.org.role(email, permissions).await?;
    for (field, issue_type, level) in levels {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    issue_type: issue_type
                        .map(str::parse::<IssueTypeId>)
                        .transpose()
                        .map_err(|error| error.to_string())?,
                    field: key_of(field)?,
                    level: Some(*level),
                },
            )
            .await?;
    }
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

/// An issue of the type *Bug* and one of a second type, both with the choice `choice` in the field
/// `key`: their keys.
async fn an_issue_per_type(
    site: &Site<'_>,
    ada: &mut Browser,
    key: &str,
    choice: &str,
) -> Result<(String, String), Failure> {
    // A second issue type, so a level can be set for one of the two.
    let task = ok(&ada
        .send(
            "POST",
            "/api/v1/issue-types",
            Some(json!({"name": "Question", "icon": "star"})),
        )
        .await?)?["id"]
        .as_str()
        .ok_or("no type")?
        .to_owned();
    let in_bug = ok(&site.make(ada, "A bug").await?)?["key"]
        .as_str()
        .ok_or("no key")?
        .to_owned();
    let in_task = ok(&ada
        .send(
            "POST",
            &acme_issues(),
            Some(json!({"issue_type": task, "title": "A question"})),
        )
        .await?)?["key"]
        .as_str()
        .ok_or("no key")?
        .to_owned();
    for issue in [&in_bug, &in_task] {
        let version =
            ok(&ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?)?["issue"]["version"]
                .clone();
        let set = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "fields": { key: choice }})),
            )
            .await?;
        ok(&set)?;
    }
    Ok((in_bug, in_task))
}

#[tokio::test]
async fn cf_06_an_own_field_follows_its_level_per_role_and_per_issue_type() -> TestResult {
    each_database("cf_06_levels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_of(&level)?, id_of(&level["choices"][1])?);
        let (in_bug, in_task) = an_issue_per_type(&site, &mut ada, &key, &major).await?;
        // A role that sees the field in general (the starting level for it) but not for Bug.
        let see = [
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::ChangeIssues,
        ];
        let mut tim = reader_of(
            &site,
            &rig,
            "tim@example.test",
            &see,
            &[
                (&key, None, FieldLevel::CanChange),
                (&key, Some(&site.bug), FieldLevel::Hidden),
            ],
        )
        .await?;
        let bug = tim.send("GET", &format!("{ISSUES}/{in_bug}"), None).await?;
        assert!(
            ok(&bug)?["issue"]["own_fields"].get(&key).is_none(),
            "hidden for Bug: {}",
            bug.body
        );
        assert!(
            ok(&bug)?["changeable"]
                .as_array()
                .is_some_and(|keys| !keys.iter().any(|each| each == &json!(key)))
        );
        let question = tim
            .send("GET", &format!("{ISSUES}/{in_task}"), None)
            .await?;
        assert_eq!(ok(&question)?["issue"]["own_fields"][&key], json!(major));
        assert!(
            ok(&question)?["changeable"]
                .as_array()
                .is_some_and(|keys| keys.iter().any(|each| each == &json!(key)))
        );
        // A change of a field that is hidden for the type is refused like one that is not there.
        let version = ok(&bug)?["issue"]["version"].clone();
        let refused = tim
            .send(
                "PATCH",
                &format!("{ISSUES}/{in_bug}"),
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden"),
            "{}",
            refused.body
        );
        // A role that can only see the field cannot change it.
        let mut viewer = reader_of(
            &site,
            &rig,
            "vera@example.test",
            &see,
            &[(&key, None, FieldLevel::CanSee)],
        )
        .await?;
        let seen = viewer
            .send("GET", &format!("{ISSUES}/{in_bug}"), None)
            .await?;
        assert_eq!(ok(&seen)?["issue"]["own_fields"][&key], json!(major));
        let version = ok(&seen)?["issue"]["version"].clone();
        let refused = viewer
            .send(
                "PATCH",
                &format!("{ISSUES}/{in_bug}"),
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        // The definition of the field is not shown to a role that sees it nowhere.
        let mut nobody = reader_of(&site, &rig, "nora@example.test", &see, &[]).await?;
        let list = nobody.send("GET", FIELDS, None).await?;
        assert_eq!(ok(&list)?["fields"].as_array().map(Vec::len), Some(0));
        let one = nobody.send("GET", &format!("{FIELDS}/{key}"), None).await?;
        assert_eq!(one.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_06_a_hidden_own_field_never_leaves_in_any_answer_or_history() -> TestResult {
    each_database("cf_06_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_of(&level)?, id_of(&level["choices"][1])?);
        let made = ok(&ada
            .send(
                "POST",
                &acme_issues(),
                Some(
                    json!({"issue_type": site.bug, "title": "Secret", "fields": { &key: &major }}),
                ),
            )
            .await?)?
        .clone();
        let issue = made["key"].as_str().ok_or("no key")?.to_owned();
        let path = format!("{ISSUES}/{issue}");
        // A role that changes the title, with the own field hidden (it has no level for it).
        let mut writer = reader_of(
            &site,
            &rig,
            "wes@example.test",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
            ],
            &[("title", None, FieldLevel::CanChange)],
        )
        .await?;
        let detail = writer.send("GET", &path, None).await?;
        assert!(!detail.body.to_string().contains(&key), "{}", detail.body);
        assert!(!detail.body.to_string().contains(&major), "{}", detail.body);
        let list = writer
            .send("GET", &format!("{}?fields=own_fields", acme_issues()), None)
            .await?;
        assert!(!list.body.to_string().contains(&major), "{}", list.body);
        // A write is refused alike for a hidden field and for one that is not there.
        let version = ok(&detail)?["issue"]["version"].clone();
        let hidden = writer
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        let missing = writer
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": version,
                    "fields": { "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c": &major },
                })),
            )
            .await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (missing.status, missing.code())
        );
        assert_eq!(hidden.code(), "access.forbidden");
        // Ada changes the own field; the writer, with the old version, is told who changed what:
        // the field that is hidden from them is not among the fields, nor in the values now.
        let minor = id_of(&level["choices"][0])?;
        ok(&ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: &minor }})),
            )
            .await?)?;
        let stale = writer
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "title": "Renamed"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert!(!stale.body.to_string().contains(&key), "{}", stale.body);
        assert!(!stale.body.to_string().contains(&minor), "{}", stale.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_combining_two_choices_moves_every_value_and_keeps_a_record() -> TestResult {
    each_database("cf_07_combine", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, minor, major) = (
            id_of(&level)?,
            id_of(&level["choices"][0])?,
            id_of(&level["choices"][1])?,
        );
        let mut issues = Vec::new();
        for (title, choice) in [("First", &major), ("Second", &minor), ("Third", &major)] {
            let made = ok(&ada
                .send(
                    "POST",
                    &acme_issues(),
                    Some(
                        json!({"issue_type": site.bug, "title": title, "fields": { &key: choice }}),
                    ),
                )
                .await?)?
            .clone();
            issues.push((
                made["key"].as_str().ok_or("no key")?.to_owned(),
                made["version"].clone(),
            ));
        }
        let combined = ada
            .send(
                "POST",
                &format!("{FIELDS}/{key}/choices/{major}/combine"),
                Some(json!({"version": level["version"], "into": minor})),
            )
            .await?;
        let after = ok(&combined)?;
        assert_eq!(
            after["version"],
            json!(level["version"].as_i64().unwrap_or(0) + 1)
        );
        assert_eq!(after["choices"][1]["retired"], true);
        for (issue, version) in &issues {
            let seen = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
            assert_eq!(
                ok(&seen)?["issue"]["own_fields"][&key],
                json!(minor),
                "{issue}"
            );
            // The issue that held the combined choice has a new version: its history has a record.
            let moved = ok(&seen)?["issue"]["version"].as_i64().unwrap_or(0);
            let before = version.as_i64().unwrap_or(0);
            assert!(moved == before || moved == before + 1, "{issue}");
        }
        assert_eq!(
            ok(&ada
                .send("GET", &format!("{ISSUES}/{}", issues[0].0), None)
                .await?)?["issue"]["version"],
            json!(issues[0].1.as_i64().unwrap_or(0) + 1)
        );
        assert_eq!(
            ok(&ada
                .send("GET", &format!("{ISSUES}/{}", issues[1].0), None)
                .await?)?["issue"]["version"],
            issues[1].1
        );
        // A person who looked at the first issue before is told that the field changed.
        let stale = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{}", issues[0].0),
                Some(json!({"version": issues[0].1, "title": "Late"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert_eq!(stale.body["conflict"]["current"][&key], json!(minor));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_a_retired_choice_stays_on_old_issues_only() -> TestResult {
    each_database("cf_07_retire", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_of(&level)?, id_of(&level["choices"][1])?);
        let old = ok(&ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({"issue_type": site.bug, "title": "Old", "fields": { &key: &major }})),
            )
            .await?)?
        .clone();
        let retired = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{key}/choices/{major}"),
                Some(json!({"version": level["version"], "retired": true})),
            )
            .await?;
        ok(&retired)?;
        // The old issue keeps the choice.
        let seen = ada
            .send(
                "GET",
                &format!("{ISSUES}/{}", old["key"].as_str().ok_or("no key")?),
                None,
            )
            .await?;
        assert_eq!(ok(&seen)?["issue"]["own_fields"][&key], json!(major));
        // A new issue cannot get it.
        let refused = ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({"issue_type": site.bug, "title": "New", "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::BAD_REQUEST, "field.unknown_choice"),
            "{}",
            refused.body
        );
        // The old issue may be saved with the value it has.
        let kept = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{}", old["key"].as_str().ok_or("no key")?),
                Some(json!({"version": old["version"], "fields": { &key: &major }})),
            )
            .await?;
        ok(&kept)?;
        // A role that works with fields of the starting roles: a Team member sees and changes it.
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let list = tim
            .send("GET", &format!("{FIELDS}?project=ACME"), None)
            .await?;
        assert_eq!(ok(&list)?["fields"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}
