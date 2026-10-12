//! Own fields over HTTP on both databases (CF-01, CF-03, CF-04, CF-06, CF-07, CF-08): a wrong value
//! is refused with a sentence, a field is used only where it is meant, the level of a role counts
//! (also for one issue type), a hidden field is in no answer, and combining two choices moves every
//! value and leaves a record.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

pub(super) const FIELDS: &str = "/api/v1/fields";

/// Makes a field as `browser`; the answer, which has to be a success.
pub(super) async fn make_field(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let made = browser.send("POST", FIELDS, Some(body)).await?;
    Ok(ok(&made)?.clone())
}

/// The ID of a field or a choice in an answer.
pub(super) fn id_of(value: &Value) -> Result<String, Failure> {
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
pub(super) async fn severity(ada: &mut Browser) -> Result<Value, Failure> {
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

/// Sets the levels `(field, issue type, level)` of a role, as the administrator.
pub(super) async fn set_levels(
    site: &Site<'_>,
    rig: &Rig,
    role: RoleId,
    levels: &[(&str, Option<&str>, FieldLevel)],
) -> Result<(), Failure> {
    for (field, issue_type, level) in levels {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role,
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
    Ok(())
}

/// A person with a role of their own that has `permissions` and the given levels, for the whole
/// organization or, with an issue type, for that type only: `(field, issue type, level)`.
pub(super) async fn reader_of(
    site: &Site<'_>,
    rig: &Rig,
    email: &str,
    permissions: &[Permission],
    levels: &[(&str, Option<&str>, FieldLevel)],
) -> Result<Browser, Failure> {
    let role = site.org.role(email, permissions).await?;
    set_levels(site, rig, role.id, levels).await?;
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

/// What a role with no level for the fields reads of an issue that holds a hidden pick-one field
/// (`key`, with the value `major`) and a hidden long text (`notes`): none of it, in the issue, in
/// the list and in the definitions. The answer about the issue is returned.
async fn reads_nothing_of(
    ada: &mut Browser,
    writer: &mut Browser,
    path: &str,
    (key, major, notes): (&str, &str, &str),
) -> Result<Reply, Failure> {
    let detail = writer.send("GET", path, None).await?;
    assert!(!detail.body.to_string().contains(key), "{}", detail.body);
    assert!(!detail.body.to_string().contains(major), "{}", detail.body);
    // Neither the text nor the HTML of the long text, nor its list of rendered fields.
    for hidden in [notes, "Hidden", "<strong>words</strong>"] {
        assert!(!detail.body.to_string().contains(hidden), "{}", detail.body);
    }
    assert!(
        ok(&detail)?["own_fields_html"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{}",
        detail.body
    );
    let list = writer
        .send("GET", &format!("{}?fields=own_fields", acme_issues()), None)
        .await?;
    assert!(!list.body.to_string().contains(major), "{}", list.body);
    // The definition of the field is not there for them, and the list does not hold it.
    let one = writer.send("GET", &format!("{FIELDS}/{key}"), None).await?;
    assert_eq!(one.code(), "resource.not_found", "{}", one.body);
    let all = writer.send("GET", FIELDS, None).await?;
    assert!(!all.body.to_string().contains(key), "{}", all.body);
    let for_ada = ada.send("GET", &format!("{FIELDS}/{key}"), None).await?;
    assert_eq!(for_ada.status, StatusCode::OK, "{}", for_ada.body);
    Ok(detail)
}

#[tokio::test]
async fn cf_06_a_hidden_own_field_never_leaves_in_any_answer_or_history() -> TestResult {
    each_database("cf_06_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_of(&level)?, id_of(&level["choices"][1])?);
        // A long text is sent as HTML next to the values: hidden, it is in no answer either.
        let notes = id_of(
            &make_field(
                &mut ada,
                json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let made = ok(&ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({
                    "issue_type": site.bug,
                    "title": "Secret",
                    "fields": { &key: &major, &notes: "Hidden **words**" },
                })),
            )
            .await?)?
        .clone();
        assert!(made["own_fields"][&notes].is_string(), "{made}");
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
        let detail = reads_nothing_of(&mut ada, &mut writer, &path, (&key, &major, &notes)).await?;
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
        // The record of the change that the conflict reads from (the history of the issue) names
        // the hidden fields of the change in neither the key nor the text.
        for hidden in [&notes, "Hidden", "words"] {
            assert!(!stale.body.to_string().contains(hidden), "{}", stale.body);
        }
        // Ada reads the text and the HTML of it.
        let seen = ada.send("GET", &path, None).await?;
        assert_eq!(
            ok(&seen)?["issue"]["own_fields"][&notes],
            "Hidden **words**"
        );
        assert!(
            ok(&seen)?["own_fields_html"][0]["html"]
                .as_str()
                .is_some_and(|html| html.contains("<strong>words</strong>"))
        );
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

#[tokio::test]
async fn cf_06_a_lead_who_may_not_see_a_field_reads_neither_its_name_nor_its_rows() -> TestResult {
    each_database("cf_06_names", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let salary = make_field(
            &mut ada,
            json!({"name": "Salary band", "kind": "text", "applies_to": "issues"}),
        )
        .await?;
        let elsewhere = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project")?
            .to_owned();
        let only_beta = make_field(
            &mut ada,
            json!({
                "name": "Beta only",
                "kind": "text",
                "applies_to": "issues",
                "scope": {"projects": [elsewhere]},
            }),
        )
        .await?;
        let (salary_id, beta_id) = (id_of(&salary)?, id_of(&only_beta)?);
        // A lead who decides what roles see and checks access, with the salary band hidden.
        let mut lead = reader_of(
            &site,
            &rig,
            "lena@example.test",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::DecideWhatRolesSee,
                Permission::CheckAccess,
            ],
            &[
                (&salary_id, None, FieldLevel::Hidden),
                (&beta_id, None, FieldLevel::CanSee),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let me = lead.send("GET", "/api/v1/auth/me", None).await?;
        let lena = id_of(ok(&me)?)?;
        // The same answers for Ada, who manages fields, do name them.
        let table = ada.send("GET", "/api/v1/field-permissions", None).await?;
        assert!(ok(&table)?.to_string().contains("Salary band"));
        for (screen, path) in [
            ("field permissions", "/api/v1/field-permissions".to_owned()),
            (
                "field permissions of a project",
                format!("/api/v1/field-permissions?project={}", site.acme),
            ),
            (
                "check access",
                format!("/api/v1/check-access/people/{lena}?project={}", site.acme),
            ),
            ("the list of fields", FIELDS.to_owned()),
        ] {
            let seen = lead.send("GET", &path, None).await?;
            assert_eq!(seen.status, StatusCode::OK, "{screen}: {}", seen.body);
            let text = seen.body.to_string();
            assert!(!text.contains("Salary band"), "{screen}: {text}");
            assert!(!text.contains(&salary_id), "{screen}: {text}");
            // A field that belongs to another project is not told either, in a project.
            if screen != "field permissions" && screen != "the list of fields" {
                assert!(!text.contains("Beta only"), "{screen}: {text}");
            }
        }
        // At the organization the lead reads the field that they may see and that sits in a
        // project they see.
        let list = lead.send("GET", FIELDS, None).await?;
        assert_eq!(ok(&list)?["fields"].as_array().map(Vec::len), Some(1));
        // A field they may not see answers like one that is not there; a field they may see does not.
        let hidden = lead
            .send("GET", &format!("{FIELDS}/{salary_id}"), None)
            .await?;
        assert_eq!(hidden.code(), "resource.not_found", "{}", hidden.body);
        let hidden_used = lead
            .send("GET", &format!("{FIELDS}/{salary_id}/used-by"), None)
            .await?;
        assert!(matches!(
            hidden_used.code(),
            "resource.not_found" | "access.forbidden"
        ));
        let shown = lead
            .send("GET", &format!("{FIELDS}/{beta_id}"), None)
            .await?;
        assert_eq!(shown.status, StatusCode::OK, "{}", shown.body);
        move_preview_hides(&site, &rig, &mut ada, (&salary_id, &beta_id)).await?;
        Ok(())
    })
    .await
}

/// The move preview of a project with a sub-project: Pia's role (on the new parent) reads the
/// fields `salary` and `beta` and a field only for the sub-project; a lead who may not see the
/// salary band reads neither its name nor its row, and reads the field of the sub-project by name.
async fn move_preview_hides(
    site: &Site<'_>,
    rig: &Rig,
    ada: &mut Browser,
    (salary_id, beta_id): (&str, &str),
) -> TestResult {
    // The move preview: a project with a sub-project (and a field that is only for the
    // sub-project) moves below ACME, where Pia has a role that reads all three fields.
    let make = |short: &'static str| json!({"short_name": short, "name": short});
    let moved = id_of(&ada.send("POST", PROJECTS, Some(make("MOVED"))).await?.body)?;
    let sub = id_of(&ada.send("POST", PROJECTS, Some(make("MSUB"))).await?.body)?;
    let into_moved = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{sub}/move-preview"),
            Some(json!({"to": moved})),
        )
        .await?;
    let version = into_moved.body["preview"].clone();
    ok(&ada
        .send(
            "POST",
            &format!("{PROJECTS}/{sub}/move"),
            Some(json!({"to": moved, "preview": version})),
        )
        .await?)?;
    let sub_only = make_field(
        ada,
        json!({
            "name": "Sub only",
            "kind": "text",
            "applies_to": "issues",
            "scope": {"projects": [sub]},
        }),
    )
    .await?;
    let sub_id = id_of(&sub_only)?;
    let pia = site.org.person().await?;
    let reads = site
        .org
        .role(
            "Pia reads",
            &[Permission::SeeProjects, Permission::SeeIssues],
        )
        .await?;
    let all_three = [
        (salary_id, None, FieldLevel::CanSee),
        (beta_id, None, FieldLevel::CanSee),
        (sub_id.as_str(), None, FieldLevel::CanSee),
    ];
    set_levels(site, rig, reads.id, &all_three).await?;
    let acme_id: ProjectId = site.acme.parse().map_err(|error| format!("{error}"))?;
    site.org.give(&pia, reads.id, acme_id).await?;
    // The lead may also move projects, and may see "Sub only" but not "Salary band".
    let mut mona = reader_of(
        site,
        rig,
        "mona@example.test",
        &[
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::MoveProjects,
        ],
        &[
            (salary_id, None, FieldLevel::Hidden),
            (&sub_id, None, FieldLevel::CanSee),
        ],
    )
    .await?;
    let ask = json!({"to": site.acme});
    let path = format!("{PROJECTS}/{moved}/move-preview");
    let whole = ada.send("POST", &path, Some(ask.clone())).await?;
    let everything = whole.body.to_string();
    assert!(everything.contains(salary_id), "{everything}");
    assert!(everything.contains("Sub only"), "{everything}");
    let seen = mona.send("POST", &path, Some(ask)).await?;
    assert_eq!(seen.status, StatusCode::OK, "{}", seen.body);
    let text = seen.body.to_string();
    // Hidden: neither the name nor the ID of the field, nor a row with its levels.
    assert!(!text.contains("Salary band"), "move preview: {text}");
    assert!(!text.contains(salary_id), "move preview: {text}");
    // Allowed: the field of the sub-project, by its name, in the sub-project's row.
    assert!(text.contains("Sub only"), "move preview: {text}");
    Ok(())
}

#[tokio::test]
async fn cf_06_an_own_field_never_starts_with_a_default_that_cannot_be_kept() -> TestResult {
    each_database("cf_06_default", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let refuse = |reply: &Reply| {
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::BAD_REQUEST, "request.invalid"),
                "{}",
                reply.body
            );
        };
        // A kind that points to people, or a long text, takes no starting value.
        for kind in ["person", "long-text"] {
            let made = ada
                .send(
                    "POST",
                    FIELDS,
                    Some(json!({
                        "name": format!("Starts {kind}"),
                        "kind": kind,
                        "applies_to": "issues",
                        "default_value": "x",
                    })),
                )
                .await?;
            refuse(&made);
        }
        // A long text cannot be made for projects at all.
        let long = ada
            .send(
                "POST",
                FIELDS,
                Some(json!({"name": "Notes", "kind": "long-text", "applies_to": "projects"})),
            )
            .await?;
        refuse(&long);
        // A starting value is used, and dropped when the rules no longer allow it.
        let field = make_field(
            &mut ada,
            json!({"name": "Region", "kind": "text", "applies_to": "issues", "default_value": "North"}),
        )
        .await?;
        let key = id_of(&field)?;
        let made = ok(&site.make(&mut ada, "Uses the default").await?)?.clone();
        assert_eq!(made["own_fields"][&key], "North");
        let tightened = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{key}"),
                Some(json!({"version": field["version"], "rules": {"max_length": 3}})),
            )
            .await?;
        assert!(ok(&tightened)?["default_value"].is_null(), "{}", tightened.body);
        let later = ok(&site.make(&mut ada, "Starts empty").await?)?.clone();
        assert!(later["own_fields"].get(&key).is_none());
        Ok(())
    })
    .await
}

/// Fields that point to a person, people, a group, an issue, and a long text: their IDs.
async fn reference_fields(ada: &mut Browser) -> Result<Vec<String>, Failure> {
    let mut keys = Vec::new();
    for (name, kind) in [
        ("Tester", "person"),
        ("Reviewers", "people"),
        ("Team", "group"),
        ("Duplicate of", "issue"),
        ("Notes", "long-text"),
    ] {
        let made = make_field(
            ada,
            json!({"name": name, "kind": kind, "applies_to": "issues"}),
        )
        .await?;
        keys.push(id_of(&made)?);
    }
    Ok(keys)
}

/// The ways a person in a field is refused: the ID of another kind of thing, a person who is not
/// there, and a person who cannot see the issue.
async fn person_refusals(
    ada: &mut Browser,
    rig: &Rig,
    site: &Site<'_>,
    path: &str,
    tester: &str,
    version: &Value,
    (other, me): (&Value, &str),
) -> TestResult {
    // The ID of another kind of thing is no person, and a person who is not there is no choice.
    let wrong_kind = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: other }})),
        )
        .await?;
    assert_eq!(
        wrong_kind.code(),
        "field.wrong_value",
        "{}",
        wrong_kind.body
    );
    let gone = format!(
        "{}{}",
        &me[..me.len() - 1],
        if me.ends_with('a') { 'b' } else { 'a' }
    );
    let nobody = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: gone }})),
        )
        .await?;
    assert_eq!(nobody.code(), "field.unknown_reference", "{}", nobody.body);
    // A person who has no role anywhere cannot see the issue, so they cannot be named in it.
    let outsider = rig
        .person(&site.org.admin.session, "otto@example.test")
        .await?
        .person
        .id
        .to_string();
    let unseen = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: outsider }})),
        )
        .await?;
    assert_eq!(unseen.code(), "field.unknown_reference", "{}", unseen.body);
    Ok(())
}

