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
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, ok};
use super::policy_rig::Member;
use super::rig::Rig;

mod choices;
mod levels;
mod long_text;
mod references;
mod rules;

pub(super) const FIELDS: &str = "/api/v1/fields";

/// Creates a field as `browser`; the answer, which has to be a success.
pub(super) async fn create_field(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let made = browser.send("POST", FIELDS, Some(body)).await?;
    Ok(ok(&made)?.clone())
}

/// The path of the issues of ACME.
fn acme_issues() -> String {
    format!("{PROJECTS}/ACME/issues")
}

/// Creates an issue of the type `bug` in ACME with these values of own fields; the answer.
async fn create_with(ada: &mut Browser, bug: &str, fields: Value) -> Result<Reply, Failure> {
    let body = json!({"issue_type": bug, "title": "An issue", "fields": fields});
    ada.send("POST", &acme_issues(), Some(body)).await
}

/// A field *Severity* with three choices, for issues.
pub(super) async fn severity(ada: &mut Browser) -> Result<Value, Failure> {
    create_field(
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
    let in_bug = ok(&site.create(ada, "A bug").await?)?["readable_id"]
        .as_str()
        .ok_or("no key")?
        .to_owned();
    let in_task = ok(&ada
        .send(
            "POST",
            &acme_issues(),
            Some(json!({"issue_type": task, "title": "A question"})),
        )
        .await?)?["readable_id"]
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