#[tokio::test]
async fn cf_01_people_groups_and_issues_are_shown_for_the_viewer_and_a_long_text_waits_for_its_lock()
-> TestResult {
    each_database("cf_01_references", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let keys = reference_fields(&mut ada).await?;
        let [tester, reviewers, team, duplicate, notes] = keys.as_slice() else {
            return Err("five fields".into());
        };
        let me = id_of(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let group = id_of(ok(&ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Core"})))
            .await?)?)?;
        let first = ok(&site.make(&mut ada, "The first").await?)?.clone();
        let (guest, _) = site.person(StartingRole::Guest, "gus").await?;
        let mut guest = guest;
        let second = ok(&site.make(&mut guest, "The second").await?)?.clone();
        let path = format!("{ISSUES}/{}", second["key"].as_str().ok_or("no key")?);
        let values = json!({
            tester: me, reviewers: [me], team: group, duplicate: first["id"],
        });
        let set = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": second["version"], "fields": values})),
            )
            .await?;
        let own = &ok(&set)?["own_fields"];
        // A person is a PersonRef, a group is its ID (and its name for those who see names), an
        // issue is its readable ID.
        assert_eq!(own[tester]["shown"], "name", "{own}");
        assert_eq!(own[reviewers][0]["shown"], "name", "{own}");
        assert_eq!(own[team]["id"], json!(group), "{own}");
        assert_eq!(own[duplicate]["key"], first["key"], "{own}");
        // The Guest sees only the issue they reported: the issue named in the field is not theirs
        // to see, so the field shows nothing.
        let seen = guest.send("GET", &path, None).await?;
        let theirs = &ok(&seen)?["issue"]["own_fields"];
        assert!(theirs.get(duplicate).is_none(), "{theirs}");
        let version = ok(&set)?["version"].clone();
        person_refusals(
            &mut ada,
            &rig,
            &site,
            &path,
            tester,
            &version,
            (&first["id"], &me),
        )
        .await?;
        // A long text is saved only under its lock: the change of the issue refuses it.
        let long = ada
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": ok(&set)?["version"],
                    "fields": { notes: "Steps" },
                })),
            )
            .await?;
        assert_eq!(long.code(), "request.invalid", "{}", long.body);
        let on_project = ada
            .send(
                "PATCH",
                &format!("{PROJECTS}/{}", site.acme),
                Some(json!({"version": 1, "fields": { notes: "Steps" }})),
            )
            .await?;
        assert_eq!(
            on_project.status,
            StatusCode::BAD_REQUEST,
            "{}",
            on_project.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_01_a_long_text_own_field_is_saved_under_its_lock_and_comes_back_as_html() -> TestResult
{
    each_database("cf_01_long_text", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let notes = make_field(
            &mut ada,
            json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
        )
        .await?;
        let field = id_of(&notes)?;
        let made = ok(&site.make(&mut ada, "Has notes").await?)?.clone();
        let (id, key) = (id_of(&made)?, made["key"].as_str().ok_or("no key")?.to_owned());
        let taken = ada
            .send(
                "POST",
                "/api/v1/edit-locks",
                Some(json!({
                    "what": {"record": {"kind": "issue", "id": id}, "part": format!("field-{field}")},
                    "editor_id": "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5",
                })),
            )
            .await?;
        assert_eq!(taken.body["status"], "taken", "{}", taken.body);
        let saved = ada
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/fields/{field}/text"),
                Some(json!({
                    "version": made["version"],
                    "lock_token": taken.body["lock_token"],
                    "text": "Steps: **bold**",
                })),
            )
            .await?;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        // The page gets the text and the HTML the server made of it, for a viewer who may see it.
        let seen = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        let detail = ok(&seen)?;
        assert_eq!(detail["issue"]["own_fields"][&field], "Steps: **bold**");
        assert!(
            detail["own_fields_html"][0]["html"]
                .as_str()
                .is_some_and(|html| html.contains("<strong>bold</strong>")),
            "{detail}"
        );
        assert_eq!(detail["own_fields_html"][0]["field"], json!(field));
        // A role that may not see the field gets neither.
        let mut reader = reader_of(
            &site,
            &rig,
            "rae@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[(&field, None, FieldLevel::Hidden), ("title", None, FieldLevel::CanSee)],
        )
        .await?;
        let theirs = reader.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        assert!(!ok(&theirs)?.to_string().contains("bold"), "{}", theirs.body);
        assert!(ok(&theirs)?["own_fields_html"].as_array().is_some_and(Vec::is_empty));
        Ok(())
    })
    .await
}
